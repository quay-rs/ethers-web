use ethers::{
    providers::JsonRpcError,
    types::{Address, Signature, SignatureError},
    utils::{
        hex::{decode, FromHexError},
        serialize, ConversionError,
    },
};
use gloo_utils::format::JsValueSerdeExt;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use wasm_bindgen::{closure::Closure, prelude::*, JsValue};

#[wasm_bindgen]
pub struct Eip1193Request {
    method: String,
    params: JsValue,
}

#[wasm_bindgen]
impl Eip1193Request {
    pub fn new(method: String, params: JsValue) -> Eip1193Request {
        Eip1193Request { method, params }
    }

    #[wasm_bindgen(getter)]
    pub fn method(&self) -> String {
        self.method.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn params(&self) -> JsValue {
        self.params.clone()
    }
}

#[derive(Debug, Clone)]
// All attributes this library needs is thread unsafe.
// But wasm itself is a single threaded... something.
// To avoid problems with Send and Sync, all these parameters are
// fetched whenever it is needed
pub struct Eip1193 {}

#[derive(Error, Debug)]
/// Error thrown when sending an HTTP request
pub enum Eip1193Error {
    /// Thrown if the request failed
    #[error("JsValue error")]
    JsValueError(String),

    /// Thrown if no window.ethereum is found in DOM
    #[error("No ethereum found")]
    JsNoEthereum,

    #[error("Cannot parse ethereum response")]
    JsParseError,

    #[error("Not implemented yet")]
    Unimplemented,

    #[error(transparent)]
    /// Thrown if the response could not be parsed
    JsonRpcError(#[from] JsonRpcError),

    #[error(transparent)]
    /// Serde JSON Error
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    ConversionError(#[from] ConversionError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    HexError(#[from] FromHexError),
}

#[wasm_bindgen(inline_js = "export function get_provider_js() {return window.ethereum}")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn get_provider_js() -> Result<Option<Ethereum>, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[derive(Clone, Debug)]
    /// An EIP-1193 provider object. Available by convention at `window.ethereum`
    pub type Ethereum;

    #[wasm_bindgen(catch, method)]
    async fn request(_: &Ethereum, args: Eip1193Request) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method)]
    fn on(_: &Ethereum, eventName: &str, listener: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method, js_name = "removeListener")]
    fn removeListener(_: &Ethereum, eventName: &str, listener: &Closure<dyn FnMut(JsValue)>);
}

impl Ethereum {
    pub fn default() -> Result<Self, Eip1193Error> {
        if let Ok(Some(eth)) = get_provider_js() {
            return Ok(eth);
        } else {
            return Err(Eip1193Error::JsNoEthereum);
        }
    }
}

impl From<JsValue> for Eip1193Error {
    fn from(src: JsValue) -> Self {
        Eip1193Error::JsValueError(format!("{:?}", src))
    }
}

impl Eip1193 {
    /// Sends the request via `window.ethereum` in Js
    pub async fn request<T: Serialize + Send + Sync, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Eip1193Error> {
        let ethereum = Ethereum::default()?;
        let t_params = JsValue::from_serde(&params)?;
        let typename_object = JsValue::from_str("type");

        let parsed_params = if !t_params.is_null() {
            js_sys::Array::from(&t_params).map(&mut |val, _, _| {
                if let Some(trans) = js_sys::Object::try_from(&val) {
                    if let Ok(obj_type) = js_sys::Reflect::get(trans, &typename_object) {
                        if let Some(type_string) = obj_type.as_string() {
                            let t_copy = trans.clone();
                            _ = match type_string.as_str() {
                                "0x01" => js_sys::Reflect::set(
                                    &t_copy,
                                    &typename_object,
                                    &JsValue::from_str("0x1"),
                                ),
                                "0x02" => js_sys::Reflect::set(
                                    &t_copy,
                                    &typename_object,
                                    &JsValue::from_str("0x2"),
                                ),
                                "0x03" => js_sys::Reflect::set(
                                    &t_copy,
                                    &typename_object,
                                    &JsValue::from_str("0x3"),
                                ),
                                _ => Ok(true),
                            };
                            return t_copy.into();
                        }
                    }
                }

                val
            })
        } else {
            js_sys::Array::new()
        };

        let payload = Eip1193Request::new(method.to_string(), parsed_params.into());

        match ethereum.request(payload).await {
            Ok(r) => Ok(r.into_serde().unwrap()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, Eip1193Error> {
        let data = serialize(&data);
        let from = serialize(from);

        let sig: String = self.request("eth_signTypedData_v4", [from, data]).await?;
        let sig = sig.strip_prefix("0x").unwrap_or(&sig);

        let sig = decode(sig)?;
        Ok(Signature::try_from(sig.as_slice())?)
    }

    pub fn is_available() -> bool {
        Ethereum::default().is_ok()
    }

    pub fn new() -> Self {
        Eip1193 {}
    }

    pub fn on(self, event: &str, callback: Box<dyn FnMut(JsValue)>) -> Result<(), Eip1193Error> {
        let ethereum = Ethereum::default()?;
        let closure = Closure::wrap(callback);
        ethereum.on(event, &closure);
        closure.forget();
        Ok(())
    }
}
