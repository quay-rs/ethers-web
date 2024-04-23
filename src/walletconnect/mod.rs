pub mod error;

use self::error::Error;
use async_trait::async_trait;
use ethers::{
    providers::{Http, JsonRpcClient},
    types::{Address, Signature},
    utils::{hex::decode, serialize},
};
use futures::channel::oneshot;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_value, json};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    str::FromStr,
};
use unsafe_send_sync::UnsafeSendSync;
use walletconnect_client::{prelude::*, WalletConnectState};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub(crate) struct WalletConnectProvider {
    client: UnsafeSendSync<WalletConnect>,
    provider: Option<UnsafeSendSync<Http>>,
}

impl Debug for WalletConnectProvider {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Wallet Connect signer {:?} chain id: {}", self.address(), self.chain_id())
    }
}
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for WalletConnectProvider {
    type Error = Error;

    async fn request<T: Serialize + Send + Sync, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Error> {
        let params = json!(params);

        let chain_id = self.client.chain_id();

        if self.client.supports_method(method) {
            let (sender, receiver) = oneshot::channel();
            let m = method.to_string();
            let client = self.client.clone();
            spawn_local(async move {
                _ = sender.send(client.request(&m, Some(params), chain_id).await)
            });
            let res = receiver.await.map_err(|_| Error::CommsError)??;

            Ok(from_value(res)?)
        } else if let Some(provider) = &self.provider {
            Ok(provider.request(method, params).await?)
        } else {
            Err(Error::MissingProvider)
        }
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
        _ = self.client.disconnect().await;
    }

    /// Get chain id
    pub fn chain_id(&self) -> u64 {
        self.client.chain_id()
    }

    /// Get chain id
    pub fn set_chain_id(&mut self, chain_id: u64) {
        self.client.set_chain_id(chain_id)
    }

    /// Get current valid address
    pub fn address(&self) -> Address {
        self.client.address()
    }

    /// Get all accounts connected to currently set chain_id
    pub fn accounts(&self) -> Option<Vec<Address>> {
        self.accounts_for_chain(self.client.chain_id())
    }

    /// Get all accounts available for chain id
    pub fn accounts_for_chain(&self, chain_id: u64) -> Option<Vec<Address>> {
        self.client.get_accounts_for_chain_id(chain_id)
    }

    /// Get next message
    pub async fn next(&self) -> Result<Option<Event>, WalletConnectError> {
        self.client.next().await
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
