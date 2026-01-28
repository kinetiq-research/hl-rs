use std::collections::HashMap;

use proc_macro::TokenStream;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

use crate::{ensure_named_fields, parse_action_attrs};

enum FieldKind {
    Address,
    String,
    Decimal,
    Numeric,
}

struct FieldInfo {
    ident: syn::Ident,
    kind: FieldKind,
    is_option: bool,
}

fn build_field_map(
    fields: &syn::FieldsNamed,
) -> Result<(HashMap<String, FieldInfo>, bool), syn::Error> {
    let mut field_map = HashMap::new();
    let mut has_nonce = false;

    for field in fields.named.iter() {
        let Some(name) = field.ident.as_ref() else {
            continue;
        };
        let name_str = name.to_string();

        if name_str == "hyperliquid_chain" || name_str == "hyperliquidChain" {
            continue;
        }

        let ty_str = quote! { #field.ty }.to_string();
        let is_option = ty_str.contains("Option");
        let kind = if ty_str.contains("Address") {
            FieldKind::Address
        } else if ty_str.contains("String") || ty_str.contains("str") {
            FieldKind::String
        } else if ty_str.contains("Decimal") {
            FieldKind::Decimal
        } else {
            FieldKind::Numeric
        };

        if name_str == "nonce" {
            has_nonce = true;
        }

        field_map.insert(
            name_str,
            FieldInfo {
                ident: name.clone(),
                kind,
                is_option,
            },
        );
    }

    Ok((field_map, has_nonce))
}

fn build_struct_hash_tokens(
    fields: &syn::FieldsNamed,
    full_types_preimage: &str,
) -> Result<(Vec<TokenStream2>, bool), syn::Error> {
    let (field_map, has_nonce) = build_field_map(fields)?;

    let params = parse_types_params(full_types_preimage)?;
    let mut tokens = Vec::with_capacity(params.len() + 1);
    tokens.push(quote! {
        alloy::dyn_abi::DynSolValue::FixedBytes(type_hash, 32)
    });

    for (ty, name) in params {
        let ty_lower = ty.to_lowercase();
        if name == "hyperliquidChain" {
            if ty_lower != "string" {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "hyperliquidChain must be string",
                ));
            }
            tokens.push(quote! {
                alloy::dyn_abi::DynSolValue::FixedBytes(
                    alloy::primitives::keccak256(chain.get_hyperliquid_chain()),
                    32,
                )
            });
            continue;
        }

        if name == "nonce" || name == "time" {
            let field = field_map.get("nonce").ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "nonce field missing")
            })?;
            let size = parse_uint_size(&ty, field.ident.span())?;
            let ident = &field.ident;
            let expr = if field.is_option {
                quote! {
                    {
                        let nonce = self.#ident.expect("nonce must be set before signing");
                        alloy::dyn_abi::DynSolValue::Uint(
                            alloy::primitives::U256::from(nonce),
                            #size,
                        )
                    }
                }
            } else {
                quote! {
                    alloy::dyn_abi::DynSolValue::Uint(
                        alloy::primitives::U256::from(self.#ident),
                        #size,
                    )
                }
            };
            tokens.push(expr);
            continue;
        }

        let field = field_map.get(&name).ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), format!("field not found: {name}"))
        })?;

        let ident = &field.ident;
        let token = match field.kind {
            FieldKind::Address => {
                if ty_lower != "address" && ty_lower != "string" {
                    return Err(syn::Error::new(
                        ident.span(),
                        "address field must map to address or string type",
                    ));
                }
                quote! {
                    alloy::dyn_abi::DynSolValue::FixedBytes(
                        alloy::primitives::keccak256(self.#ident.to_string().to_lowercase()),
                        32,
                    )
                }
            }
            FieldKind::String => {
                if ty_lower != "string" {
                    return Err(syn::Error::new(
                        ident.span(),
                        "string field must map to string type",
                    ));
                }
                quote! {
                    alloy::dyn_abi::DynSolValue::FixedBytes(
                        alloy::primitives::keccak256(&self.#ident),
                        32,
                    )
                }
            }
            FieldKind::Decimal => {
                if ty_lower != "string" {
                    return Err(syn::Error::new(
                        ident.span(),
                        "decimal field must map to string type",
                    ));
                }
                quote! {
                    alloy::dyn_abi::DynSolValue::FixedBytes(
                        alloy::primitives::keccak256(self.#ident.to_string()),
                        32,
                    )
                }
            }
            FieldKind::Numeric => {
                let size = parse_uint_size(&ty, ident.span())?;
                quote! {
                    alloy::dyn_abi::DynSolValue::Uint(
                        alloy::primitives::U256::from(self.#ident),
                        #size,
                    )
                }
            }
        };

        tokens.push(token);
    }

    Ok((tokens, has_nonce))
}

fn build_multisig_hash_tokens(
    fields: &syn::FieldsNamed,
    multisig_types_preimage: &str,
    multisig_types_lit: &syn::LitStr,
) -> Result<Vec<TokenStream2>, syn::Error> {
    let (field_map, _has_nonce) = build_field_map(fields)?;
    let params = parse_types_params(multisig_types_preimage)?;
    let mut tokens = Vec::with_capacity(params.len() + 1);

    tokens.push(quote! {
        alloy::dyn_abi::DynSolValue::FixedBytes(
            alloy::primitives::keccak256(#multisig_types_lit.as_bytes()),
            32,
        )
    });

    for (ty, name) in params {
        let ty_lower = ty.to_lowercase();
        match name.as_str() {
            "hyperliquidChain" => {
                if ty_lower != "string" {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "hyperliquidChain must be string",
                    ));
                }
                tokens.push(quote! {
                    alloy::dyn_abi::DynSolValue::FixedBytes(
                        alloy::primitives::keccak256(meta.signing_chain.get_hyperliquid_chain()),
                        32,
                    )
                });
            }
            "payloadMultiSigUser" => {
                if ty_lower == "address" {
                    tokens.push(quote! {
                        alloy::dyn_abi::DynSolValue::Address(payload_multi_sig_user)
                    });
                } else if ty_lower == "string" {
                    tokens.push(quote! {
                        alloy::dyn_abi::DynSolValue::FixedBytes(
                            alloy::primitives::keccak256(
                                payload_multi_sig_user.to_string().to_lowercase(),
                            ),
                            32,
                        )
                    });
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "payloadMultiSigUser must map to address or string type",
                    ));
                }
            }
            "outerSigner" => {
                if ty_lower == "address" {
                    tokens.push(quote! {
                        alloy::dyn_abi::DynSolValue::Address(outer_signer)
                    });
                } else if ty_lower == "string" {
                    tokens.push(quote! {
                        alloy::dyn_abi::DynSolValue::FixedBytes(
                            alloy::primitives::keccak256(
                                outer_signer.to_string().to_lowercase(),
                            ),
                            32,
                        )
                    });
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "outerSigner must map to address or string type",
                    ));
                }
            }
            "nonce" | "time" => {
                let field = field_map.get("nonce").ok_or_else(|| {
                    syn::Error::new(proc_macro2::Span::call_site(), "nonce field missing")
                })?;
                let size = parse_uint_size(&ty, field.ident.span())?;
                let ident = &field.ident;
                let expr = if field.is_option {
                    quote! {
                        {
                            let nonce = self.#ident.expect("nonce must be set before signing");
                            alloy::dyn_abi::DynSolValue::Uint(
                                alloy::primitives::U256::from(nonce),
                                #size,
                            )
                        }
                    }
                } else {
                    quote! {
                        alloy::dyn_abi::DynSolValue::Uint(
                            alloy::primitives::U256::from(self.#ident),
                            #size,
                        )
                    }
                };
                tokens.push(expr);
            }
            _ => {
                let field = field_map.get(&name).ok_or_else(|| {
                    syn::Error::new(proc_macro2::Span::call_site(), format!("field not found: {name}"))
                })?;
                let ident = &field.ident;
                let token = match field.kind {
                    FieldKind::Address => {
                        if ty_lower != "address" && ty_lower != "string" {
                            return Err(syn::Error::new(
                                ident.span(),
                                "address field must map to address or string type",
                            ));
                        }
                        quote! {
                            alloy::dyn_abi::DynSolValue::FixedBytes(
                                alloy::primitives::keccak256(self.#ident.to_string().to_lowercase()),
                                32,
                            )
                        }
                    }
                    FieldKind::String => {
                        if ty_lower != "string" {
                            return Err(syn::Error::new(
                                ident.span(),
                                "string field must map to string type",
                            ));
                        }
                        quote! {
                            alloy::dyn_abi::DynSolValue::FixedBytes(
                                alloy::primitives::keccak256(&self.#ident),
                                32,
                            )
                        }
                    }
                    FieldKind::Decimal => {
                        if ty_lower != "string" {
                            return Err(syn::Error::new(
                                ident.span(),
                                "decimal field must map to string type",
                            ));
                        }
                        quote! {
                            alloy::dyn_abi::DynSolValue::FixedBytes(
                                alloy::primitives::keccak256(self.#ident.to_string()),
                                32,
                            )
                        }
                    }
                    FieldKind::Numeric => {
                        let size = parse_uint_size(&ty, ident.span())?;
                        quote! {
                            alloy::dyn_abi::DynSolValue::Uint(
                                alloy::primitives::U256::from(self.#ident),
                                #size,
                            )
                        }
                    }
                };
                tokens.push(token);
            }
        }
    }

    Ok(tokens)
}

fn build_user_signed_action_impl(
    ident: &syn::Ident,
    action_type_lit: &syn::LitStr,
    types_lit: &syn::LitStr,
    struct_hash_tokens: &[TokenStream2],
    multisig_hash_tokens: &[TokenStream2],
    uses_time: bool,
) -> TokenStream2 {
    quote! {
        impl crate::exchange::action_v2::UserSignedAction for #ident {
            const ACTION_TYPE: &'static str = #action_type_lit;

            fn struct_hash(&self, chain: &crate::SigningChain) -> alloy::primitives::B256 {
                let type_hash = alloy::primitives::keccak256(#types_lit);
                let values = vec![
                    #(#struct_hash_tokens,)*
                ];
                let tuple = alloy::dyn_abi::DynSolValue::Tuple(values);
                alloy::primitives::keccak256(tuple.abi_encode())
            }
        }

        impl crate::exchange::action_v2::Action for #ident {
            fn action_type() -> &'static str {
                <Self as crate::exchange::action_v2::UserSignedAction>::ACTION_TYPE
            }

            fn is_user_signed() -> bool {
                true
            }

            fn uses_time() -> bool {
                #uses_time
            }

            fn signing_hash(
                &self,
                meta: &crate::exchange::action_v2::SigningMeta,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                Ok(<Self as crate::exchange::action_v2::UserSignedAction>::eip712_signing_hash(
                    self,
                    meta.signing_chain,
                ))
            }

            fn multisig_signing_hash(
                &self,
                meta: &crate::exchange::action_v2::SigningMeta,
                payload_multi_sig_user: alloy::primitives::Address,
                outer_signer: alloy::primitives::Address,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                let values = vec![
                    #(#multisig_hash_tokens,)*
                ];

                let domain = alloy::sol_types::eip712_domain! {
                    name: "HyperliquidSignTransaction",
                    version: "1",
                    chain_id: meta.signing_chain.signature_chain_id(),
                    verifying_contract: alloy::primitives::Address::ZERO,
                };
                let domain_hash = domain.hash_struct();

                let tuple = alloy::dyn_abi::DynSolValue::Tuple(values);
                let struct_hash = alloy::primitives::keccak256(tuple.abi_encode());

                let mut digest = [0u8; 66];
                digest[0] = 0x19;
                digest[1] = 0x01;
                digest[2..34].copy_from_slice(&domain_hash[..]);
                digest[34..66].copy_from_slice(&struct_hash[..]);

                Ok(alloy::primitives::keccak256(&digest))
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

pub(crate) fn derive_user_signed_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let (action_type_override, _, types_preimage) = match parse_action_attrs(&input.attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error().into(),
    };

    let Some(types_preimage) = types_preimage else {
        return syn::Error::new(
            input.ident.span(),
            "UserSignedAction requires #[action(types = \"...\")]",
        )
        .to_compile_error()
        .into();
    };
    let full_types_preimage = format!("HyperliquidTransaction:{types_preimage}");

    let fields = match ensure_named_fields(&input) {
        Ok(fields) => fields,
        Err(err) => return err.to_compile_error().into(),
    };

    let ident = &input.ident;
    let action_type_value =
        action_type_override.unwrap_or_else(|| ident.to_string().to_lower_camel_case());
    let action_type_lit = LitStr::new(&action_type_value, ident.span());
    let types_lit = LitStr::new(&full_types_preimage, ident.span());

    let parsed_params = match parse_types_params(&full_types_preimage) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };
    let uses_time = parsed_params
        .iter()
        .any(|(_, name)| name == "time");

    let (struct_hash_tokens, has_nonce) =
        match build_struct_hash_tokens(fields, &full_types_preimage) {
            Ok(result) => result,
            Err(err) => return err.to_compile_error().into(),
        };

    let (multisig_types_preimage, _) = match build_multisig_types(&full_types_preimage) {
            Ok(result) => result,
            Err(err) => return err.to_compile_error().into(),
        };
    let multisig_types_lit = LitStr::new(&multisig_types_preimage, ident.span());

    if !has_nonce {
        return syn::Error::new(
            input.ident.span(),
            "UserSignedAction derive requires a `nonce` field",
        )
        .to_compile_error()
        .into();
    }

    let multisig_hash_tokens = match build_multisig_hash_tokens(
        fields,
        &multisig_types_preimage,
        &multisig_types_lit,
    ) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };

    build_user_signed_action_impl(
        ident,
        &action_type_lit,
        &types_lit,
        &struct_hash_tokens,
        &multisig_hash_tokens,
        uses_time,
    )
    .into()
}

fn parse_types_params(full_types_preimage: &str) -> Result<Vec<(String, String)>, syn::Error> {
    let component =
        alloy_dyn_abi::eip712::parser::ComponentType::parse(full_types_preimage).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to parse types: {e}"),
            )
        })?;

    let mut parsed_params: Vec<(String, String)> = Vec::new();
    for prop in component.props {
        parsed_params.push((prop.ty.span().to_string(), prop.name.to_string()));
    }

    Ok(parsed_params)
}

