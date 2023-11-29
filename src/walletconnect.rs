use ethers::types::{Address, Signature};
use log::debug;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};
use unsafe_send_sync::UnsafeSendSync;
use walletconnect_client::prelude::*;

#[derive(Clone)]
pub struct WalletConnectProvider {
    client: UnsafeSendSync<Arc<WalletConnect>>,
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
    /// Sends the request via `window.ethereum` in Js
    pub async fn request<T: Serialize + Send + Sync, R: DeserializeOwned>(
        &self,
        method: &str,
        _params: T,
    ) -> Result<R, WalletConnectError> {
        debug!("Method incoming! {method}");
        Err(WalletConnectError::Unknown)
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        _data: T,
        _from: &Address,
    ) -> Result<Signature, WalletConnectError> {
        // let provider = self.provider.deref();
        // let provider: Provider<Eip1193> = provider.borrow_mut().clone();
        // let data = serialize(&data);
        // let from = serialize(from);
        //
        // let sig: String = provider
        //     .request("eth_signTypedData_v4", [from, data])
        //     .await?;
        // let sig = sig.strip_prefix("0x").unwrap_or(&sig);
        //
        // let sig = decode(sig)?;
        // Ok(Signature::try_from(sig.as_slice())?)
        Err(WalletConnectError::Unknown)
    }
}
