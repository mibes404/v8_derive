#![warn(clippy::pedantic)]

mod helpers;

extern crate proc_macro2;

use helpers::{get_ident, quote_get_field_as};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;

/// Derive `TryFromValue` for a struct
///
/// # Panics
/// When the input is not a struct
#[proc_macro_derive(FromValue)]
pub fn try_from_value(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);

    let struct_identifier = &input.ident;

    #[allow(clippy::single_match_else)]
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut implementation = TokenStream::new();
            implementation.extend(quote! {});

            for field in fields {
                let Some(identifier) = field.ident.as_ref() else {
                    continue;
                };

                let field_impl = match &field.ty {
                    syn::Type::Path(type_path) => {
                        let ident = get_ident(type_path);

                        match quote_get_field_as(ident, identifier, field, false) {
                            Some(value) => {
                                quote! {
                                    #identifier: #value,
                                }
                            }
                            None => continue,
                        }
                    }
                    _ => unimplemented!(),
                };

                implementation.extend(field_impl);
            }

            quote! {
                #[automatically_derived]
                impl v8_derive::TryFromValue for #struct_identifier {
                    fn try_from_value(
                        input: &v8::Local<'_, v8::Value>,
                        scope: &mut v8::PinScope<'_, '_>,
                    ) -> v8_derive::errors::Result<Self>
                    where
                        Self: Sized {
                            Ok(Self {
                                #implementation
                            })
                    }
                }
            }
        }
        _ => {
            panic!("Only structs are supported");
        }
    }
    .into()
}

/// Derive `IntoValue` for a struct
///
/// # Panics
/// When the input is not a struct
#[proc_macro_derive(IntoValue)]
pub fn into_value(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);

    let struct_identifier = &input.ident;

    #[allow(clippy::single_match_else)]
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut implementation = TokenStream::new();
            implementation.extend(quote! {});

            for field in fields {
                let Some(identifier) = field.ident.as_ref() else {
                    continue;
                };

                let field_impl = match &field.ty {
                    syn::Type::Path(_type_path) => {
                        quote! {
                            let js_key = v8::String::new(scope, stringify!(#identifier)).unwrap().into();
                            let js_val = self.#identifier.into_value(scope);
                            object.set(scope, js_key, js_val);
                        }
                    }
                    _ => unimplemented!(),
                };

                implementation.extend(field_impl);
            }

            quote! {
                #[automatically_derived]
                impl v8_derive::IntoValue for #struct_identifier {
                    fn into_value<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Value> {
                        let object = v8::Object::new(scope);
                        #implementation
                        object.into()
                    }
                }
            }
        }
        _ => {
            panic!("Only structs are supported");
        }
    }
    .into()
}
