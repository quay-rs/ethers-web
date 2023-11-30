use console_log;
use log::Level;
use yew::prelude::*;
use yew_app::{
    provider::EthereumContextProvider,
    views::{
        button_connect::WalletButton, button_erc20::TransferButton, button_sign::SignatureButton,
        code_view::CodeView,
    },
};

#[function_component]
fn App() -> Html {
    html! {
        <EthereumContextProvider>
            <WalletButton />
            <SignatureButton />
            <TransferButton />
            <CodeView />
        </EthereumContextProvider>
    }
}

fn main() {
    _ = console_log::init_with_level(Level::Debug);
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
