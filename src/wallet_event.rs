use std::fmt::Display;

pub(crate) enum WalletEvent {
    AccountsChanged,
    ChainChanged,
    // TODO: Add implementation for Connect state
    // Connect,
    Disconnect,
    // TODO: Add implementation for Message state
    // Message,
}

impl WalletEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            WalletEvent::AccountsChanged => "accountsChanged",
            WalletEvent::ChainChanged => "chainChanged",
            // TODO: Add implementation for Connect state
            // WalletEvent::Connect => "connect",
            WalletEvent::Disconnect => "disconnect",
            // TODO: Add implementation for Message state
            // WalletEvent::Message => "message",
        }
    }
}

impl Display for WalletEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
