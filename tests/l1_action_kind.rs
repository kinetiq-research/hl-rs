//! `L1ActionKind` hashes and serialises exactly as the concrete action it holds.
//!
//! For one sample of every `L1ActionKind` variant, on mainnet and testnet, with and
//! without a vault address and expiry, the dispatched signing hash must equal
//! `PreparedAction::new(concrete)`, and the `SignedL1Action` JSON must equal the
//! `SignedAction<T>` JSON byte for byte.
//!
//! The reference side is an exhaustive `match` with no wildcard, so a new variant
//! fails to compile here until it is listed, and the coverage check then fails until
//! it has a sample.

use std::collections::BTreeSet;

use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use hl_rs::{
    Action, ActionKind, AgentEnableDexAbstraction, AssetRequest, BatchCancel, DeployerFeeConfig,
    DisableDex, Error, FinalizeEvmContract, InsertMarginTable, L1ActionKind, MarginMode,
    MigrateDexQuoteToken, NoOp, PerpDexSchema, PerpSetDeployerFees, PreparedAction, RegisterAsset,
    ReserveRequestWeight, SetFeeRecipient, SetFeeScale, SetFundingClamps, SetFundingInterestRates,
    SetFundingMultipliers, SetGrowthModes, SetMarginModes, SetMarginTableIds, SetOpenInterestCaps,
    SetOracle, SetPerpAnnotation, SetSubDeployers, SignedActionKind, SigningChain,
    SubDeployerVariant, ToggleTrading, UsdSend, VaultTransfer,
};
use rust_decimal_macros::dec;
use serde_json::{json, Value};

const VECTORS: &str = include_str!("vectors/l1_wire.json");
const NONCE: u64 = 1_700_000_000_123;

fn signature() -> Signature {
    Signature::new(
        B256::repeat_byte(0x11).into(),
        B256::repeat_byte(0x22).into(),
        true,
    )
}

/// The `PreparedAction<T>` hash and the `SignedAction<T>` JSON of the concrete action.
fn concrete<A: Action + Clone>(
    action: &A,
    chain: &SigningChain,
    vault: Option<Address>,
    expires: Option<u64>,
) -> (B256, String) {
    let prepared = PreparedAction::new(action.clone(), chain, vault, expires).unwrap();
    let hash = prepared.signing_hash();
    let body = serde_json::to_string(&prepared.with_signature(signature())).unwrap();
    (hash, body)
}

macro_rules! l1_variants {
    ($($variant:ident),* $(,)?) => {
        const ALL_VARIANTS: &[&str] = &[$(stringify!($variant)),*];

        /// The variant's name and the concrete-type reference hash and body.
        fn reference(
            kind: &L1ActionKind,
            chain: &SigningChain,
            vault: Option<Address>,
            expires: Option<u64>,
        ) -> (&'static str, B256, String) {
            match kind {
                $(L1ActionKind::$variant(a) => {
                    let (hash, body) = concrete(a, chain, vault, expires);
                    (stringify!($variant), hash, body)
                })*
            }
        }
    };
}

l1_variants![
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
    FinalizeEvmContract,
    UpdateLeverage,
    UpdateIsolatedMargin,
    ScheduleCancel,
    BatchOrder,
    BatchCancel,
    CancelByCloid,
    BatchModify,
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
];

/// Every action in the SDK vector file, read through `SignedActionKind`.
fn vector_samples() -> Vec<L1ActionKind> {
    let file: Value = serde_json::from_str(VECTORS).unwrap();
    file["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let envelope = json!({
                "action": case["action"],
                "nonce": NONCE,
                "signature": {"r": "0x1", "s": "0x2", "v": 27},
            });
            let signed = SignedActionKind::from_json(&envelope.to_string()).unwrap();
            L1ActionKind::try_from(signed.action).expect("vector is an L1 action")
        })
        .collect()
}

/// The variants the vector file has no case for.
fn constructed_samples() -> Vec<L1ActionKind> {
    let user = Address::repeat_byte(0x33);
    let asset = AssetRequest {
        coin: "TEST".to_string(),
        sz_decimals: 2,
        oracle_px: dec!(10.5),
        margin_table_id: 1,
        only_isolated: false,
    };
    vec![
        ReserveRequestWeight::new(1000).into(),
        NoOp::invalidate_nonce(NONCE).into(),
        VaultTransfer::deposit(user, 1_000_000).into(),
        AgentEnableDexAbstraction::new().into(),
        FinalizeEvmContract::by_create_nonce(7, 3).into(),
        RegisterAsset::new("dex", asset)
            .deploy_dex(PerpDexSchema::new("Test DEX", 0).with_oracle_updater(user))
            .into(),
        SetOpenInterestCaps::new("dex", vec![("TEST", 1_000_000)]).into(),
        ToggleTrading::halt("dex", "TEST").into(),
        SetOracle {
            dex: "dex".to_string(),
            oracle_pxs: vec![("dex:TEST".to_string(), "10.5".to_string())],
            mark_pxs: vec![vec![("dex:TEST".to_string(), "10.4".to_string())]],
            external_perp_pxs: vec![("dex:TEST".to_string(), "10.6".to_string())],
            nonce: None,
        }
        .into(),
        SetFeeRecipient::new("dex", user).into(),
        SetFeeScale::new("dex", dec!(0.5)).into(),
        SetFundingMultipliers::new("dex", vec![("TEST", 1.5)]).into(),
        SetFundingInterestRates::new("dex", vec![("TEST", dec!(0.0001))]).into(),
        SetFundingClamps::new("dex", vec![("TEST", dec!(0.04))]).into(),
        SetMarginTableIds::new("dex", vec![("TEST", 10)]).into(),
        InsertMarginTable::new("dex", "two tiers", vec![(0, 20), (1_000_000, 10)]).into(),
        MigrateDexQuoteToken::new("dex", "dex2", PerpDexSchema::new("Test DEX 2", 0)).into(),
        DisableDex::new("dex").into(),
        SetSubDeployers::new("dex")
            .enable_permissions(user, vec![SubDeployerVariant::SetOracle])
            .into(),
        SetGrowthModes::new("dex", vec![("TEST", true)]).into(),
        SetMarginModes::new("dex", vec![("TEST", MarginMode::NoCross)]).into(),
        SetPerpAnnotation::new("dex", "TEST", "Crypto", "A test perp").into(),
        PerpSetDeployerFees::new("dex", vec![("TEST", DeployerFeeConfig::new("1.0", false))])
            .into(),
    ]
}

