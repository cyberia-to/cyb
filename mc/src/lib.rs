//! mc — model compilation.
//!
//! Reference implementation of the [CT-1 spec](https://cyber.page/compiled-transformers-spec):
//! read a `.graph` cybergraph snapshot, produce a `.model` transformer
//! checkpoint per the [cyb-model](https://cyber.page/cyb-model) format.
//!
//! ```ignore
//! use mc::Graph;
//!
//! let g = Graph::open("bostrom-23195000.graph")?;
//! println!("snapshot {}: {} cyberlinks", g.name(), g.cyberlinks()?.count());
//! ```

pub mod error;
pub mod graph;
pub mod model;
pub mod pass;

pub use error::{McError, Result};
pub use graph::{Cyberlink, Graph};
pub use model::Model;
