use ethers::{
    providers::{Http, HttpClientError, JsonRpcClient},
    types::{Address, Signature, SignatureError},
    utils::{hex::decode, serialize},
};
use hex::FromHexError;
use log::error;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_value, json};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    str::FromStr,
};
use thiserror::Error;
use unsafe_send_sync::UnsafeSendSync;
use walletconnect_client::{prelude::*, WalletConnectState};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing RPC provider")]
    MissingProvider,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    WalletConnectError(#[from] WalletConnectError),

    #[error(transparent)]
    HttpClientError(#[from] HttpClientError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    HexError(#[from] FromHexError),
}

#[derive(Clone)]
pub struct WalletConnectProvider {
    client: UnsafeSendSync<WalletConnect>,
    provider: Option<UnsafeSendSync<Http>>,
}

impl Debug for WalletConnectProvider {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Wallet Connect provider" // "Wallet Connect signer {:?} chain id: {}",
                                      // self.address(),
                                      // self.chain_id()
        )
    }
}

impl WalletConnectProvider {
    pub fn new(client: WalletConnect, rpc_url: Option<String>) -> Self {
        let provider = match rpc_url {
            Some(url) => {
                if let Ok(p) = Http::from_str(&url) {
                    Some(UnsafeSendSync::new(p))
                } else {
                    None
                }
            }
            _ => None,
        };
        Self { client: UnsafeSendSync::new(client), provider }
    }

    pub fn get_state(&self) -> WalletConnectState {
        self.client.get_state()
    }

    pub async fn disconnect(&self) {
        self.client.disconnect();
    }

    /// Get chain id
    pub fn chain_id(&self) -> u64 {
        self.client.chain_id()
    }

    /// Get current valid address
    pub fn address(&self) -> ethers::types::Address {
        self.client.address()
    }

    /// Get all accounts connected to currently set chain_id
    pub fn accounts(&self) -> Option<Vec<ethers::types::Address>> {
        self.accounts_for_chain(self.client.chain_id())
    }

    /// Get all accounts available for chain id
    pub fn accounts_for_chain(&self, chain_id: u64) -> Option<Vec<ethers::types::Address>> {
        self.client.get_accounts_for_chain_id(chain_id)
    }

    /// Get next message
    pub async fn next(
        &self,
    ) -> Result<Option<walletconnect_client::event::Event>, walletconnect_client::Error> {
        self.client.next().await
    }

    /// Sends request via WalletConnectClient
    pub async fn request<T: Serialize + Send + Sync, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Error> {
        let params = json!(params);

        let chain_id = self.client.chain_id();

        if self.client.supports_method(method) {
            Ok(from_value(self.client.request(method, Some(params), chain_id).await?)?)
        } else {
            if let Some(provider) = &self.provider {
                Ok(provider.request(method, params).await?)
            } else {
                Err(Error::MissingProvider)
            }
        }
    }

    /// Builds typed data Json structure to send it to WalletConnect and sends via client's channel
    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, Error> {
        let data = serialize(&data);
        let from = serialize(from);

        let sig: String = self.request("eth_signTypedData_v4", [from, data]).await?;
        let sig = sig.strip_prefix("0x").unwrap_or(&sig);

        let sig = decode(sig)?;
        Ok(Signature::try_from(sig.as_slice())?)
    }
}