fn parse_uint_size(ty: &str, span: proc_macro2::Span) -> Result<usize, syn::Error> {
    let ty_lower = ty.to_lowercase();
    if !ty_lower.starts_with("uint") {
        return Err(syn::Error::new(span, "numeric field must map to uint type"));
    }
    let suffix = ty_lower.trim_start_matches("uint");
    if suffix.is_empty() {
        return Ok(256);
    }
    suffix
        .parse::<usize>()
        .map_err(|_| syn::Error::new(span, "invalid uint size"))
}

fn build_multisig_types(
    full_types_preimage: &str,
) -> Result<(String, Vec<(String, String)>), syn::Error> {
    let (prefix, rest) = full_types_preimage
        .split_once(':')
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "types missing ':'"))?;
    let (struct_name, _) = rest
        .split_once('(')
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "types missing '('"))?;

    let parsed_params = parse_types_params(full_types_preimage)?;

    let mut multisig_params: Vec<(String, String)> = Vec::new();
    let mut enriched = false;
    for (ty, name) in parsed_params {
        multisig_params.push((ty, name.clone()));
        if name == "hyperliquidChain" {
            multisig_params.push(("address".to_string(), "payloadMultiSigUser".to_string()));
            multisig_params.push(("address".to_string(), "outerSigner".to_string()));
            enriched = true;
        }
    }

    let param_list = if enriched {
        multisig_params
            .iter()
            .map(|(ty, name)| format!("{ty} {name}"))
            .collect::<Vec<_>>()
            .join(",")
    } else {
        multisig_params
            .iter()
            .map(|(ty, name)| format!("{ty} {name}"))
            .collect::<Vec<_>>()
            .join(",")
    };

    let multisig_types = format!("{prefix}:{struct_name}({param_list})");
    Ok((multisig_types, multisig_params))
}

