//use super::*;

//// ============================================================================
//// HIP-3 PerpDeploy Actions
//// ============================================================================

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct RegisterAsset {
    //pub max_gas: Option<u64>,
    //pub asset_request: RegisterAssetRequest,
    //pub dex: String,
    //pub schema: Option<PerpDexSchemaInput>,
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct RegisterAssetRequest {
    //pub coin: String,
    //pub sz_decimals: u64,
    //pub oracle_px: String,
    //pub margin_table_id: u64,
    //pub only_isolated: bool,
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct PerpDexSchemaInput {
    //pub full_name: String,
    //pub collateral_token: u64,
    //pub oracle_updater: Option<String>,
//}

//#[derive(Debug, Clone)]
//pub struct SetOpenInterestCaps {
    //pub caps: Vec<(String, u64)>,
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct SetSubDeployers {
    //pub dex: String,
    //pub sub_deployers: Vec<SubDeployer>,
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct SubDeployer {
    //pub variant: Variant,
    //#[serde(serialize_with = "ser_lowercase")]
    //pub user: Address,
    //pub allowed: bool,
//}

//impl SubDeployer {
    //pub fn new(variant: Variant, user: Address, allowed: bool) -> Self {
        //Self {
            //variant,
            //user,
            //allowed,
        //}
    //}

    //pub fn enable(user: Address, variant: Variant) -> Self {
        //Self::new(variant, user, true)
    //}

    //pub fn disable(user: Address, variant: Variant) -> Self {
        //Self::new(variant, user, false)
    //}

    //pub fn enable_many(user: Address, variants: impl Iterator<Item = Variant>) -> Vec<Self> {
        //variants
            //.into_iter()
            //.map(|variant| Self::new(variant, user, true))
            //.collect()
    //}

    //pub fn disable_many(user: Address, variants: impl Iterator<Item = Variant>) -> Vec<Self> {
        //variants
            //.map(|variant| Self::new(variant, user, false))
            //.collect()
    //}
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub enum Variant {
    //RegisterAsset,
    //SetOpenInterestCaps,
    //SetSubDeployers,
    //SetOracle,
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase", untagged)]
//pub enum PerpDeploy {
    //RegisterAsset { register_asset: RegisterAsset },
    //SetOpenInterestCaps {
        //set_open_interest_caps: SetOpenInterestCaps,
    //},
    //SetSubDeployers { set_sub_deployers: SetSubDeployers },
//}

//pub trait PerpDeployPayload: Serialize + Clone {
    //fn payload_key(&self) -> &'static str;
    //fn into_kind(self) -> PerpDeploy;
//}

//impl PerpDeployPayload for RegisterAsset {
    //fn payload_key(&self) -> &'static str {
        //"registerAsset"
    //}

    //fn into_kind(self) -> PerpDeploy {
        //PerpDeploy::RegisterAsset {
            //register_asset: self,
        //}
    //}
//}

//impl PerpDeployPayload for SetOpenInterestCaps {
    //fn payload_key(&self) -> &'static str {
        //"setOpenInterestCaps"
    //}

    //fn into_kind(self) -> PerpDeploy {
        //PerpDeploy::SetOpenInterestCaps {
            //set_open_interest_caps: self,
        //}
    //}
//}

//impl PerpDeployPayload for SetSubDeployers {
    //fn payload_key(&self) -> &'static str {
        //"setSubDeployers"
    //}

    //fn into_kind(self) -> PerpDeploy {
        //PerpDeploy::SetSubDeployers {
            //set_sub_deployers: self,
        //}
    //}
//}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct PerpDeployAction<T> {
    //#[serde(flatten)]
    //pub action: T,
    //#[serde(skip, default)]
    //pub nonce: Option<u64>,
//}

//impl<T> PerpDeployAction<T> {
    //pub fn new(action: T) -> Self {
        //Self { action, nonce: None }
    //}
//}

//impl PerpDeployAction<SetSubDeployers> {
    //pub fn set_sub_deployers(dex: impl Into<String>, sub_deployers: Vec<SubDeployer>) -> Self {
        //Self::new(SetSubDeployers {
            //dex: dex.into(),
            //sub_deployers,
        //})
    //}
//}

//impl<T> L1Action for PerpDeployAction<T>
//where
    //T: Serialize + Clone + Send + Sync + 'static,
//{
    //const ACTION_TYPE: &'static str = "perpDeploy";
    //const EXCLUDE_VAULT_FROM_HASH: bool = true;
//}

//impl<T> Action for PerpDeployAction<T>
//where
    //T: PerpDeployPayload + Serialize + Clone + Send + Sync + 'static,
//{
    //fn action_type(&self) -> &'static str {
        //<Self as L1Action>::ACTION_TYPE
    //}

    //fn payload_key(&self) -> &'static str {
        //self.action.payload_key()
    //}

    //fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error> {
        //let vault_for_hash = if <Self as L1Action>::EXCLUDE_VAULT_FROM_HASH {
            //None
        //} else {
            //meta.vault_address
        //};

        //let connection_id = compute_l1_hash(self, meta.nonce, vault_for_hash, meta.expires_after)?;

        //Ok(agent_signing_hash(
            //connection_id,
            //&meta.signing_chain.get_source(),
        //))
    //}

    //fn multisig_signing_hash(
        //&self,
        //meta: &SigningMeta,
        //payload_multi_sig_user: Address,
        //outer_signer: Address,
    //) -> Result<B256, Error> {
        //let envelope = (
            //payload_multi_sig_user.to_string().to_lowercase(),
            //outer_signer.to_string().to_lowercase(),
            //self,
        //);

        //let vault_for_hash = if <Self as L1Action>::EXCLUDE_VAULT_FROM_HASH {
            //None
        //} else {
            //meta.vault_address
        //};

        //let connection_id =
            //compute_l1_hash(&envelope, meta.nonce, vault_for_hash, meta.expires_after)?;

        //Ok(agent_signing_hash(
            //connection_id,
            //&meta.signing_chain.get_source(),
        //))
    //}

    //fn nonce(&self) -> Option<u64> {
        //self.nonce
    //}

    //fn extract_action_kind(&self) -> ActionKind {
        //ActionKind::PerpDeploy(self.action.clone().into_kind())
    //}

    //fn with_nonce(mut self, nonce: u64) -> Self {
        //self.nonce = Some(nonce);
        //self
    //}
//}

//macro_rules! flatten_vec {
    //($struct_name:ident, $field:ident) => {
        //impl ::serde::Serialize for $struct_name {
            //fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            //where
                //S: ::serde::Serializer,
            //{
                //let mut data = self.$field.clone();
                //data.sort_by(|a, b| a.cmp(b));
                //data.serialize(serializer)
            //}
        //}

        //impl<'de> ::serde::Deserialize<'de> for $struct_name {
            //fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            //where
                //D: ::serde::Deserializer<'de>,
            //{
                //let vec_data = Vec::deserialize(deserializer)?;
                //Ok($struct_name { $field: vec_data })
            //}
        //}
    //};
//}

//flatten_vec!(SetOpenInterestCaps, caps);