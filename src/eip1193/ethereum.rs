use super::error::Eip1193Error;
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
extern "C" {
    #[derive(Clone, Debug)]
    /// An EIP-1193 provider object. Available by convention at `window.ethereum`
    pub(crate) type Ethereum;

    #[wasm_bindgen(method, catch)]
    pub(crate) async fn request(_: &Ethereum, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method)]
    pub(crate) fn on(_: &Ethereum, eventName: &str, listener: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method, js_name = "removeListener")]
    pub(crate) fn removeListener(
        _: &Ethereum,
        eventName: &str,
        listener: &Closure<dyn FnMut(JsValue)>,
    );
}

impl Ethereum {
    pub(crate) fn default_opt() -> Result<Self, Eip1193Error> {
        if let Ok(Some(eth)) = get_provider_js() {
            Ok(eth)
        } else {
            Err(Eip1193Error::JsNoEthereum)
        }
    }
}

#[wasm_bindgen(inline_js = "export function get_provider_js() {return window.ethereum}")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn get_provider_js() -> Result<Option<Ethereum>, JsValue>;
}
