pub mod error;
pub mod ethereum;
pub mod request;

use crate::{
    eip1193::{error::Eip1193Error, ethereum::Ethereum, request::Eip1193Request},
    event::WalletEvent,
};
use async_trait::async_trait;
use ethers::{
    providers::JsonRpcClient,
    types::{Address, Signature},
    utils::{hex::decode, serialize},
};
use futures::channel::oneshot;
use gloo_utils::format::JsValueSerdeExt;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Clone)]
// All attributes this library needs is thread unsafe.
// But wasm itself is a single threaded... something.
// To avoid problems with Send and Sync, all these parameters are
// fetched whenever it is needed
pub(crate) struct Eip1193 {}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for Eip1193 {
    type Error = Eip1193Error;

    /// Sends the request via `window.ethereum` in Js
    /// Someone should hang for T to be any type..., there is special place in hell for this
    async fn request<T: Serialize + Send + Sync, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Self::Error> {
        let (sender, receiver) = oneshot::channel();

        let m = method.to_string();

        let parsed_params = parse_params(params, &m).unwrap_or_default();
        spawn_local(async move {
            if let Ok(ethereum) = Ethereum::default_opt() {
                let payload = Eip1193Request::new(m, parsed_params);

                let response = ethereum.request(payload).await;
                let res = match response {
                    Ok(r) => match js_sys::JSON::stringify(&r) {
                        Ok(r) => Ok(r.as_string().unwrap()),
                        Err(err) => Err(err.into()),
                    },
                    Err(e) => Err(e.into()),
                };
                _ = sender.send(res);
            } else {
                _ = sender.send(Err(Eip1193Error::JsNoEthereum));
            }
        });

        let res = receiver.await.map_err(|_| Eip1193Error::CommunicationError)?;
        Ok(serde_json::from_str(&res?)?)
    }
}

impl Default for Eip1193 {
    fn default() -> Self {
        Self::new()
    }
}

impl Eip1193 {
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
        Ethereum::default_opt().is_ok()
    }

    pub fn new() -> Self {
        Eip1193 {}
    }

    pub fn on(
        self,
        event: WalletEvent,
        callback: Box<dyn FnMut(JsValue)>,
    ) -> Result<(), Eip1193Error> {
        let ethereum = Ethereum::default_opt()?;
        let closure = Closure::wrap(callback);
        ethereum.on(event.as_str(), &closure);
        closure.forget();
        Ok(())
    }
}

const METAMASK_METHOD_WITH_WRONG_IMPLEMENTATION_SIGNATURE: &str = "wallet_watchAsset";

