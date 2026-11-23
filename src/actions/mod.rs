//! New action type system with strongly typed generics.
//!
//! This module provides a trait-based action system where:
//! - `L1Action` trait: for actions signed via connection_id mechanism
//! - `UserSignedAction` trait: for actions signed via EIP-712 typed data
//! - `Action` trait: unified interface auto-implemented for both
//! - `SignedAction<T>`: strongly typed signed action ready to submit
//! - `L1ActionKind`: any L1 action, hashed and signed when the type is chosen at runtime

use alloy::primitives::{Address, B256};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{Error, SigningChain};

mod core;
mod l1_action_kind;
mod l1_actions;
mod multisig;
mod multisig_validation;
mod serialization;
mod signing;
mod traits;
mod user_signed_actions;

pub use core::{PreparedAction, SignedAction, SignedActionKind};
pub use l1_action_kind::{PreparedL1Action, SignedL1Action};
pub use l1_actions::*;
pub use multisig::{
    assemble_signed_multisig_action, build_multisig_action, multisig_inner_signing_hash,
    multisig_inner_user_signed_signing_hash, multisig_outer_signing_hash, MultiSigAction,
    MultiSigPayload, MultiSigSigningPayload, MultisigSigningHashes, SignedMultiSigAction,
};
pub use multisig_validation::{
    validate_multisig_l1_action, validate_multisig_user_signed_action, MultiSigRequest,
    MultiSigRequestAction, MultiSigRequestPayload, MultisigValidation, MultisigValidationError,
};
pub use traits::{Action, L1Action, UserSignedAction};
pub use user_signed_actions::*;

pub(crate) use serialization::L1ActionWrapper;
pub(crate) use serialization::{build_action_value, build_multisig_inner_action_value};
pub(crate) use signing::{agent_signing_hash, compute_l1_hash, SigningMeta};

