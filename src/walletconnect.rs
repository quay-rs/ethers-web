// use ethers::signers::Signer as EthersSigner;
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};
use unsafe_send_sync::UnsafeSendSync;
use walletconnect_client::prelude::*;

#[derive(Clone)]
pub struct Signer {
    client: UnsafeSendSync<Arc<WalletConnect>>,
}

impl Debug for Signer {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Wallet Connect signer")
    }
}

// impl EthersSigner for Signer {
//     type Error = WalletConnectError;
//
//     fn address(&self) -> ethers::types::Address {
//         self.client.address()
//     }
//
//     fn chain_id(&self) -> u64 {
//         self.client.chain_id()
//     }
//     fn sign_message<'life0, 'async_trait, S>(
//         &'life0 self,
//         message: S,
//     ) -> core::pin::Pin<
//         Box<
//             dyn core::future::Future<Output = Result<ethers::types::Signature, Self::Error>>
//                 + core::marker::Send
//                 + 'async_trait,
//         >,
//     >
//     where
//         S: 'async_trait + Send + Sync + AsRef<[u8]>,
//         'life0: 'async_trait,
//         Self: 'async_trait,
//     {
//         todo!()
//     }
//
//     fn sign_typed_data<'life0, 'life1, 'async_trait, T>(
//         &'life0 self,
//         payload: &'life1 T,
//     ) -> core::pin::Pin<
//         Box<
//             dyn core::future::Future<Output = Result<ethers::types::Signature, Self::Error>>
//                 + core::marker::Send
//                 + 'async_trait,
//         >,
//     >
//     where
//         T: 'async_trait + ethers::types::transaction::eip712::Eip712 + Send + Sync,
//         'life0: 'async_trait,
//         'life1: 'async_trait,
//         Self: 'async_trait,
//     {
//         todo!()
//     }
//
//     fn with_chain_id<T: Into<u64>>(self, chain_id: T) -> Self {
//         let new_self = self.clone();
//         // TODO: Set new chain_id here
//         new_self
//     }
//
//     fn sign_transaction<'life0, 'life1, 'async_trait>(
//         &'life0 self,
//         message: &'life1 ethers::types::transaction::eip2718::TypedTransaction,
//     ) -> core::pin::Pin<
//         Box<
//             dyn core::future::Future<Output = Result<ethers::types::Signature, Self::Error>>
//                 + core::marker::Send
//                 + 'async_trait,
//         >,
//     >
//     where
//         'life0: 'async_trait,
//         'life1: 'async_trait,
//         Self: 'async_trait,
//     {
//         todo!()
//     }
// }
