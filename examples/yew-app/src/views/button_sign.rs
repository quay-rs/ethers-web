use std::collections::BTreeMap;

use crate::ethereum::UseEthereum;
use ethers::types::transaction::eip712::{EIP712Domain, Eip712DomainType, TypedData};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use yew::{platform::spawn_local, prelude::*};

const DOCUMENT_SIGNATURE_NAME: &str = "Document Signature";
const VERIFIER_NAME: &str = "Test App";

fn domain_types() -> Vec<Eip712DomainType> {
    vec![Eip712DomainType {
        name: "name".to_string(),
        r#type: "string".to_string(),
    }]
}

fn typed_data_for_document(name: &str) -> TypedData {
    let mut types = BTreeMap::new();

    types.insert("EIP712Domain".to_string(), domain_types());
    types.insert(
        DOCUMENT_SIGNATURE_NAME.to_string(),
        DocumentDescription::types(),
    );
    TypedData {
        domain: EIP712Domain {
            name: Some(VERIFIER_NAME.to_string()),
            version: Some("1".to_string()),
            chain_id: None,
            verifying_contract: None,
            salt: None,
        },
        types,
        primary_type: DOCUMENT_SIGNATURE_NAME.to_string(),
        message: DocumentDescription::new(name).into_value(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDescription {
    pub name: String,
    pub content: String,
}

impl DocumentDescription {
    pub fn types() -> Vec<Eip712DomainType> {
        vec![
            Eip712DomainType {
                name: "content".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "name".to_string(),
                r#type: "string".to_string(),
            },
        ]
    }

    pub fn into_value(&self) -> BTreeMap<String, serde_json::Value> {
        let mut types = BTreeMap::new();
        types.insert(
            "content".to_string(),
            serde_json::Value::String(self.content.clone()),
        );

        types.insert(
            "name".to_string(),
            serde_json::Value::String(self.name.clone()),
        );
        types
    }

    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            content: format!("By signing this message you comply with {}. This request will not trigger a blockchain transaction or cost any gas fees.", name),
        }
    }
}
#[function_component(SignatureButton)]
pub fn signature_button() -> Html {
    let ethereum = use_context::<UseEthereum>().expect(
        "No ethereum found. You must wrap your components in an <EthereumContextProvider />",
    );

    let onclick = {
        let ethereum = ethereum.clone();
        Callback::from(move |_: MouseEvent| {
            if ethereum.is_connected() {
                let data = typed_data_for_document("Some Document");
                let ethereum = ethereum.clone();
                spawn_local(async move {
                    let signature_res = ethereum
                        .sign_typed_data(json!(data).to_string(), &ethereum.account())
                        .await;
                    // Checking signature
                    let address = ethereum.account();
                    if let Ok(signature_res) = signature_res {
                        let recover_address = signature_res.recover_typed_data(&data).unwrap();
                        info!("Signing with {:?} recovered {:?}", address, recover_address);
                    } else {
                        error!("Signature failed");
                    }
                });
            } else {
                info!("Are we disconnected?");
            }
        })
    };
    html! {
        <button {onclick} disabled={!ethereum.is_connected()}>{"Test signature"}</button>
    }
}