/// Macro that generates the `ActionKind` and `L1ActionKind` enums and the runtime dispatcher.
///
/// When adding a new action, add it to the `user_signed` or `l1` list. The dispatcher
/// automatically determines how to match each action based on its trait constants:
///
/// - **Flatten types** (`action_type == payload_key`): matched by unique `action_type`
/// - **Sum types** (`action_type != payload_key`): matched by `action_type` + `payload_key`
///
/// Sum types (like perpDeploy, spotDeploy) share the same `action_type` but are
/// discriminated by which `payload_key` is present in the JSON object.
///
/// `ActionKind` holds every action. `L1ActionKind` holds the `l1` list only, so a
/// user-signed action has no conversion into it.
macro_rules! impl_action_kind {
    (
        user_signed { $($user:ident),* $(,)? }
        l1 { $($l1:ident),* $(,)? }
    ) => {
        impl_action_kind!(@all [$($user,)* $($l1,)*] [$($l1,)*]);
    };
    (@all [$($action:ident,)*] [$($l1:ident,)*]) => {
        // Every type in the `l1` list must be an `L1Action`. `L1ActionKind` hashes
        // through `Action::signing_hash`, which would take the EIP-712 path for a
        // user-signed type, so a misplaced type fails to compile here instead.
        const _: fn() = || {
            fn is_l1_action<T: L1Action>() {}
            $(is_l1_action::<$l1>();)*
        };

        #[derive(Debug, Clone)]
        pub enum ActionKind {
            $($action($action),)*
            /// Fallback for forward-compatibility when we don't recognize `action.type`
            /// or its payload shape yet.
            Unknown {
                action_type: String,
                raw: Value,
            },
        }

        /// An L1 action of any type: one hashed as msgpack into a `connectionId` and
        /// signed as the EIP-712 `Agent` struct, the class an agent (API wallet) key
        /// signs.
        ///
        /// Use it where the action type is only known at runtime, for example when one
        /// code path signs whatever action a request asks for. It hashes and signs exactly
        /// as [`PreparedAction`] does for the concrete type it holds, and the
        /// [`SignedL1Action`] it produces serialises to the same `/exchange` body as
        /// [`SignedAction`].
        ///
        /// Every L1 action type converts into it with `From`:
        ///
        /// ```
        /// use hl_rs::{BatchCancel, L1ActionKind};
        ///
        /// let kind = L1ActionKind::from(BatchCancel::single(0, 1)).with_nonce(1_700_000_000_000);
        /// assert_eq!(kind.action_type(), "cancel");
        /// ```
        ///
        /// A user-signed action does not, because it is signed over EIP-712 typed data
        /// rather than a `connectionId`:
        ///
        /// ```compile_fail,E0277
        /// use alloy::primitives::Address;
        /// use hl_rs::{L1ActionKind, UsdSend};
        /// use rust_decimal::Decimal;
        ///
        /// let send = UsdSend { destination: Address::ZERO, amount: Decimal::ONE, nonce: None };
        /// let kind = L1ActionKind::from(send);
        /// ```
        #[derive(Debug, Clone)]
        pub enum L1ActionKind {
            $($l1($l1),)*
        }

        $(
            impl From<$l1> for L1ActionKind {
                fn from(action: $l1) -> Self {
                    L1ActionKind::$l1(action)
                }
            }
        )*

        impl From<L1ActionKind> for ActionKind {
            fn from(action: L1ActionKind) -> Self {
                match action {
                    $(L1ActionKind::$l1(a) => ActionKind::$l1(a),)*
                }
            }
        }

        /// Narrows a deserialised [`ActionKind`] to an L1 action. A user-signed or
        /// unknown action is returned unchanged as the error.
        impl TryFrom<ActionKind> for L1ActionKind {
            type Error = ActionKind;

            fn try_from(action: ActionKind) -> Result<Self, Self::Error> {
                match action {
                    $(ActionKind::$l1(a) => Ok(L1ActionKind::$l1(a)),)*
                    other => Err(other),
                }
            }
        }

        impl L1ActionKind {
            /// The action's `type` on the wire, e.g. `"order"` or `"spotDeploy"`.
            pub fn action_type(&self) -> &'static str {
                match self {
                    $(L1ActionKind::$l1(_) => <$l1 as Action>::ACTION_TYPE,)*
                }
            }

            /// The nonce set on the action, if any.
            pub fn nonce(&self) -> Option<u64> {
                match self {
                    $(L1ActionKind::$l1(a) => Action::nonce(a),)*
                }
            }

            /// Sets the nonce the action is hashed and submitted with.
            pub fn with_nonce(self, nonce: u64) -> Self {
                match self {
                    $(L1ActionKind::$l1(a) => L1ActionKind::$l1(Action::with_nonce(a, nonce)),)*
                }
            }

            /// The EIP-712 `Agent` digest a key signs for this action.
            ///
            /// Equal to [`PreparedAction::signing_hash`] for the concrete action with the
            /// same nonce, vault address and expiry.
            ///
            /// # Errors
            /// [`Error::NonceNotSet`] if the action has no nonce: unlike
            /// [`PreparedAction::new`], this never reads the clock. Also any error from
            /// hashing the action.
            pub fn signing_hash(
                &self,
                signing_chain: &SigningChain,
                vault_address: Option<Address>,
                expires_after: Option<u64>,
            ) -> Result<B256, Error> {
                let meta = SigningMeta {
                    nonce: self.nonce().ok_or(Error::NonceNotSet)?,
                    vault_address,
                    expires_after,
                    signing_chain,
                };
                match self {
                    $(L1ActionKind::$l1(a) => Action::signing_hash(a, &meta),)*
                }
            }

            /// Computes the signing hash and holds it with the action, ready for a
            /// signature.
            ///
            /// # Errors
            /// As [`L1ActionKind::signing_hash`].
            pub fn prepare(
                self,
                signing_chain: &SigningChain,
                vault_address: Option<Address>,
                expires_after: Option<u64>,
            ) -> Result<PreparedL1Action, Error> {
                let nonce = self.nonce().ok_or(Error::NonceNotSet)?;
                let signing_hash = self.signing_hash(signing_chain, vault_address, expires_after)?;
                Ok(PreparedL1Action::new(
                    self,
                    nonce,
                    vault_address,
                    expires_after,
                    signing_hash,
                ))
            }
        }

        impl serde::Serialize for SignedL1Action {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match &self.action {
                    $(L1ActionKind::$l1(a) => serialization::serialize_signed_action(
                        a,
                        self.nonce,
                        &self.signature,
                        self.vault_address,
                        self.expires_after,
                        None,
                        serializer,
                    ),)*
                }
            }
        }

        /// Dispatch a JSON action object to the appropriate `ActionKind` variant.
        ///
        /// This is generated by the `impl_action_kind!` macro and used by
        /// `SignedActionKind` deserialization.
        ///
        /// Dispatch logic:
        /// - **Flatten types** (`action_type == payload_key`): unique action_type,
        ///   return immediately on match
        /// - **Sum types** (`action_type != payload_key`): collect all matches where
        ///   both action_type AND payload_key match, then validate exactly one matched
        pub(crate) fn dispatch_action_kind<E: serde::de::Error>(
            obj: &serde_json::Map<String, serde_json::Value>,
        ) -> Result<ActionKind, E> {
            let action_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| E::custom("missing action type"))?;

            // Collect sum type matches (action_type != payload_key)
            // Flatten types return immediately since their action_type is unique
            let mut sum_type_matches: Vec<ActionKind> = Vec::new();

            $(
                if action_type == <$action as Action>::ACTION_TYPE {
                    let is_flatten = <$action as Action>::ACTION_TYPE == <$action as Action>::PAYLOAD_KEY;
                    if is_flatten {
                        // Flatten type: unique action_type, return immediately
                        return Ok(ActionKind::$action(parse_action::<$action, E>(obj)?));
                    } else if obj.contains_key(<$action as Action>::PAYLOAD_KEY) {
                        // Sum type: matched both action_type and payload_key
                        sum_type_matches.push(ActionKind::$action(parse_action::<$action, E>(obj)?));
                    }
                }
            )*

            // Handle sum type results
            match sum_type_matches.len() {
                1 => Ok(sum_type_matches.into_iter().next().unwrap()),
                0 => Ok(ActionKind::Unknown {
                    action_type: action_type.to_string(),
                    raw: serde_json::Value::Object(obj.clone()),
                }),
                _ => Err(E::custom(format!(
                    "ambiguous action type '{}' (multiple payload keys present)",
                    action_type
                ))),
            }
        }

        /// Helper to parse an action from a JSON object using the shared extraction logic.
        fn parse_action<T, E>(obj: &serde_json::Map<String, serde_json::Value>) -> Result<T, E>
        where
            T: Action + DeserializeOwned,
            E: serde::de::Error,
        {
            serialization::parse_action_from_obj(obj)
        }
    };
}