fn parse_params<T: Serialize + Send + Sync>(
    params: T,
    method: &String,
) -> Result<JsValue, Eip1193Error> {
    let t_params = JsValue::from_serde(&params)?;
    let typename_object = JsValue::from_str("type");
    if !t_params.is_null() {
        // NOTE: Metamask experimental method with different options signature then rest of code
        // source: https://docs.metamask.io/wallet/reference/wallet_watchasset/
        if method != METAMASK_METHOD_WITH_WRONG_IMPLEMENTATION_SIGNATURE {
            let mut error = None;
            let default_result = js_sys::Array::from(&t_params)
                .map(&mut |val, _, _| {
                    if let Some(trans) = js_sys::Object::try_from(&val) {
                        if let Ok(obj_type) = js_sys::Reflect::get(trans, &typename_object) {
                            if let Some(type_string) = obj_type.as_string() {
                                let t_copy = trans.clone();
                                let result = match type_string.as_str() {
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

                                return if let Err(e) = result {
                                    error = Some(Eip1193Error::JsValueError(format!("{:?}", e)));
                                    js_sys::Array::new().into()
                                } else {
                                    t_copy.into()
                                };
                            }
                        }
                    }

                    val
                })
                .into();

            if let Some(e) = error {
                Err(e)
            } else {
                Ok(default_result)
            }
        } else {
            // NOTE: Yes, MM requires a different implementation for options for one method
            // source: https://docs.metamask.io/wallet/reference/wallet_watchasset/
            Ok(t_params)
        }
    } else {
        Ok(js_sys::Array::new().into())
    }
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use ethers::prelude::H160;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::str::FromStr;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    #[derive(Serialize, Deserialize)]
    struct UnsupportedParamsStruct {
        field1: String,
        field2: i32,
    }

    #[wasm_bindgen_test]
    fn test_wrong_params_struct_should_return_qualified_empty_array() {
        // arrange
        let params = UnsupportedParamsStruct { field1: "test".to_string(), field2: 123 };

        // optimistic act
        let result = test_parse_params_with(params, "wrong_method");

        // assert
        assert!(result.is_array());
        assert!(result.is_object());

        let js_array = result.dyn_into::<js_sys::Array>().unwrap();
        assert_eq!(js_array.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_correct_passed_params_returns_params_in_js_array() {
        // arrange
        let params = vec![
            H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            H160::from_str("0x0000000000000000000000000000000000000002").unwrap(),
            H160::from_str("0x0000000000000000000000000000000000000003").unwrap(),
        ];

        // optimistic act
        let js_value = test_parse_params_with(params, "correct_method");

        // assert
        assert_eq!(js_value.is_array(), true);
        assert_eq!(js_value.is_object(), true);

        let js_array = js_value.dyn_into::<js_sys::Array>().unwrap();
        assert_eq!(js_array.length(), 3);
        assert_eq!(
            js_array.get(0).as_string().unwrap(),
            "0x0000000000000000000000000000000000000001"
        );
        assert_eq!(
            js_array.get(1).as_string().unwrap(),
            "0x0000000000000000000000000000000000000002"
        );
        assert_eq!(
            js_array.get(2).as_string().unwrap(),
            "0x0000000000000000000000000000000000000003"
        );
    }

    #[wasm_bindgen_test]
    fn test_wrong_params_signature_for_mm_wallet_watch_asset_but_successful() {
        // arrange
        let params = json!({
            "type": "Whatever",
            "another_value": "Tralalala",
            "value_should_be_passed": "passed",
            "and_another_value_should_be_passed": "to keep another length of object",
        });

        let expected = "JsValue(Object({\"and_another_value_should_be_passed\":\"to keep another length of object\",\"another_value\":\"Tralalala\",\"type\":\"Whatever\",\"value_should_be_passed\":\"passed\"}))";

        // optimistic act
        let js_value = test_parse_params_with(params, "wallet_watchAsset");

        // assert
        assert_eq!(js_value.is_object(), true);
        assert_eq!(format!("{js_value:?}"), expected);
    }

    #[wasm_bindgen_test]
    fn test_metamask_unsupported_behavior_when_got_type_as_0x0i_instead_0xi() {
        for i in 1..4 {
            // arrange
            let internal_type = format!("0x0{}", i);
            let params = json!([{
                "type": internal_type,
                "another_value": "Tralalala",
                "value_should_be_passed": "passed",
                "and_another_value_should_be_passed": "to keep another length of object",
            }]);
            let internal_expected_type = format!("0x{}", i);

            let expected = format!("JsValue(Object({{\"and_another_value_should_be_passed\":\"to keep another length of object\",\"another_value\":\"Tralalala\",\"type\":\"{}\",\"value_should_be_passed\":\"passed\"}}))", internal_expected_type);

            // optimistic act
            let js_value = test_parse_params_with(params.clone(), "correct_method");

            // assert
            assert_eq!(js_value.is_array(), true);
            assert_eq!(js_value.is_object(), true);

            let js_array = js_value.dyn_into::<js_sys::Array>().unwrap();
            assert_eq!(js_array.length(), 1);

            let value = js_array.get(0);
            assert_eq!(value.is_object(), true);

            assert_eq!(format!("{value:?}"), expected);
        }
    }

    fn test_parse_params_with<T: Serialize + Send + Sync>(params: T, method: &str) -> JsValue {
        let result = parse_params(params, &method.to_string());
        assert!(result.is_ok());
        result.unwrap()
    }
}
