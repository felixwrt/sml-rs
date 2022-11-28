extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(SmlParse, attributes(tag))]
pub fn sml_parse_macro(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    match derive_input.data {
        syn::Data::Struct(strukt) => struct_derive_macro(strukt, derive_input.ident, derive_input.generics),
        syn::Data::Enum(enum_) => enum_derive_macro(enum_, derive_input.ident, derive_input.generics),
        _ => quote!(compile_error!(
            "SmlParse can only be applied to structs and enums."
        ))
        .into(),
    }
}

fn struct_derive_macro(strukt: syn::DataStruct, ident: syn::Ident, generics: syn::Generics) -> TokenStream {
    let strukt_name = ident;
    let strukt_generics = generics;

    let nf = match strukt.fields {
        syn::Fields::Named(nf) => nf,
        _ => {
            return quote!(compile_error!(
                "SmlParse cannot be applied to tuple structs."
            ))
            .into();
        }
    };

    let num_fields = nf.named.len();

    let mut fields = vec![];
    let mut field_names = vec![];

    for field in nf.named {
        let field_ty = field.ty;
        let name = field.ident.unwrap();

        fields.push(quote! {
            let (input, #name) = <#field_ty>::parse(input)?;
        });

        field_names.push(name);
    }

    let toks = quote!(
        impl<'i> crate::parser::SmlParseTlf<'i> for #strukt_name #strukt_generics {
            fn check_tlf(tlf: &TypeLengthField) -> bool {
                *tlf == crate::parser::tlf::TypeLengthField::new(crate::parser::tlf::Ty::ListOf, #num_fields as u32)
            }
        
            fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
                #(#fields)*

                let val = #strukt_name {
                    #(#field_names),*
                };
                Ok((input, val))
            }
        }
    );
    toks.into()
}

fn enum_derive_macro(enum_: syn::DataEnum, ident: syn::Ident, generics: syn::Generics) -> TokenStream {
    use quote::ToTokens;
    // TODO: improve error handling

    let name = ident;

    let mut is_u32 = false;

    let mut variant_lines = vec![];

    for variant in &enum_.variants {
        let Some(attr) = variant
            .attrs
            .iter()
            .find(|a| a.path.to_token_stream().to_string() == "tag")
            .map(|x| &x.tokens)
        else {
            return enum_derive_macro_implicit(&enum_, name, generics);
        };
        let tag_value = u32::from_str_radix(attr.to_string().trim_start_matches("(0x").trim_end_matches(')'), 16).expect("Couldn't parse tag");
        // if one of the tags is out of range for u8, it should be u32 instead
        let tag = proc_macro2::Literal::u32_unsuffixed(tag_value);
        if tag_value > 255 {
            is_u32 = true;
        }
        let var_name = &variant.ident;
        let ty = match &variant.fields {
            syn::Fields::Unnamed(fu) => fu.unnamed.iter().next().unwrap().ty.clone(),
            _ => panic!(),
        };

        variant_lines.push(quote!(
            #tag => {
                let (input, x) = <#ty>::parse(input)?;
                Ok((input, #name::#var_name(x)))
            },
        ));
    }

    let holley_workaround_check = if name == "Time" {
        quote!(
            *tlf == crate::parser::tlf::TypeLengthField::new(crate::parser::tlf::Ty::Unsigned, 4)
        )
    } else { quote!( false ) };

    let holley_workaround = if name == "Time" {
        quote!(
            // Workaround for Holley DTZ541:
            // For the `Time` type, this meter doesn't respect the spec.
            // Intead of a TLF of type ListOf and length 2, it directly sends an u32 integer,
            // which is encoded by a TLF of Unsigned and length 4 followed by four bytes containing 
            // the data. 
            if #holley_workaround_check {
                let (input, bytes) = crate::parser::take::<4>(input)?;
                return Ok((input, Time::SecIndex(u32::from_be_bytes(bytes.clone()))));
            }
        )
    } else { quote!() };

    let tag_ty = if is_u32 { quote!(u32) } else { quote!(u8) };

    let toks = quote!(
        impl<'i> crate::parser::SmlParseTlf<'i> for #name #generics {

            fn check_tlf(tlf: &TypeLengthField) -> bool {
                (tlf.ty == crate::parser::tlf::Ty::ListOf && tlf.len == 2) || #holley_workaround_check
            }
        
            fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
                #holley_workaround

                // parse tag
                let (input, tag) = #tag_ty::parse(input)?;

                // parse element
                match tag {
                    #(#variant_lines)*
                    _ => { return Err(crate::parser::ParseError::UnexpectedVariant); }
                }
            }
        }
    );
    // println!("{}", toks.to_string());
    toks.into()
}


fn enum_derive_macro_implicit(enum_: &syn::DataEnum, ident: syn::Ident, generics: syn::Generics) -> TokenStream {
    let mut variant_lines = vec![];

    for variant in &enum_.variants {
        let var_name = &variant.ident;
        let ty = match &variant.fields {
            syn::Fields::Unnamed(fu) => fu.unnamed.iter().next().unwrap().ty.clone(),
            _ => panic!(),
        };

        variant_lines.push(quote!(
            tlf if <#ty>::check_tlf(tlf) => map(<#ty>::parse_with_tlf(input, tlf), Self::#var_name),
        ));
    }

    let toks = quote!(
        impl<'i> crate::parser::SmlParseTlf<'i> for #ident #generics {

            fn check_tlf(tlf: &TypeLengthField) -> bool {
                true
            }
        
            fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
                match tlf {
                    #(#variant_lines)*
                    _ => Err(ParseError::TlfMismatch(core::any::type_name::<Self>()))
                }
            }
        }
    );
    // println!("{}", toks.to_string());
    toks.into()
}