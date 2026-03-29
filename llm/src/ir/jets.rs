//! Jet registry -- maps formula hashes to fused GPU kernels
//!
//! A jet is a recognized composition of atoms replaced by a single GPU
//! kernel dispatch. The runtime maintains a jet registry mapping formula
//! hashes to fused implementations.
//!
//! Same atom sequence -> same hash -> same jet. Always.

use std::collections::HashMap;
use super::atoms::{Atom, CmpOp, ReduceOp, SlidePattern};

/// Formula hash -- deterministic hash of atom composition.
/// Same atom sequence always produces the same hash.
pub type FormulaHash = u64;

/// Compute formula hash from atom sequence.
///
/// This must be deterministic: identical atom sequences always produce
/// the same hash, regardless of platform or run.
pub fn formula_hash(atoms: &[Atom]) -> FormulaHash {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    for atom in atoms {
        atom.hash(&mut hasher);
    }
    hasher.finish()
}

/// A jet is a fused GPU kernel replacing a recognized atom composition
pub struct Jet {
    /// Human-readable jet name
    pub name: &'static str,
    /// Formula hash of the atom composition
    pub hash: FormulaHash,
    /// The canonical atom decomposition this jet accelerates
    pub atoms: Vec<Atom>,
}

/// Jet registry -- maps formula hash to jet.
///
/// Populated at startup, never changes. The registry covers all ~48 ops
/// from the spec.
pub struct JetRegistry {
    jets: HashMap<FormulaHash, Jet>,
}

impl JetRegistry {
    /// Create a new registry with all known jets registered
    pub fn new() -> Self {
        let mut registry = Self { jets: HashMap::new() };
        registry.register_all();
        registry
    }

    fn register(&mut self, name: &'static str, atoms: Vec<Atom>) {
        let hash = formula_hash(&atoms);
        self.jets.insert(hash, Jet {
            name,
            hash,
            atoms,
        });
    }

