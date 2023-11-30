use std::sync::Arc;

use ethers::{
    providers::Provider,
    types::{Address, Signature},
};
use ethers_web::{Ethereum, EthereumBuilder, EthereumError, Event, WalletType};
use log::error;
use serde::Serialize;
use yew::{platform::spawn_local, prelude::*};

#[derive(Clone, Debug)]
pub struct UseEthereum {
    pub ethereum: UseStateHandle<Ethereum>,
    pub connected: UseStateHandle<bool>,
    pub accounts: UseStateHandle<Option<Vec<Address>>>,
    pub chain_id: UseStateHandle<Option<u64>>,
    pub pairing_url: UseStateHandle<Option<String>>,
}

impl PartialEq for UseEthereum {
    fn eq(&self, other: &Self) -> bool {
        self.connected == other.connected
            && self.accounts == other.accounts
            && self.chain_id == other.chain_id
            && self.pairing_url == other.pairing_url
    }
}

impl UseEthereum {
    pub fn connect(&mut self, wallet_type: WalletType) {
        // We check if it is possible to connect
        let this = self.clone();
        if (*self.ethereum).is_available(wallet_type) {
            spawn_local(async move {
                let mut eth = (*this.ethereum).clone();
                let me = this.clone();
                if eth
                    .connect(
                        wallet_type,
                        Some(Arc::new(move |event| match event {
                            Event::ConnectionWaiting(url) => me.pairing_url.set(Some(url)),
                            Event::Connected => {
                                me.connected.set(true);
                                me.pairing_url.set(None)
                            }
                            Event::Disconnected => me.connected.set(false),
                            Event::ChainIdChanged(chain_id) => me.chain_id.set(chain_id),
                            Event::AccountsChanged(accounts) => me.accounts.set(accounts),
                        })),
                    )
                    .await
                    .is_ok()
                {
                    this.ethereum.set(eth);
                }
            });
        } else {
            error!("This wallet type is unavailable!");
        }
    }

    pub fn provider(&self) -> Provider<Ethereum> {
        let eth = (*self.ethereum).clone();
        Provider::<Ethereum>::new(eth)
    }

    pub fn disconnect(&mut self) {
        let mut eth = (*self.ethereum).clone();
        eth.disconnect();
        self.ethereum.set(eth);
        self.connected.set(false);
    }

    pub fn is_connected(&self) -> bool {
        *self.connected
    }

    pub fn injected_available(&self) -> bool {
        (*self.ethereum).injected_available()
    }

    pub fn walletconnect_available(&self) -> bool {
        (*self.ethereum).walletconnect_available()
    }

    pub fn chain_id(&self) -> u64 {
        (*self.chain_id).unwrap_or(0)
    }

    pub fn account(&self) -> Address {
        *self
            .accounts
            .as_ref()
            .and_then(|a| a.first())
            .unwrap_or(&Address::zero())
    }

    pub fn main_account(&self) -> String {
        self.accounts
            .as_ref()
            .and_then(|a| a.first())
            .unwrap_or(&Address::zero())
            .to_string()
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        (*self.ethereum).sign_typed_data(data, from).await
    }
}

#[hook]
pub fn use_ethereum() -> UseEthereum {
    let mut builder = EthereumBuilder::new();

    if let Some(project_id) = std::option_env!("PROJECT_ID") {
        builder.walletconnect_id(project_id);
    }
    if let Some(rpc_url) = std::option_env!("RPC_URL") {
        builder.rpc_node(rpc_url);
    }
    let connected = use_state(move || false);
    let accounts = use_state(move || None as Option<Vec<Address>>);
    let chain_id = use_state(move || None as Option<u64>);
    let ethereum = use_state(move || builder.build());
    let pairing_url = use_state(move || None as Option<String>);

    UseEthereum {
        ethereum,
        connected,
        accounts,
        chain_id,
        pairing_url,
    }
}
