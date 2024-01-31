use std::sync::Arc;

use ethers::{
    providers::Provider,
    types::{Address, Signature},
};
use ethers_web::{Ethereum, EthereumBuilder, EthereumError, Event, WalletType};
use log::{debug, error};
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
        let mut this = self.clone();
        this.disconnect();
        if (*self.ethereum).is_available(wallet_type) {
            spawn_local(async move {
                let mut eth = (*this.ethereum).clone();
                if eth.connect(wallet_type).await.is_ok() {
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
        if self.is_connected() {
            let mut eth = (*self.ethereum).clone();
            let ethereum = self.ethereum.clone();
            spawn_local(async move {
                let _ = eth.disconnect().await;
                ethereum.set(eth);
            });
        }
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
        *self.accounts.as_ref().and_then(|a| a.first()).unwrap_or(&Address::zero())
    }

    pub fn main_account(&self) -> String {
        self.accounts.as_ref().and_then(|a| a.first()).unwrap_or(&Address::zero()).to_string()
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        (*self.ethereum).sign_typed_data(data, from).await
    }

    pub fn run(&self) {
        let con = self.connected.clone();
        let acc = self.accounts.clone();
        let cid = self.chain_id.clone();
        let purl = self.pairing_url.clone();
        let eth = self.ethereum.clone();

        debug!("Spawning listeners");
        spawn_local(async move {
            let mut keep_looping = true;
            while keep_looping {
                match eth.next().await {
                    Ok(Some(event)) => match event {
                        Event::ConnectionWaiting(url) => {
                            debug!("{url}");
                            purl.set(Some(url));
                        }
                        Event::Connected => {
                            con.set(true);
                            purl.set(None)
                        }
                        Event::Disconnected => {
                            con.set(false);
                            keep_looping = false;
                        }
                        Event::ChainIdChanged(chain_id) => cid.set(chain_id),
                        Event::AccountsChanged(accounts) => acc.set(accounts),
                    },
                    Ok(None) => debug!("No event, continuing"),
                    Err(err) => {
                        keep_looping = false;
                        error!("Error on fetching event message {err:?}");
                    }
                }
            }
            debug!("Listener loop ended");
        });
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
    let pairing_url = use_state(move || None as Option<String>);

    let ethereum = use_state(move || builder.url("http://localhost").build());

    let con = connected.clone();
    let acc = accounts.clone();
    let cid = chain_id.clone();
    let purl = pairing_url.clone();

    use_effect_with_deps(
        move |ethereum| {
            if ethereum.has_provider() {
                debug!("Start running");
                let eth = ethereum.clone();
                spawn_local(async move {
                    let mut keep_looping = true;
                    while keep_looping {
                        match eth.next().await {
                            Ok(Some(event)) => match event {
                                Event::ConnectionWaiting(url) => {
                                    debug!("{url}");
                                    purl.set(Some(url));
                                }
                                Event::Connected => {
                                    con.set(true);
                                    purl.set(None)
                                }
                                Event::Disconnected => {
                                    con.set(false);
                                    keep_looping = false;
                                }
                                Event::ChainIdChanged(chain_id) => cid.set(chain_id),
                                Event::AccountsChanged(accounts) => acc.set(accounts),
                            },
                            Ok(None) => debug!("No event, continuing"),
                            Err(err) => {
                                keep_looping = false;
                                error!("Error on fetching event message {err:?}");
                            }
                        }
                    }
                    debug!("Listener loop ended");
                });
            }
        },
        ethereum.clone(),
    );
    UseEthereum { ethereum, connected, accounts, chain_id, pairing_url }
}
