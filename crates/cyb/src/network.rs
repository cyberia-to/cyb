//! Product network endpoints for cyb.
//! Default: spacepussy-test (soft3 chaosnet) — not cosmos space-pussy on cybernode.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    /// soft3 product chaosnet.
    SpacePussyTest,
}

impl Network {
    pub const DEFAULT: Network = Network::SpacePussyTest;

    pub fn chain_id(self) -> &'static str {
        match self {
            Self::SpacePussyTest => "spacepussy-test",
        }
    }

    pub fn rpc(self) -> &'static str {
        match self {
            Self::SpacePussyTest => "http://127.0.0.1:7780",
        }
    }

    pub fn lcd(self) -> &'static str {
        match self {
            Self::SpacePussyTest => "http://127.0.0.1:7781",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "spacepussy-test" | "space-pussy-test" | "sptest" | "soft3-test" | "soft3" | "test"
            | "default" => Some(Self::SpacePussyTest),
            _ => None,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::DEFAULT
    }
}
