use std::{
    borrow::Borrow,
    cell::RefCell,
    fmt::{Debug, Formatter, Result as FmtResult},
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use ethers::{
    providers::{Http, HttpClientError, JsonRpcClient, JsonRpcError, ProviderError, RpcError},
    types::{Address, Signature, SignatureError},
    utils::{hex::decode, serialize},
};
use hex::FromHexError;
use log::error;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_value, json};
use thiserror::Error;
use walletconnect_client::{prelude::*, WalletConnectState};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing RPC provider")]
    MissingProvider,

    #[error("Deadlock")]
    Deadlock,

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

impl Error {
    pub fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            Error::WalletConnectError(e) => e.as_error_respose(),
            _ => None,
        }
    }
}

impl RpcError for Error {
    fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            Error::WalletConnectError(werr) => match werr {
                WalletConnectError::WalletError(err) => Some(err),
                _ => None,
            },
            _ => None,
        }
    }

    fn as_serde_error(&self) -> Option<&serde_json::Error> {
        match self {
            Error::SerdeJsonError(err) => Some(err),
            _ => None,
        }
    }
}

impl Into<ProviderError> for Error {
    fn into(self) -> ProviderError {
        match self {
            Error::SerdeJsonError(se) => ProviderError::SerdeJson(se),
            Error::WalletConnectError(ref werr) => match werr {
                WalletConnectError::WalletError(_) => {
                    ProviderError::JsonRpcClientError(Box::new(self))
                }
                _ => ProviderError::CustomError(format!("{:?}", self)),
            },
            _ => ProviderError::CustomError(format!("{:?}", self)),
        }
    }
}

#[derive(Clone)]
pub struct WalletConnectProvider {
    client: Arc<RefCell<WalletConnect>>,
    provider: Option<Arc<RefCell<Http>>>,
}

// TODO: It's not pretiest but wasm in browsers are still single-threaded
unsafe impl Send for WalletConnectProvider {}
unsafe impl Sync for WalletConnectProvider {}

impl Debug for WalletConnectProvider {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Wallet Connect signer {:?} chain id: {}", self.address(), self.chain_id())
    }
}

impl WalletConnectProvider {
    pub fn new(client: WalletConnect, rpc_url: Option<String>) -> Self {
        let provider = match rpc_url {
            Some(url) => {
                if let Ok(p) = Http::from_str(&url) {
                    Some(Arc::new(RefCell::new(p)))
                } else {
                    None
                }
            }
            _ => None,
        };
        Self { client: Arc::new(RefCell::new(client)), provider }
    }

    pub fn get_state(&self) -> WalletConnectState {
        (*self.client).borrow().get_state()
    }

    pub async fn disconnect(&self) {
        _ = (*self.client).borrow().disconnect().await;
    }

    /// Get chain id
    pub fn chain_id(&self) -> u64 {
        (*self.client).borrow().chain_id()
    }

    /// Get chain id
    pub fn set_chain_id(&mut self, chain_id: u64) {
        (*self.client).borrow_mut().set_chain_id(chain_id);
    }

    /// Get current valid address
    pub fn address(&self) -> ethers::types::Address {
        (*self.client).borrow().address()
    }

    /// Get all accounts connected to currently set chain_id
    pub fn accounts(&self) -> Option<Vec<ethers::types::Address>> {
        self.accounts_for_chain(self.chain_id())
    }

    /// Get all accounts available for chain id
    pub fn accounts_for_chain(&self, chain_id: u64) -> Option<Vec<ethers::types::Address>> {
        (*self.client).borrow().get_accounts_for_chain_id(chain_id)
    }

    /// Get next message
    pub async fn next(&self) -> Result<Option<walletconnect_client::event::Event>, Error> {
        Ok((*self.client).borrow_mut().next().await?)
    }

    /// Builds typed data Json structure to send it to WalletConnect and sends via client's channel
    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, Error> {
        let data = serialize(&data);
        let from = serialize(from);
        let params = json!([from, data]);
        let chain_id = self.chain_id();
        let sig: String = from_value(
            self.client
                .borrow_mut()
                .request("eth_signTypedData_v4", Some(params), chain_id)
                .await?,
        )?;
        let sig = sig.strip_prefix("0x").unwrap_or(&sig);

        let sig = decode(sig)?;
        Ok(Signature::try_from(sig.as_slice())?)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for WalletConnectProvider {
    type Error = Error;

    /// Sends request via WalletConnectClient
    async fn request<T: Serialize + Send + Sync + std::fmt::Debug, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Error> {
        let params = json!(params);

        // let chain_id = self.chain_id();

        // if (*self.client).borrow().supports_method(method) {
        // Ok(from_value((*self.client).borrow_mut().request(method, Some(params), chain_id).await?)?)
        // } else {
        match &self.provider {
            Some(provider) => Ok((*provider).borrow_mut().request(method, params).await?),
            None => Err(Error::MissingProvider),
        }
        // }
    }
}
