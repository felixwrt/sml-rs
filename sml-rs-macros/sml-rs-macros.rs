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

#[proc_macro_derive(CompactDebug)]
pub fn compact_debug_macro(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    match derive_input.data {
        syn::Data::Struct(strukt) => struct_derive_macro_2(strukt, derive_input.ident, derive_input.generics),
        syn::Data::Enum(enum_) => enum_derive_macro(enum_, derive_input.ident, derive_input.generics),
        _ => quote!(compile_error!(
            "SmlParse can only be applied to structs and enums."
        ))
        .into(),
    }
}

fn struct_derive_macro_2(strukt: syn::DataStruct, ident: syn::Ident, generics: syn::Generics) -> TokenStream {
    use quote::ToTokens;

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

    let mut fields = vec![];

    for field in nf.named {
        let field_ty = field.ty;

        let field_ty_str = field_ty.to_token_stream().to_string();

        let name = field.ident.unwrap();

        let field_expr = if field_ty_str.starts_with("Option") {
            quote!( &e )
        } else {
            quote!( &self.#name )
        };

        let field_expr = if field_ty_str.contains("OctetStr") {
            quote!(
                &OctetStrFormatter(#field_expr)
            )
        } else {
            quote!(
                #field_expr
            )
        };

        fields.push(if field_ty_str.starts_with("Option") {
            quote! {
                if let Some(e) = &self.#name {
                    x.field(stringify!(#name), #field_expr);
                }
            }
        } else {
            quote! {
                x.field(stringify!(#name), #field_expr);
            }
        });
    }

    let toks = quote!(
        impl<'i> core::fmt::Debug for #strukt_name #strukt_generics {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let mut x = f.debug_struct(stringify!(#strukt_name));
                #(#fields)*
                // x.field("obj_name", &OctetStrFormatter(&self.obj_name));
                // if let Some(e) = &self.status {
                //     x.field("status", &e);
                // }
                // if let Some(e) = &self.val_time {
                //     x.field("val_time", e);
                // }
                // if let Some(e) = &self.unit {
                //     x.field("unit", e);
                // }
                // if let Some(e) = &self.scaler {
                //     x.field("scaler", e);
                // }
                // x.field("value", &self.value);
                // if let Some(e) = &self.value_signature {
                //     x.field("value_signature", &OctetStrFormatter(e));
                // }
                x.finish()
            }
        }
    );
    toks.into()
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