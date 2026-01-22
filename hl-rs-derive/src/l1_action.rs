use proc_macro::TokenStream;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, LitStr};

use crate::{ensure_struct_fields, parse_action_attrs};

fn has_nonce_field(fields: &syn::FieldsNamed) -> bool {
    fields
        .named
        .iter()
        .any(|field| field.ident.as_ref().is_some_and(|ident| ident == "nonce"))
}

fn build_l1_action_impl(ident: &syn::Ident, action_type_lit: &syn::LitStr) -> TokenStream2 {
    quote! {
        impl crate::exchange::action_v2::L1Action for #ident {
            const ACTION_TYPE: &'static str = #action_type_lit;
        }

        impl crate::exchange::action_v2::Action for #ident {
            fn action_type(&self) -> &'static str {
                <Self as crate::exchange::action_v2::L1Action>::ACTION_TYPE
            }

            fn signing_hash(
                &self,
                meta: &crate::exchange::action_v2::SigningMeta,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                let vault_for_hash =
                    if <Self as crate::exchange::action_v2::L1Action>::EXCLUDE_VAULT_FROM_HASH {
                        None
                    } else {
                        meta.vault_address
                    };

                let connection_id = crate::exchange::action_v2::compute_l1_hash(
                    self,
                    meta.nonce,
                    vault_for_hash,
                    meta.expires_after,
                )?;

                Ok(crate::exchange::action_v2::agent_signing_hash(
                    connection_id,
                    &meta.signing_chain.get_source(),
                ))
            }

            fn multisig_signing_hash(
                &self,
                meta: &crate::exchange::action_v2::SigningMeta,
                payload_multi_sig_user: alloy::primitives::Address,
                outer_signer: alloy::primitives::Address,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                let envelope = (
                    payload_multi_sig_user.to_string().to_lowercase(),
                    outer_signer.to_string().to_lowercase(),
                    self,
                );

                let vault_for_hash =
                    if <Self as crate::exchange::action_v2::L1Action>::EXCLUDE_VAULT_FROM_HASH {
                        None
                    } else {
                        meta.vault_address
                    };

                let connection_id = crate::exchange::action_v2::compute_l1_hash(
                    &envelope,
                    meta.nonce,
                    vault_for_hash,
                    meta.expires_after,
                )?;

                Ok(crate::exchange::action_v2::agent_signing_hash(
                    connection_id,
                    &meta.signing_chain.get_source(),
                ))
            }

            fn nonce(&self) -> Option<u64> {
                self.nonce
            }

            fn extract_action_kind(&self) -> crate::exchange::action_v2::ActionKind {
                crate::exchange::action_v2::ActionKind::#ident(self.clone())
            }

            fn with_nonce(mut self, nonce: u64) -> Self {
                self.nonce = Some(nonce);
                self
            }
        }
    }
}

pub(crate) fn derive_l1_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let (action_type_override, _) = match parse_action_attrs(&input.attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error().into(),
    };

    let data_fields = match ensure_struct_fields(&input) {
        Ok(fields) => fields,
        Err(err) => return err.to_compile_error().into(),
    };

    let has_nonce = match data_fields {
        Fields::Named(fields) => has_nonce_field(fields),
        _ => false,
    };

    if !has_nonce {
        return syn::Error::new(
            input.ident.span(),
            "L1Action derive requires a `nonce: Option<u64>` field",
        )
        .to_compile_error()
        .into();
    }

    let ident = &input.ident;
    let action_type_value =
        action_type_override.unwrap_or_else(|| ident.to_string().to_lower_camel_case());
    let action_type_lit = LitStr::new(&action_type_value, ident.span());

    build_l1_action_impl(ident, &action_type_lit).into()
}
