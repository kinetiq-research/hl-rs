use proc_macro::TokenStream;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

use crate::{ensure_named_fields, parse_action_attrs};

fn build_user_signed_match_arms(
    fields: &syn::FieldsNamed,
) -> Result<(Vec<TokenStream2>, bool), syn::Error> {
    let mut match_arms = Vec::new();
    let mut has_nonce = false;

    for field in fields.named.iter() {
        let Some(name) = field.ident.as_ref() else { continue };
        let name_str = name.to_string();

        if name_str == "nonce" {
            has_nonce = true;
            let ty_str = quote! { #field.ty }.to_string();
            let expr = if ty_str.contains("Option") {
                quote! {
                    let nonce = self.#name.expect("nonce must be set before signing");
                    match parsed_ty {
                        alloy::dyn_abi::DynSolType::Uint(size) => values.push(
                            alloy::dyn_abi::DynSolValue::Uint(
                                alloy::primitives::U256::from(nonce),
                                size,
                            ),
                        ),
                        _ => panic!("nonce must map to uint type"),
                    }
                }
            } else {
                quote! {
                    match parsed_ty {
                        alloy::dyn_abi::DynSolType::Uint(size) => values.push(
                            alloy::dyn_abi::DynSolValue::Uint(
                                alloy::primitives::U256::from(self.#name),
                                size,
                            ),
                        ),
                        _ => panic!("nonce must map to uint type"),
                    }
                }
            };
            match_arms.push(quote! { #name_str => { #expr } });
            continue;
        }

        if name_str == "hyperliquid_chain" || name_str == "hyperliquidChain" {
            // handled explicitly in the type preimage loop
            continue;
        }

        let ty_str = quote! { #field.ty }.to_string();
        if ty_str.contains("Address") {
            let expr = quote! {
                match parsed_ty {
                    alloy::dyn_abi::DynSolType::Address | alloy::dyn_abi::DynSolType::String => values.push(
                        alloy::dyn_abi::DynSolValue::FixedBytes(
                            alloy::primitives::keccak256(self.#name.to_string().to_lowercase()),
                            32,
                        ),
                    ),
                    _ => panic!("address field must map to address or string type"),
                }
            };
            match_arms.push(quote! { #name_str => { #expr } });
        } else if ty_str.contains("String") || ty_str.contains("str") {
            let expr = quote! {
                match parsed_ty {
                    alloy::dyn_abi::DynSolType::String => values.push(
                        alloy::dyn_abi::DynSolValue::FixedBytes(
                            alloy::primitives::keccak256(&self.#name),
                            32,
                        ),
                    ),
                    _ => panic!("string field must map to string type"),
                }
            };
            match_arms.push(quote! { #name_str => { #expr } });
        } else if ty_str.contains("Decimal") {
            let expr = quote! {
                match parsed_ty {
                    alloy::dyn_abi::DynSolType::String => values.push(
                        alloy::dyn_abi::DynSolValue::FixedBytes(
                            alloy::primitives::keccak256(self.#name.to_string()),
                            32,
                        ),
                    ),
                    _ => panic!("decimal field must map to string type"),
                }
            };
            match_arms.push(quote! { #name_str => { #expr } });
        } else {
            let expr = quote! {
                match parsed_ty {
                    alloy::dyn_abi::DynSolType::Uint(size) => values.push(
                        alloy::dyn_abi::DynSolValue::Uint(
                            alloy::primitives::U256::from(self.#name),
                            size,
                        ),
                    ),
                    _ => panic!("numeric field must map to uint type"),
                }
            };
            match_arms.push(quote! { #name_str => { #expr } });
        }
    }

    Ok((match_arms, has_nonce))
}

fn build_user_signed_action_impl(
    ident: &syn::Ident,
    action_type_lit: &syn::LitStr,
    types_lit: &syn::LitStr,
    multisig_types_lit: &syn::LitStr,
    multisig_params: &[(String, String)],
    match_arms: &[TokenStream2],
) -> TokenStream2 {
    let multisig_param_tokens: Vec<TokenStream2> = multisig_params
        .iter()
        .map(|(ty, name)| {
            let ty_lit = LitStr::new(ty, proc_macro2::Span::call_site());
            let name_lit = LitStr::new(name, proc_macro2::Span::call_site());
            quote! { (#ty_lit, #name_lit) }
        })
        .collect();

    quote! {
        impl crate::exchange::action_v2::UserSignedAction for #ident {
            const ACTION_TYPE: &'static str = #action_type_lit;

            fn struct_hash(&self, chain: &crate::SigningChain) -> alloy::primitives::B256 {
                let type_hash = alloy::primitives::keccak256(#types_lit);
                let types_str = #types_lit;
                let (_, params) = types_str
                    .split_once(':')
                    .expect("types must include prefix like HyperliquidTransaction:Struct(...)");
                let params = params
                    .split_once('(')
                    .and_then(|(_, rest)| rest.strip_suffix(')'))
                    .expect("types must include param list");

                let mut values: Vec<alloy::dyn_abi::DynSolValue> =
                    Vec::with_capacity(params.split(',').count() + 1);
                values.push(alloy::dyn_abi::DynSolValue::FixedBytes(type_hash, 32));

                for param in params.split(',') {
                    let param = param.trim();
                    if param.is_empty() {
                        continue;
                    }
                    let mut parts = param.split_whitespace();
                    let ty_str = parts.next().expect("param type missing");
                    let name = parts.next().expect("param name missing");

                    let parsed_ty: alloy::dyn_abi::DynSolType = ty_str
                        .parse()
                        .expect("failed to parse type in preimage");

                    match name {
                        "hyperliquidChain" => match parsed_ty {
                            alloy::dyn_abi::DynSolType::String => values.push(
                                alloy::dyn_abi::DynSolValue::FixedBytes(
                                    alloy::primitives::keccak256(chain.get_hyperliquid_chain()),
                                    32,
                                ),
                            ),
                            _ => panic!("hyperliquidChain must be string"),
                        },
                        #(#match_arms,)*
                        _ => panic!("unknown param in types preimage: {}", name),
                    }
                }

                let tuple = alloy::dyn_abi::DynSolValue::Tuple(values);
                alloy::primitives::keccak256(tuple.abi_encode())
            }
        }

        impl crate::exchange::action_v2::Action for #ident {
            fn action_type(&self) -> &'static str {
                <Self as crate::exchange::action_v2::UserSignedAction>::ACTION_TYPE
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
                const MULTISIG_TYPES: &str = #multisig_types_lit;
                const MULTISIG_PARAMS: &[(&str, &str)] = &[
                    #(#multisig_param_tokens,)*
                ];

                let mut values: Vec<alloy::dyn_abi::DynSolValue> =
                    Vec::with_capacity(MULTISIG_PARAMS.len() + 1);
                values.push(alloy::dyn_abi::DynSolValue::FixedBytes(
                    alloy::primitives::keccak256(MULTISIG_TYPES.as_bytes()),
                    32,
                ));

                for (ty_str, name) in MULTISIG_PARAMS {
                    let parsed_ty: alloy::dyn_abi::DynSolType = ty_str
                        .parse()
                        .expect("failed to parse type in preimage");

                    match name {
                        "hyperliquidChain" => match parsed_ty {
                            alloy::dyn_abi::DynSolType::String => values.push(
                                alloy::dyn_abi::DynSolValue::FixedBytes(
                                    alloy::primitives::keccak256(
                                        meta.signing_chain.get_hyperliquid_chain(),
                                    ),
                                    32,
                                ),
                            ),
                            _ => panic!("hyperliquidChain must be string"),
                        },
                        "payloadMultiSigUser" => match parsed_ty {
                            alloy::dyn_abi::DynSolType::Address => values.push(
                                alloy::dyn_abi::DynSolValue::Address(payload_multi_sig_user),
                            ),
                            alloy::dyn_abi::DynSolType::String => values.push(
                                alloy::dyn_abi::DynSolValue::FixedBytes(
                                    alloy::primitives::keccak256(
                                        payload_multi_sig_user.to_string().to_lowercase(),
                                    ),
                                    32,
                                ),
                            ),
                            _ => panic!("payloadMultiSigUser must map to address or string type"),
                        },
                        "outerSigner" => match parsed_ty {
                            alloy::dyn_abi::DynSolType::Address => values.push(
                                alloy::dyn_abi::DynSolValue::Address(outer_signer),
                            ),
                            alloy::dyn_abi::DynSolType::String => values.push(
                                alloy::dyn_abi::DynSolValue::FixedBytes(
                                    alloy::primitives::keccak256(
                                        outer_signer.to_string().to_lowercase(),
                                    ),
                                    32,
                                ),
                            ),
                            _ => panic!("outerSigner must map to address or string type"),
                        },
                        #(#match_arms,)*
                        _ => panic!("unknown param in types preimage: {}", name),
                    }
                }

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

    let (action_type_override, types_preimage) = match parse_action_attrs(&input.attrs) {
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

    let (match_arms, has_nonce) = match build_user_signed_match_arms(fields) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };

    let (multisig_types_preimage, multisig_params) =
        match build_multisig_types(&full_types_preimage) {
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

    build_user_signed_action_impl(
        ident,
        &action_type_lit,
        &types_lit,
        &multisig_types_lit,
        &multisig_params,
        &match_arms,
    )
    .into()
}

fn build_multisig_types(
    full_types_preimage: &str,
) -> Result<(String, Vec<(String, String)>), syn::Error> {
    let (prefix, rest) = full_types_preimage
        .split_once(':')
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "types missing ':'"))?;
    let (struct_name, params) = rest.split_once('(').ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "types missing '('")
    })?;
    let params = params.strip_suffix(')').ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "types missing ')'")
    })?;

    let mut parsed_params: Vec<(String, String)> = Vec::new();
    for param in params.split(',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }
        let mut parts = param.split_whitespace();
        let ty = parts
            .next()
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "param type missing"))?;
        let name = parts
            .next()
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "param name missing"))?;
        parsed_params.push((ty.to_string(), name.to_string()));
    }

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
