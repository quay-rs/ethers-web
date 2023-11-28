use console_log;
use yew_app::{
    provider::EthereumContextProvider, 
    views::{
        button_connect::WalletButton,
        button_sign::SignatureButton,
        button_erc20::TransferButton,
    }
};
use log::Level;
use yew::prelude::*;

#[function_component]
fn App() -> Html {
    html! {
        <EthereumContextProvider>
            <WalletButton />
            <SignatureButton />
            <TransferButton />
        </EthereumContextProvider>
    }
}

fn main() {
    _ = console_log::init_with_level(Level::Debug);
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
