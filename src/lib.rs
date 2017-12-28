// #[recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, VariantData, MetaItem, NestedMetaItem, Lit, Ty};
use quote::Tokens;

#[proc_macro_derive(Error, attributes(error))]
pub fn enum_from(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let s = input.to_string();

    // Parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    let gen = build_enum_from(&ast);

    // Return the generated impl
    gen.parse().unwrap()
}

fn build_enum_from(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    match ast.body {
        Body::Enum(ref variants) => {
            let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
            let mut from_tokens = Tokens::new();
            let mut display_tokens = Tokens::new();
            let mut description_tokens = Tokens::new();
            let mut cause_tokens = Tokens::new();
            let mut cause_patterns = 0;

            'variants: for variant in variants {
                let field = match variant.data {
                    VariantData::Tuple(ref fields) if fields.len() == 1 => Some(fields.first().unwrap()),
                    VariantData::Unit => None,
                    // _ => continue,
                    _ => panic!("Only zero- or one-field enum variants are supported, {:?}", variant.data),
                };

                let variant_name = &variant.ident;
                let mut desc = None;
                let mut skip_from = false;

                for attr in variant.attrs.iter() {
                    match attr.value {
                        MetaItem::List(ref k, ref items) if k == "error" => {
                            for item in items {
                                match item {
                                    &NestedMetaItem::MetaItem(MetaItem::Word(ref word)) => {
                                        match word.as_ref() {
                                            "skip" => continue 'variants,
                                            "skip_from" => skip_from = true,
                                            _ => {}
                                        }
                                    },
                                    &NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, Lit::Str(ref value, _))) => {
                                        match name.as_ref() {
                                            "desc" => desc = Some(value),
                                            _ => {}
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        },
                        _ => {}
                    }
                }

                if !skip_from {
                    if let Some(ty) = field.map(|f| &f.ty) {
                        from_tokens.append(quote! {
                            impl #impl_generics From<#ty> for #name #ty_generics #where_clause {
                                fn from(err: #ty) -> #name #ty_generics {
                                    #name::#variant_name(err)
                                }
                            }
                        });
                    }
                }

                let is_string_type = field.map(|f| is_string_type(&f.ty)).unwrap_or(false);

                if field.is_some() {
                    if let Some(desc) = desc {
                        display_tokens.append(quote! {
                            #name::#variant_name(ref err) => write!(f, "{}: {}", #desc, err),
                        });

                        description_tokens.append(quote! {
                            #name::#variant_name(_) => #desc,
                        });
                    } else {
                        display_tokens.append(quote! {
                            #name::#variant_name(ref err) => write!(f, "{}", err),
                        });

                        if is_string_type {
                            description_tokens.append(quote! {
                                #name::#variant_name(ref s) => s,
                            });
                        } else {
                            description_tokens.append(quote! {
                                #name::#variant_name(ref err) => err.description(),
                            });
                        }
                    }

                    if !is_string_type {
                        cause_tokens.append(quote! {
                            #name::#variant_name(ref err) => Some(err),
                        });
                        cause_patterns += 1;
                    }
                } else {
                    // is unit field
                    if let Some(desc) = desc {
                        display_tokens.append(quote! {
                            #name::#variant_name => write!(f, "{}", #desc),
                        });

                        description_tokens.append(quote! {
                            #name::#variant_name => #desc,
                        });
                    } else {
                        display_tokens.append(quote! {
                            #name::#variant_name => write!(f, "{}", "#variant_name"),
                        });

                        description_tokens.append(quote! {
                            #name::#variant_name => "#variant_name",
                        });
                    }
                }
            }

            let cause_catchall = if cause_patterns < variants.len() {
                quote! {
                    _ => None,
                }
            } else {
                quote! {}
            };

            quote! {
                #from_tokens

                impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        match *self {
                            #display_tokens
                        }
                    }
                }

                impl #impl_generics ::std::error::Error for #name #ty_generics #where_clause {
                    fn description(&self) -> &str {
                        match *self {
                            #description_tokens
                        }
                    }

                    fn cause(&self) -> Option<&::std::error::Error> {
                        match *self {
                            #cause_tokens
                            #cause_catchall
                        }
                    }
                }
            }
        },
        _ => panic!(format!("#[derive(ErrorStatus)] can only be applied to enums. \
            {} is not an enum.", name))
    }
}

fn is_string_type(ty: &Ty) -> bool {
    use syn::{Path, PathSegment};

    match ty {
        &Ty::Path(None, Path { ref segments, ..}) if segments.len() == 1 => {
            match segments.first().unwrap() {
                &PathSegment {
                            ref ident, ..
                        } if ident == "String" => {
                    true
                },
                _ => false
            }
        },
        _ => false
    }
}