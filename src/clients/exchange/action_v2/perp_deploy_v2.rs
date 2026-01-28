use hl_rs_derive::L1Action;

macro_rules! flatten_vec {
    ($struct_name:ident, $field:ident) => {
        impl ::serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let mut data = self.$field.clone();
                data.sort_by(|a, b| a.cmp(b));
                data.serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let vec_data = Vec::deserialize(deserializer)?;
                Ok($struct_name {
                    $field: vec_data,
                    nonce: None,
                })
            }
        }
    };
}

#[derive(Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setOpenInterestCaps")]
pub struct SetOpenInterestCaps {
    pub caps: Vec<(String, u64)>,
    pub nonce: Option<u64>,
}

impl SetOpenInterestCaps {
    pub fn new(dex_name: impl Into<String>, caps: Vec<(impl Into<String>, u64)>) -> Self {
        let dex_name = dex_name.into().to_lowercase();
        Self {
            caps: caps
                .into_iter()
                .map(|(symbol, cap)| (format!("{dex_name}:{}", symbol.into().to_uppercase()), cap))
                .collect(),
            nonce: None,
        }
    }
}

flatten_vec!(SetOpenInterestCaps, caps);
