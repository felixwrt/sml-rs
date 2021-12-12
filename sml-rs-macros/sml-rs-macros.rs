extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(SmlParse, attributes(tag))]
pub fn sml_parse_macro(input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::Item);
    match item {
        syn::Item::Struct(strukt) => struct_derive_macro(strukt),
        syn::Item::Enum(enum_) => enum_derive_macro(enum_),
        _ => quote!(compile_error!(
            "SmlParse can only be applied to structs and enums."
        ))
        .into(),
    }
}

fn struct_derive_macro(strukt: syn::ItemStruct) -> TokenStream {
    let strukt_name = strukt.ident;
    let strukt_generics = strukt.generics;

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
        // todo: rewrite such that the lifetime parameter 'i isn't hardcoded here anymore
        impl<'i> crate::SmlParse<'i> for #strukt_name #strukt_generics {
            fn parse(input: &'i [u8]) -> nom::IResult<&[u8], Self> {
                let (input, tlf) = crate::tlf::TypeLengthField::parse(input)?;
                if tlf.ty != crate::tlf::Ty::ListOf || tlf.len != #num_fields {
                    return Err(error(input));
                }

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

fn enum_derive_macro(enum_: syn::ItemEnum) -> TokenStream {
    use quote::ToTokens;
    // TODO: improve error handling

    let name = enum_.ident;
    let generics = enum_.generics;

    let mut is_u32 = false;

    let mut variant_lines = vec![];

    for variant in enum_.variants {
        let attr = variant
            .attrs
            .into_iter()
            .filter(|a| a.path.to_token_stream().to_string() == "tag")
            .next()
            .unwrap()
            .tokens;
        let tag_paren = syn::parse2::<syn::ExprParen>(attr).unwrap();
        let tag = match tag_paren.expr.as_ref() {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Int(tag) => tag,
                _ => {
                    panic!()
                }
            },
            _ => {
                panic!();
            }
        };

        // if one of the tags is out of range for u8, it should be u32 instead
        if tag.base10_parse::<u8>().is_err() {
            is_u32 = true;
        }
        //let tag_value = tag.base10_parse::<u32>().unwrap();
        let var_name = variant.ident;
        let ty = match variant.fields {
            syn::Fields::Unnamed(fu) => fu.unnamed.iter().next().unwrap().ty.clone(),
            _ => panic!(),
        };

        variant_lines.push(quote!(
            #tag => nom::combinator::map(<#ty>::parse, #name::#var_name)(input),
        ));
    }

    let tag_ty = if is_u32 { quote!(u32) } else { quote!(u8) };

    let toks = quote!(
        impl<'i> SmlParse<'i> for #name #generics {
            fn parse(input: &'i [u8]) -> nom::IResult<&[u8], Self> {
                // ListOf(Tag(u8), Elmt)

                // parse ListOf-tlf
                let (input, tlf) = crate::tlf::TypeLengthField::parse(input)?;
                if tlf.ty != crate::tlf::Ty::ListOf || tlf.len != 2 {
                    return Err(error(input));
                }

                // parse tag
                let (input, tag) = #tag_ty::parse(input)?;

                // parse element
                match tag {
                    #(#variant_lines)*
                    _ => { return Err(error(input)) }
                }
            }
        }
    );
    // println!("{}", toks.to_string());
    toks.into()
}