    fn register_all(&mut self) {
        // === Core linear algebra ===
        self.register("matmul", vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("add", vec![Atom::Add]);
        self.register("mul", vec![Atom::Mul]);
        self.register("sub", vec![Atom::Add, Atom::Mul]);
        self.register("div", vec![Atom::Mul, Atom::Exp]);
        self.register("transpose", vec![Atom::Read]);
        self.register("concat", vec![Atom::Write]);
        self.register("clamp", vec![Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)]);
        self.register("nan_to_num", vec![Atom::Cmp(CmpOp::LessThan), Atom::Mul, Atom::Add]);

        // === Attention ===
        self.register("sdpa", vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            Atom::Exp, Atom::Reduce(ReduceOp::Sum),
            Atom::Mul,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("kv_cache", vec![Atom::Write, Atom::Read]);
        self.register("rope", vec![Atom::Mul, Atom::Add]);
        self.register("sinusoidal_embed", vec![Atom::Mul, Atom::Exp]);
        self.register("relative_pos_embedding", vec![Atom::Read]);

        // === Normalization ===
        self.register("rmsnorm", vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
        ]);
        self.register("layernorm", vec![
            Atom::Reduce(ReduceOp::Mean), Atom::Add, Atom::Mul,
            Atom::Reduce(ReduceOp::Mean), Atom::Mul, Atom::Add,
        ]);
        self.register("batchnorm", vec![
            Atom::Add, Atom::Mul, Atom::Mul, Atom::Add,
        ]);
        self.register("groupnorm", vec![
            Atom::Reduce(ReduceOp::Mean), Atom::Add, Atom::Mul,
            Atom::Reduce(ReduceOp::Mean), Atom::Mul, Atom::Add,
        ]);
        self.register("instancenorm", vec![
            Atom::Reduce(ReduceOp::Mean), Atom::Add, Atom::Mul,
            Atom::Reduce(ReduceOp::Mean), Atom::Mul,
        ]);
        self.register("adaln", vec![Atom::Mul, Atom::Add]);

        // === Activation ===
        self.register("silu", vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("gelu", vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("geglu", vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul, Atom::Mul]);
        self.register("swiglu", vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul, Atom::Mul]);
        self.register("glu", vec![Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("relu", vec![Atom::Cmp(CmpOp::Max)]);
        self.register("leaky_relu", vec![Atom::Cmp(CmpOp::Max), Atom::Mul, Atom::Add]);
        self.register("prelu", vec![Atom::Cmp(CmpOp::Max), Atom::Mul, Atom::Add]);
        self.register("sigmoid", vec![Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("tanh", vec![Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("softmax", vec![Atom::Exp, Atom::Reduce(ReduceOp::Sum), Atom::Mul]);

        // === Convolution ===
        // Note: conv jets use representative kernel sizes. The formula hash
        // depends on the exact SlidePattern, so conv2d with different kernels
        // will have different hashes. We register common patterns.
        self.register("conv2d_3x3", vec![
            Atom::Slide(SlidePattern::Window2D { kernel: (3, 3), stride: (1, 1) }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("conv1d_3", vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 3, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("conv3d_3x3x3", vec![
            Atom::Slide(SlidePattern::Window3D { kernel: (3, 3, 3), stride: (1, 1, 1) }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("patch_embed_16", vec![
            Atom::Slide(SlidePattern::Window2D { kernel: (16, 16), stride: (16, 16) }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);

        // === Embedding ===
        self.register("token_embed", vec![Atom::Read]);
        // Note: token_embed and transpose/pos_embed share the same hash
        // because they all decompose to [Read]. The dispatch layer
        // distinguishes them by Op type, not formula hash alone.

        // === Spatial ===
        self.register("interpolate", vec![Atom::Read, Atom::Mul, Atom::Add]);
        self.register("unpatchify", vec![Atom::Write]);

        // === Special ===
        self.register("noise_schedule", vec![Atom::Mul, Atom::Exp]);
        self.register("flow_step", vec![Atom::Mul, Atom::Add, Atom::Exp]);
        self.register("quantize", vec![Atom::Mul, Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)]);
        self.register("dequantize", vec![Atom::Mul, Atom::Add]);
        self.register("sample", vec![
            Atom::Exp, Atom::Reduce(ReduceOp::Sum), Atom::Mul, Atom::Cmp(CmpOp::Max),
        ]);

        // === Adapter ops ===
        self.register("lora_apply", vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            Atom::Mul,
            Atom::Add,
        ]);
        self.register("kron", vec![Atom::Mul]);
        self.register("matrix_inverse", vec![Atom::Mul, Atom::Add, Atom::Reduce(ReduceOp::Sum)]);

        // === Fused ops ===
        self.register("fused_norm_matmul", vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);
        self.register("fused_skip_norm", vec![
            Atom::Add,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
        ]);
        self.register("fused_swiglu", vec![
            Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul,
            Atom::Mul,
        ]);
        self.register("flash_attention", vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            Atom::Exp, Atom::Reduce(ReduceOp::Sum),
            Atom::Mul,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ]);

        // === Legacy ===
        self.register("argmax", vec![Atom::Cmp(CmpOp::GreaterThan), Atom::Reduce(ReduceOp::Max)]);
    }

    /// Look up a jet by its formula hash
    pub fn lookup(&self, hash: FormulaHash) -> Option<&Jet> {
        self.jets.get(&hash)
    }

    /// Look up a jet by decomposing an op and hashing its atoms
    pub fn lookup_op(&self, op: &super::ops::Op) -> Option<&Jet> {
        let atoms = super::atoms::decompose(op);
        if atoms.is_empty() {
            return None; // layout-only ops have no jet
        }
        let hash = formula_hash(&atoms);
        self.lookup(hash)
    }

    /// Number of registered jets
    pub fn len(&self) -> usize {
        self.jets.len()
    }

    /// Whether the registry is empty
    pub fn is_empty(&self) -> bool {
        self.jets.is_empty()
    }

    /// List all registered jets as (name, hash) pairs
    pub fn list(&self) -> Vec<(&str, FormulaHash)> {
        let mut result: Vec<_> = self.jets.values().map(|j| (j.name, j.hash)).collect();
        result.sort_by_key(|(name, _)| *name);
        result
    }
}

impl Default for JetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_not_empty() {
        let reg = JetRegistry::new();
        assert!(reg.len() > 30, "Expected 30+ jets, got {}", reg.len());
    }

    #[test]
    fn test_formula_hash_deterministic() {
        let atoms = vec![Atom::Mul, Atom::Add];
        let h1 = formula_hash(&atoms);
        let h2 = formula_hash(&atoms);
        assert_eq!(h1, h2, "Same atoms must produce same hash");
    }

    #[test]
    fn test_different_atoms_different_hash() {
        let h1 = formula_hash(&[Atom::Mul, Atom::Add]);
        let h2 = formula_hash(&[Atom::Add, Atom::Mul]);
        assert_ne!(h1, h2, "Different atom order must produce different hash");
    }

    #[test]
    fn test_lookup_matmul() {
        let reg = JetRegistry::new();
        let jet = reg.lookup_op(&super::super::ops::Op::Matmul);
        assert!(jet.is_some(), "Matmul should have a jet");
        assert_eq!(jet.unwrap().name, "matmul");
    }

    #[test]
    fn test_lookup_relu() {
        let reg = JetRegistry::new();
        let jet = reg.lookup_op(&super::super::ops::Op::Relu);
        assert!(jet.is_some(), "Relu should have a jet");
        assert_eq!(jet.unwrap().name, "relu");
    }

    #[test]
    fn test_layout_ops_have_no_jet() {
        let reg = JetRegistry::new();
        let jet = reg.lookup_op(&super::super::ops::Op::Reshape { shape: vec![1, -1] });
        assert!(jet.is_none(), "Reshape is layout-only, no jet");
    }

    #[test]
    fn test_list_sorted() {
        let reg = JetRegistry::new();
        let list = reg.list();
        for i in 1..list.len() {
            assert!(list[i - 1].0 <= list[i].0, "List should be sorted by name");
        }
    }
}
