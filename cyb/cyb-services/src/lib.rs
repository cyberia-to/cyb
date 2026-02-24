#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "ipfs")]
pub mod ipfs;
#[cfg(feature = "mining")]
pub mod mining;
#[cfg(feature = "db")]
pub mod server;

#[cfg(feature = "mining")]
use std::sync::Arc;

#[cfg(feature = "mining")]
use mining::MiningState;

pub struct CybServices {
    #[cfg(feature = "mining")]
    pub mining: Arc<MiningState>,
}

impl CybServices {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "mining")]
            mining: Arc::new(MiningState::new()),
        }
    }

    pub async fn start(&self) {
        #[cfg(feature = "ipfs")]
        match ipfs::start_ipfs().await {
            Ok(()) => println!("[cyb-services] IPFS daemon started"),
            Err(e) => eprintln!("[cyb-services] IPFS start failed: {:?}", e),
        }
    }
}