#[test]
fn every_variant_hashes_and_serialises_as_its_concrete_type() {
    let samples: Vec<L1ActionKind> = vector_samples()
        .into_iter()
        .chain(constructed_samples())
        .map(|kind| kind.with_nonce(NONCE))
        .collect();

    let mut covered = BTreeSet::new();
    let mut failures = Vec::new();
    let chains = [SigningChain::Mainnet, SigningChain::Testnet];
    let metas = [
        (None, None),
        (Some(Address::repeat_byte(0x44)), Some(NONCE + 60_000)),
    ];

    for kind in &samples {
        for chain in &chains {
            for (vault, expires) in metas {
                let (name, want_hash, want_body) = reference(kind, chain, vault, expires);
                covered.insert(name);

                let got_hash = kind.signing_hash(chain, vault, expires).unwrap();
                let prepared = kind.clone().prepare(chain, vault, expires).unwrap();
                let prepared_hash = prepared.signing_hash();
                let got_body =
                    serde_json::to_string(&prepared.with_signature(signature())).unwrap();

                let ctx = format!("{name} on {chain:?}, vault {vault:?}, expires {expires:?}");
                if got_hash != want_hash {
                    failures.push(format!("{ctx}: hash {got_hash} != {want_hash}"));
                }
                if prepared_hash != want_hash {
                    failures.push(format!(
                        "{ctx}: prepared hash {prepared_hash} != {want_hash}"
                    ));
                }
                if got_body != want_body {
                    failures.push(format!("{ctx}: body {got_body} != {want_body}"));
                }
            }
        }
    }

    let all: BTreeSet<&str> = ALL_VARIANTS.iter().copied().collect();
    let missing: Vec<&&str> = all.difference(&covered).collect();
    assert!(missing.is_empty(), "variants with no sample: {missing:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn action_type_and_nonce_dispatch_to_the_concrete_type() {
    for kind in vector_samples().into_iter().chain(constructed_samples()) {
        let (name, _, body) = reference(
            &kind.clone().with_nonce(NONCE),
            &SigningChain::Mainnet,
            None,
            None,
        );
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(kind.action_type(), body["action"]["type"], "{name}");
        assert_eq!(kind.clone().with_nonce(42).nonce(), Some(42), "{name}");
    }
}

#[test]
fn an_unset_nonce_is_an_error_not_the_clock() {
    let kind = L1ActionKind::from(BatchCancel::single(0, 1));
    assert_eq!(kind.nonce(), None);
    assert!(matches!(
        kind.signing_hash(&SigningChain::Mainnet, None, None),
        Err(Error::NonceNotSet)
    ));
    assert!(matches!(
        kind.prepare(&SigningChain::Testnet, None, None),
        Err(Error::NonceNotSet)
    ));
}

#[test]
fn prepared_action_carries_its_nonce_and_metadata() {
    let vault = Some(Address::repeat_byte(0x55));
    let signed = L1ActionKind::from(BatchCancel::single(0, 1))
        .with_nonce(NONCE)
        .prepare(&SigningChain::Mainnet, vault, Some(NONCE + 1))
        .unwrap()
        .with_signature(signature());
    assert_eq!(signed.nonce, NONCE);
    assert_eq!(signed.vault_address, vault);
    assert_eq!(signed.expires_after, Some(NONCE + 1));
    let body = serde_json::to_value(&signed).unwrap();
    assert_eq!(body["nonce"], NONCE);
    assert_eq!(body["expiresAfter"], NONCE + 1);
}

#[test]
fn a_user_signed_action_does_not_narrow_to_l1() {
    let send = UsdSend {
        destination: Address::ZERO,
        amount: dec!(1),
        nonce: Some(NONCE),
    };
    let kind = ActionKind::UsdSend(send);
    assert!(matches!(
        L1ActionKind::try_from(kind),
        Err(ActionKind::UsdSend(_))
    ));
}

#[test]
fn an_l1_action_widens_back_to_action_kind() {
    let kind: ActionKind = L1ActionKind::from(BatchCancel::single(3, 9)).into();
    assert!(matches!(kind, ActionKind::BatchCancel(_)));
}
