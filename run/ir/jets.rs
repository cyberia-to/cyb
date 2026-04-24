//! Jet registry — map formula hash → named fused kernel.
//!
//! A *jet* is a recognized composition of atoms that a backend can execute
//! as a single kernel instead of atom-by-atom. The registry lets the
//! executor ask "is there a fused kernel for this atom sequence?" without
//! knowing about specific Op variants.
//!
//! Known sharp edge: several Op variants decompose to the same atom tail
//! (e.g. `TokenEmbed`, `Transpose`, `PosEmbed`, `RelativePosEmbedding` all
//! decompose to `[Read]`). Their formula hashes therefore collide. The
//! registry stores the first registration per hash; callers that care
//! about the distinction (i.e. dispatch) must use the `Op` type too, not
//! the hash alone. This matches the old runtime's behavior — jet lookup
//! is an *optimization hint*, not the canonical dispatch key.
//!
//! Spec: reference/runtime/ir.md

use super::atoms::{decompose, Atom, CmpOp, ReduceOp, SlidePattern};
use crate::core::op::Op;
use std::collections::HashMap;

/// Deterministic hash of an atom sequence. Stable across runs and platforms.
pub type FormulaHash = u64;

pub fn formula_hash(atoms: &[Atom]) -> FormulaHash {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for atom in atoms {
        atom.hash(&mut hasher);
    }
    hasher.finish()
}

pub struct Jet {
    pub name: &'static str,
    pub hash: FormulaHash,
    pub atoms: Vec<Atom>,
}

pub struct JetRegistry {
    jets: HashMap<FormulaHash, Jet>,
}

impl JetRegistry {
    pub fn new() -> Self {
        let mut r = Self {
            jets: HashMap::new(),
        };
        r.register_all();
        r
    }

    fn register(&mut self, name: &'static str, atoms: Vec<Atom>) {
        let hash = formula_hash(&atoms);
        // First registration wins — collisions are documented in the module
        // header. Callers that need exact Op-level routing use `Op::name()`.
        self.jets.entry(hash).or_insert(Jet { name, hash, atoms });
    }

