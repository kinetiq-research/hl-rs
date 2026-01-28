extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{spanned::Spanned, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta, Token};

use syn::punctuated::Punctuated;

mod l1_action;
mod user_signed_action;

pub(crate) fn parse_action_attrs(
    attrs: &[Attribute],
) -> Result<(Option<String>, Option<String>, Option<String>), syn::Error> {
    let mut action_type_override = None;
    let mut payload_key_override = None;
    let mut types_preimage = None;

    for attr in attrs {
        if attr.path().is_ident("action") {
            let args = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in args {
                let name_value = match meta {
                    Meta::NameValue(name_value) => name_value,
                    _ => continue,
                };

                if let Some(ident) = name_value.path.get_ident() {
                    match ident.to_string().as_ref() {
                        "action_type" => {
                            action_type_override = Some(extract_lit_str(&name_value.value)?);
                        }
                        "payload_key" => {
                            payload_key_override = Some(extract_lit_str(&name_value.value)?);
                        }
                        "types" => {
                            types_preimage = Some(extract_lit_str(&name_value.value)?);
                        }
                        _ => {
                            return Err(syn::Error::new(
                                name_value.span(),
                                "invalid action attribute",
                            ))
                        }
                    }
                }
            }
        }
    }
    Ok((action_type_override, payload_key_override, types_preimage))
}

fn extract_lit_str(expr_lit: &Expr) -> Result<String, syn::Error> {
    let Expr::Lit(expr_lit) = expr_lit else {
        return Err(syn::Error::new(expr_lit.span(), "must be a string literal"));
    };
    let Lit::Str(lit_str) = &expr_lit.lit else {
        return Err(syn::Error::new(expr_lit.span(), "must be a string literal"));
    };

    Ok(lit_str.value())
}

pub(crate) fn ensure_struct_fields(input: &DeriveInput) -> Result<&Fields, syn::Error> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "derive can only be used for structs",
        ));
    };
    Ok(&data.fields)
}

pub(crate) fn ensure_named_fields(input: &DeriveInput) -> Result<&syn::FieldsNamed, syn::Error> {
    match ensure_struct_fields(input)? {
        Fields::Named(fields) => Ok(fields),
        _ => Err(syn::Error::new(
            input.ident.span(),
            "derive requires named fields",
        )),
    }
}

#[proc_macro_derive(L1Action, attributes(action))]
pub fn derive_l1_action(input: TokenStream) -> TokenStream {
    l1_action::derive_l1_action(input)
}

#[proc_macro_derive(UserSignedAction, attributes(action))]
pub fn derive_user_signed_action(input: TokenStream) -> TokenStream {
    user_signed_action::derive_user_signed_action(input)
}