impl_action_kind! {
    user_signed {
        UsdSend,
        Withdraw,
        SpotTransfer,
        UsdClassTransfer,
        SendAsset,
        UserDexAbstraction,
        UserSetAbstraction,
        TokenDelegate,
        ConvertToMultiSigUser,
        ApproveAgent,
        ApproveBuilderFee,
    }
    l1 {
        // Flatten types
        ToggleBigBlocks,
        ReserveRequestWeight,
        NoOp,
        ClaimRewards,
        SetReferrer,
        CreateSubAccount,
        SubAccountTransfer,
        SubAccountSpotTransfer,
        VaultTransfer,
        BorrowLend,
        AgentSendAsset,
        AgentEnableDexAbstraction,
        AgentSetAbstraction,
        FinalizeEvmContract,
        // Trading actions
        UpdateLeverage,
        UpdateIsolatedMargin,
        ScheduleCancel,
        // Sum types: order actions (action_type = "order"/"cancel"/etc, payload_key varies)
        BatchOrder,
        BatchCancel,
        CancelByCloid,
        BatchModify,
        // Sum types: perpDeploy (action_type = "perpDeploy", payload_key varies)
        RegisterAsset,
        SetOpenInterestCaps,
        ToggleTrading,
        SetOracle,
        SetFeeRecipient,
        SetFeeScale,
        SetFundingMultipliers,
        SetFundingInterestRates,
        SetFundingClamps,
        SetMarginTableIds,
        InsertMarginTable,
        MigrateDexQuoteToken,
        DisableDex,
        SetSubDeployers,
        SetGrowthModes,
        SetMarginModes,
        SetPerpAnnotation,
        PerpSetDeployerFees,
        // Sum types: spotDeploy (action_type = "spotDeploy", payload_key varies)
        RegisterToken,
        Genesis,
        UserGenesis,
        RegisterSpot,
        RegisterHyperliquidity,
        SetDeployerFees,
        RequestEvmContract,
        EnableFreezePrivilege,
        FreezeUser,
        RevokeFreezePrivilege,
        EnableQuoteToken,
        DisableQuoteToken,
        SetDeployerLabel,
        SetTokenAnnotation,
    }
}
