//! `.model` writer per cyb-model.md spec.
//!
//! Phase 0: scaffolding only. Pass 8 (packaging) lands in phase 3.

use std::path::Path;

use crate::error::Result;

pub struct Model {
    pub name: String,
}

impl Model {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn write(&self, _path: impl AsRef<Path>) -> Result<()> {
        unimplemented!("Model::write — implemented in phase 3 (CT-1 §10)")
    }
}
