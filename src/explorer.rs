use std::collections::HashMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ExplorerResponse {
    pub listings: HashMap<String, WalletData>,
    pub count: u32,
    pub total: u32,
}

impl ExplorerResponse {
    pub fn parse_wallets(&self, project_id: &str) -> Vec<WalletDescription> {
        let mut wallets: Vec<WalletDescription> = Vec::new();
        for (_, wallet) in &self.listings {
            if let Ok(mut w) = TryInto::<WalletDescription>::try_into(wallet) {
                w.project_id = project_id.to_string();
                wallets.push(w)
            }
        }
        wallets
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct WalletData {
    pub id: String,
    pub name: String,
    pub chains: Vec<String>,
    pub image_id: String,
    pub mobile: Option<LinkSet>,
    pub desktop: Option<LinkSet>,
    pub metadata: WalletMetadata,
}

pub enum ExplorerError {
    BadWallet,
}

impl TryInto<WalletDescription> for &WalletData {
    type Error = ExplorerError;

    fn try_into(self) -> Result<WalletDescription, Self::Error> {
        let chains =
            self.chains
                .iter()
                .filter_map(|c| {
                    if c.starts_with("eip155:") {
                        c[6..].parse::<u64>().ok()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
        let mobile_schema = match &self.mobile {
            None => None,
            Some(l) => match &l.native {
                Some(url) => {
                    if !url.is_empty() {
                        Some(url.clone())
                    } else {
                        None
                    }
                }
                None => None,
            },
        };
        let desktop_schema = match &self.desktop {
            None => None,
            Some(l) => match &l.native {
                Some(url) => {
                    if !url.is_empty() {
                        Some(url.clone())
                    } else {
                        None
                    }
                }
                None => None,
            },
        };

        if mobile_schema.is_none() && desktop_schema.is_none() {
            return Err(ExplorerError::BadWallet);
        }

        Ok(WalletDescription {
            id: self.id.clone(),
            short_name: self.metadata.short_name.clone().unwrap_or(self.name.clone()),
            name: self.name.clone(),
            chains,
            image_id: self.image_id.clone(),
            project_id: "".to_owned(),
            desktop_schema,
            mobile_schema,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct LinkSet {
    pub native: Option<String>,
    pub universal: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletMetadata {
    pub short_name: Option<String>,
}

pub enum ImageSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WalletDescription {
    pub id: String,
    pub short_name: String,
    pub name: String,
    pub chains: Vec<u64>,
    pub image_id: String,
    pub project_id: String,
    pub desktop_schema: Option<String>,
    pub mobile_schema: Option<String>,
}

impl WalletDescription {
    pub fn get_image(&self, size: ImageSize) -> String {
        let size_mark = match size {
            ImageSize::Small => "sm",
            ImageSize::Medium => "md",
            ImageSize::Large => "lg",
        };
        format!(
            "https://explorer-api.walletconnect.com/v3/logo/{size_mark}/{}?projectId={}",
            self.image_id, self.project_id
        )
    }
}
