//! Default chain endpoints for cyb products.
//! Space Pussy is the install default for sync / smoke tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    SpacePussy,
    Bostrom,
}

impl Network {
    pub const DEFAULT: Network = Network::SpacePussy;

    pub fn chain_id(self) -> &'static str {
        match self {
            Self::SpacePussy => "space-pussy",
            Self::Bostrom => "bostrom",
        }
    }

    pub fn rpc(self) -> &'static str {
        match self {
            Self::SpacePussy => "https://rpc.space-pussy.cybernode.ai",
            Self::Bostrom => "https://rpc.bostrom.cybernode.ai",
        }
    }

    pub fn lcd(self) -> &'static str {
        match self {
            Self::SpacePussy => "https://lcd.space-pussy.cybernode.ai",
            Self::Bostrom => "https://lcd.bostrom.cybernode.ai",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "space-pussy" | "spacepussy" | "pussy" | "sp" => Some(Self::SpacePussy),
            "bostrom" | "boot" => Some(Self::Bostrom),
            _ => None,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::DEFAULT
    }
}