    fn register_all(&mut self) {
        // Core linear algebra
        self.register(
            "matmul",
            vec![
                Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
                Atom::Mul,
                Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register("add", vec![Atom::Add]);
        self.register("mul", vec![Atom::Mul]);
        self.register("sub", vec![Atom::Add, Atom::Mul]);
        self.register("div", vec![Atom::Mul, Atom::Exp]);
        self.register("transpose", vec![Atom::Read]);
        self.register("concat", vec![Atom::Write]);
        self.register(
            "clamp",
            vec![Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)],
        );
        self.register(
            "nan_to_num",
            vec![Atom::Cmp(CmpOp::LessThan), Atom::Mul, Atom::Add],
        );

        // Attention
        self.register(
            "sdpa",
            vec![
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
                Atom::Exp, Atom::Reduce(ReduceOp::Sum),
                Atom::Mul,
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register("kv_cache", vec![Atom::Write, Atom::Read]);
        self.register("rope", vec![Atom::Mul, Atom::Add]);
        self.register("sinusoidal_embed", vec![Atom::Mul, Atom::Exp]);

        // Normalization
        self.register(
            "rmsnorm",
            vec![Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul],
        );
        self.register(
            "layernorm",
            vec![
                Atom::Reduce(ReduceOp::Mean), Atom::Add, Atom::Mul,
                Atom::Reduce(ReduceOp::Mean), Atom::Mul, Atom::Add,
            ],
        );
        self.register(
            "batchnorm",
            vec![Atom::Add, Atom::Mul, Atom::Mul, Atom::Add],
        );
        self.register(
            "instancenorm",
            vec![
                Atom::Reduce(ReduceOp::Mean), Atom::Add, Atom::Mul,
                Atom::Reduce(ReduceOp::Mean), Atom::Mul,
            ],
        );
        self.register("adaln", vec![Atom::Mul, Atom::Add]);

        // Activations
        self.register("silu", vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul]);
        // `gelu` shares atoms with `silu`; collision resolved by Op type.
        self.register(
            "swiglu",
            vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul, Atom::Mul],
        );
        self.register("glu", vec![Atom::Exp, Atom::Add, Atom::Mul]);
        self.register("relu", vec![Atom::Cmp(CmpOp::Max)]);
        self.register(
            "leaky_relu",
            vec![Atom::Cmp(CmpOp::Max), Atom::Mul, Atom::Add],
        );
        self.register("softmax", vec![Atom::Exp, Atom::Reduce(ReduceOp::Sum), Atom::Mul]);

        // Representative conv kernel shapes. Each (kernel, stride) tuple
        // hashes differently — register only common ones; uncommon shapes
        // fall through to atom interpreter.
        self.register(
            "conv2d_3x3",
            vec![
                Atom::Slide(SlidePattern::Window2D { kernel: (3, 3), stride: (1, 1) }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register(
            "conv1d_3",
            vec![
                Atom::Slide(SlidePattern::Window1D { kernel: 3, stride: 1 }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register(
            "conv3d_3x3x3",
            vec![
                Atom::Slide(SlidePattern::Window3D { kernel: (3, 3, 3), stride: (1, 1, 1) }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register(
            "patch_embed_16",
            vec![
                Atom::Slide(SlidePattern::Window2D { kernel: (16, 16), stride: (16, 16) }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );

        // Spatial / embedding / special
        self.register("token_embed", vec![Atom::Read]); // collides with transpose
        self.register("interpolate", vec![Atom::Read, Atom::Mul, Atom::Add]);
        self.register("unpatchify", vec![Atom::Write]);
        self.register("noise_schedule", vec![Atom::Mul, Atom::Exp]);
        self.register("flow_step", vec![Atom::Mul, Atom::Add, Atom::Exp]);
        self.register(
            "quantize",
            vec![Atom::Mul, Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)],
        );
        self.register("dequantize", vec![Atom::Mul, Atom::Add]);
        self.register(
            "sample",
            vec![
                Atom::Exp, Atom::Reduce(ReduceOp::Sum), Atom::Mul,
                Atom::Cmp(CmpOp::Max),
            ],
        );

        // Adapters
        self.register(
            "lora_apply",
            vec![
                Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
                Atom::Mul,
                Atom::Add,
            ],
        );
        self.register("kron", vec![Atom::Mul]);
        self.register(
            "matrix_inverse",
            vec![Atom::Mul, Atom::Add, Atom::Reduce(ReduceOp::Sum)],
        );

        // Fused
        self.register(
            "fused_norm_matmul",
            vec![
                Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
                Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
                Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            ],
        );
        self.register(
            "fused_skip_norm",
            vec![
                Atom::Add,
                Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
            ],
        );

        self.register(
            "argmax",
            vec![Atom::Cmp(CmpOp::GreaterThan), Atom::Reduce(ReduceOp::Max)],
        );
    }

    pub fn lookup(&self, hash: FormulaHash) -> Option<&Jet> {
        self.jets.get(&hash)
    }

    /// Advisory lookup: "is there a jet for this op's atom sequence?"
    /// Returns None for layout-only ops (empty decomposition).
    pub fn lookup_op(&self, op: &Op) -> Option<&Jet> {
        let atoms = decompose(op);
        if atoms.is_empty() {
            return None;
        }
        self.lookup(formula_hash(&atoms))
    }

    pub fn len(&self) -> usize {
        self.jets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jets.is_empty()
    }

    pub fn list(&self) -> Vec<(&str, FormulaHash)> {
        let mut out: Vec<_> = self.jets.values().map(|j| (j.name, j.hash)).collect();
        out.sort_by_key(|(name, _)| *name);
        out
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
    fn registry_populates() {
        let r = JetRegistry::new();
        assert!(r.len() > 20, "expected >20 jets, got {}", r.len());
    }

    #[test]
    fn formula_hash_deterministic() {
        let a = vec![Atom::Mul, Atom::Add];
        assert_eq!(formula_hash(&a), formula_hash(&a));
    }

    #[test]
    fn order_affects_hash() {
        let h1 = formula_hash(&[Atom::Mul, Atom::Add]);
        let h2 = formula_hash(&[Atom::Add, Atom::Mul]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn matmul_and_relu_lookup() {
        let r = JetRegistry::new();
        assert_eq!(r.lookup_op(&Op::Matmul).map(|j| j.name), Some("matmul"));
        assert_eq!(r.lookup_op(&Op::Relu).map(|j| j.name), Some("relu"));
    }

    #[test]
    fn layout_ops_skip_jet() {
        let r = JetRegistry::new();
        assert!(r
            .lookup_op(&Op::Reshape { shape: vec![1, -1] })
            .is_none());
    }

    #[test]
    fn list_sorted_by_name() {
        let r = JetRegistry::new();
        let list = r.list();
        for i in 1..list.len() {
            assert!(list[i - 1].0 <= list[i].0);
        }
    }
}