#[cfg(test)]
mod tests {
    use super::build_multisig_types;

    #[test]
    fn test_build_multisig_types_inserts_after_hyperliquid_chain() {
        let input = "HyperliquidTransaction:SendAsset(string hyperliquidChain,string destination,string amount,uint64 nonce)";
        let (full_types, params) = build_multisig_types(input).unwrap();

        assert_eq!(
            full_types,
            "HyperliquidTransaction:SendAsset(string hyperliquidChain,address payloadMultiSigUser,address outerSigner,string destination,string amount,uint64 nonce)"
        );
        assert_eq!(params[0].0, "string");
        assert_eq!(params[0].1, "hyperliquidChain");
        assert_eq!(params[1].0, "address");
        assert_eq!(params[1].1, "payloadMultiSigUser");
        assert_eq!(params[2].0, "address");
        assert_eq!(params[2].1, "outerSigner");
    }

    #[test]
    fn test_build_multisig_types_no_hyperliquid_chain() {
        let input = "HyperliquidTransaction:Other(uint64 nonce)";
        let (full_types, params) = build_multisig_types(input).unwrap();

        assert_eq!(full_types, "HyperliquidTransaction:Other(uint64 nonce)");
        assert_eq!(params, vec![("uint64".to_string(), "nonce".to_string())]);
    }
}
