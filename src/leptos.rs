use std::rc::Rc;

use crate::{Ethereum as Ethers, EthereumBuilder, EthereumError, Event, WalletType};
use ethers::{
    providers::Provider,
    types::{Address, Signature},
};
use leptos::*;
use log::{debug, error};
use serde::Serialize;

/// Structure informing about current ethereum connection state
#[derive(Debug, Clone)]
pub struct EthereumState {
    pub connected: bool,
    pub accounts: Option<Vec<Address>>,
    pub chain_id: Option<u64>,
    pub pairing_url: Option<String>,
}

/// Main component for ethereum connections. Define it as your webiste root to get the access
/// to connection state and provider `use_context::<EthereumContext>()`
#[component]
pub fn Ethereum(children: Children) -> impl IntoView {
    debug!("Creating new ethereum root");

    provide_context(EthereumContext::new());

    children()
}

/// Ethereum context for your website
#[derive(Clone, Debug)]
pub struct EthereumContext {
    pub(crate) inner: Rc<EthereumInnerContext>,
}

impl EthereumContext {
    pub(crate) fn new() -> Self {
        Self { inner: Rc::new(EthereumInnerContext::new()) }
    }

    /// Connect to the wallet (defined by type)
    pub fn connect(&self, wallet_type: WalletType) {
        self.inner.connect(wallet_type);
    }

    /// Disconnect from wallet
    pub fn disconnect(&self) {
        self.inner.disconnect();
    }

    /// Checks if any wallet is currently connected
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Gets a list of all accounts from connected wallet for chosen (and set) network
    pub fn accounts(&self) -> Option<Vec<Address>> {
        self.inner.accounts()
    }

    /// Gets current chain id of connected wallet
    pub fn chain_id(&self) -> Option<u64> {
        self.inner.chain_id()
    }

    /// Returns current pairing url if wallet connect connection is awaiting to be established
    pub fn pairing_url(&self) -> Option<String> {
        self.inner.pairing_url()
    }

    /// Gets a provider you can feed to ethers constructors to start interaction with wallet and
    /// the network
    pub fn provider(&self) -> Provider<Ethers> {
        self.inner.provider()
    }

    /// Signs typed data with the wallet
    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        self.inner.sign_typed_data(data, from).await
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EthereumInnerContext {
    ethers: ReadSignal<Ethers>,
    set_ethers: WriteSignal<Ethers>,
    state: ReadSignal<EthereumState>,
    set_state: WriteSignal<EthereumState>,
}

impl EthereumInnerContext {
    pub(crate) fn new() -> Self {
        let (state, set_state) = create_signal(EthereumState {
            connected: false,
            accounts: None,
            chain_id: None,
            pairing_url: None,
        });

        let mut builder = EthereumBuilder::new();

        if let Some(project_id) = std::option_env!("PROJECT_ID") {
            builder.walletconnect_id(project_id);
        }
        if let Some(rpc_url) = std::option_env!("RPC_URL") {
            builder.rpc_node(rpc_url);
        }

        let ethereum = builder.url("http://localhost").build();

        let (ethers, set_ethers) = create_signal(ethereum);
        Self { ethers, set_ethers, state, set_state }
    }

    pub fn connect(&self, wallet_type: WalletType) {
        debug!("Here");
        self.disconnect();
        let mut eth = self.ethers.get();
        let set_eth = self.set_ethers;
        let set_state = self.set_state;
        if eth.is_available(wallet_type) {
            debug!("There");
            spawn_local(async move {
                debug!("Everywhere");
                if eth.connect(wallet_type).await.is_ok() {
                    debug!("Got it");
                    set_eth.set(eth.clone());
                    run(eth, set_state).await;
                }
            });
        } else {
            error!("This wallet type is unavailable!");
        }
    }

    pub fn disconnect(&self) {
        if self.is_connected() {
            let mut eth = self.ethers.get();
            let set_eth = self.set_ethers;
            spawn_local(async move {
                let _ = eth.disconnect().await;
                set_eth.set(eth);
            });
        }
    }

    pub fn is_connected(&self) -> bool {
        let state = self.state.get();
        state.connected
    }

    pub fn accounts(&self) -> Option<Vec<Address>> {
        let state = self.state.get();
        state.accounts
    }

    pub fn chain_id(&self) -> Option<u64> {
        let state = self.state.get();
        state.chain_id
    }

    pub fn pairing_url(&self) -> Option<String> {
        let state = self.state.get();
        state.pairing_url
    }

    pub fn provider(&self) -> Provider<Ethers> {
        let eth = self.ethers.get();
        Provider::<Ethers>::new(eth.clone())
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        let eth = self.ethers.get();
        eth.sign_typed_data(data, from).await
    }
}

async fn run(eth: Ethers, set_state: WriteSignal<EthereumState>) {
    let mut keep_looping = true;
    let mut state =
        EthereumState { connected: false, accounts: None, chain_id: None, pairing_url: None };

    while keep_looping {
        match eth.next().await {
            Ok(Some(event)) => match event {
                Event::ConnectionWaiting(url) => {
                    state.pairing_url = Some(url);
                    set_state.set(state.clone());
                }
                Event::Connected => {
                    state.connected = true;
                    state.pairing_url = None;
                    set_state.set(state.clone());
                }
                Event::Disconnected => {
                    state.connected = false;
                    set_state.set(state.clone());
                }
                Event::Broken => {}
                Event::ChainIdChanged(chain_id) => {
                    state.chain_id = chain_id;
                    set_state.set(state.clone());
                }
                Event::AccountsChanged(accounts) => {
                    state.accounts = accounts;
                    set_state.set(state.clone());
                }
            },
            Ok(None) => {}
            Err(err) => {
                keep_looping = false;
                error!("Error on fetching event message {err:?}");
            }
        }
    }
    debug!("Listener loop ended");
}
