//! KV cache compression bridge — connects TurboQuant to the inference pipeline.
//!
//! This module provides the runtime API that the wgpu/Metal backends call
//! during the forward pass to compress/decompress KV cache entries.
//!
//! Integration points:
//! 1. Model load: create KvCompressor from model config
//! 2. KV write: after K/V projection, before cache store → compress
//! 3. Attention: before score computation → decompress keys (or use QJL estimator)
//! 4. Value readout: decompress values for weighted sum

use crate::polar_quant::{TurboQuantState, QjlKey, CompressedKV};

/// Configuration for KV cache compression
#[derive(Clone, Debug)]
pub struct KvCompressConfig {
    /// Head dimension (e.g., 128 for most models)
    pub head_dim: usize,
    /// Number of KV heads
    pub kv_heads: usize,
    /// Number of layers
    pub num_layers: usize,
    /// Whether to enable compression (can be toggled at runtime)
    pub enabled: bool,
}

/// Per-layer compressed KV cache
pub struct CompressedKvCache {
    /// Compressed keys (QJL sign bits + norms) per token
    pub keys: Vec<QjlKey>,
    /// Compressed values (PolarQuant) per token
    pub values: Vec<CompressedKV>,
}

/// Runtime KV compressor — one per model
pub struct KvCompressor {
    pub config: KvCompressConfig,
    /// TurboQuant state per head (shared across layers)
    state: TurboQuantState,
    /// Compressed cache per layer
    caches: Vec<CompressedKvCache>,
}

impl KvCompressor {
    /// Create compressor from model config.
    /// Call once at model load time (~1ms).
    pub fn new(config: KvCompressConfig) -> Self {
        let state = TurboQuantState::new(config.head_dim);
        let caches = (0..config.num_layers)
            .map(|_| CompressedKvCache {
                keys: Vec::new(),
                values: Vec::new(),
            })
            .collect();
        Self {
            config,
            state,
            caches,
        }
    }

    /// Compress and store a key vector for one head at one layer.
    /// Called after K projection + RoPE, before cache store.
    /// Input: f32 slice of head_dim values.
    pub fn store_key(&mut self, layer: usize, k: &[f32]) {
        let compressed = self.state.compress_key(k);
        self.caches[layer].keys.push(compressed);
    }

    /// Compress and store a value vector for one head at one layer.
    /// Called after V projection, before cache store.
    pub fn store_value(&mut self, layer: usize, v: &[f32]) {
        let compressed = self.state.compress_value(v);
        self.caches[layer].values.push(compressed);
    }

    /// Compute attention scores for query against all cached keys at a layer.
    /// Returns: vec of n_tokens scores (before softmax).
    /// Uses QJL unbiased estimator — no decompression needed.
    pub fn attention_scores(&self, layer: usize, q: &[f32]) -> Vec<f32> {
        self.state.attention_scores(q, &self.caches[layer].keys)
    }

    /// Decompress a value at a specific position for weighted sum.
    pub fn decompress_value(&self, layer: usize, token_idx: usize) -> Vec<f32> {
        self.state.decompress_value(&self.caches[layer].values[token_idx])
    }

    /// Compute weighted sum of decompressed values: sum(score_i * v_i)
    /// This is the value side of attention output.
    pub fn weighted_value_sum(&self, layer: usize, scores: &[f32]) -> Vec<f32> {
        let d = self.config.head_dim;
        let mut result = vec![0.0f32; d];
        for (i, &score) in scores.iter().enumerate() {
            if score.abs() < 1e-8 {
                continue; // skip near-zero attention weights
            }
            let v = self.state.decompress_value(&self.caches[layer].values[i]);
            for j in 0..d {
                result[j] += score * v[j];
            }
        }
        result
    }

    /// Number of cached tokens at a layer
    pub fn seq_len(&self, layer: usize) -> usize {
        self.caches[layer].keys.len()
    }

    /// Reset cache (new generation)
    pub fn reset(&mut self) {
        for cache in &mut self.caches {
            cache.keys.clear();
            cache.values.clear();
        }
    }

    /// Memory usage in bytes (compressed cache)
    pub fn memory_bytes(&self) -> usize {
        let mut total = 0;
        for cache in &self.caches {
            for k in &cache.keys {
                total += k.signs.len() + 4; // signs + norm
            }
            for v in &cache.values {
                total += v.indices.len() + 2; // indices + radius
            }
        }
        total
    }

    /// What the same cache would cost uncompressed (f16)
    pub fn uncompressed_bytes(&self) -> usize {
        let mut tokens = 0usize;
        for cache in &self.caches {
            tokens += cache.keys.len();
        }
        // KV = 2 * tokens * head_dim * 2 bytes (f16)
        tokens * self.config.head_dim * 2 * 2
    }

    /// Compression ratio
    pub fn compression_ratio(&self) -> f32 {
        let compressed = self.memory_bytes();
        let uncompressed = self.uncompressed_bytes();
        if compressed == 0 {
            0.0
        } else {
            uncompressed as f32 / compressed as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_compressor_basic() {
        let config = KvCompressConfig {
            head_dim: 128,
            kv_heads: 8,
            num_layers: 2,
            enabled: true,
        };
        let mut compressor = KvCompressor::new(config);

        // Simulate storing KV for 10 tokens
        let mut rng = test_rng_new();
        for _token in 0..10 {
            for layer in 0..2 {
                let k: Vec<f32> = (0..128).map(|_| rng_normal(&mut rng)).collect();
                let v: Vec<f32> = (0..128).map(|_| rng_normal(&mut rng)).collect();
                compressor.store_key(layer, &k);
                compressor.store_value(layer, &v);
            }
        }

        assert_eq!(compressor.seq_len(0), 10);
        assert_eq!(compressor.seq_len(1), 10);

        // Attention scores
        let q: Vec<f32> = (0..128).map(|_| rng_normal(&mut rng)).collect();
        let scores = compressor.attention_scores(0, &q);
        assert_eq!(scores.len(), 10);

        // Compression ratio
        let ratio = compressor.compression_ratio();
        eprintln!("KV compression ratio: {ratio:.1}x");
        eprintln!("compressed: {} bytes, uncompressed: {} bytes",
            compressor.memory_bytes(), compressor.uncompressed_bytes());
        assert!(ratio > 2.0, "compression ratio {ratio} too low");

        // Reset
        compressor.reset();
        assert_eq!(compressor.seq_len(0), 0);
    }

    // Simple RNG for tests (reuse polar_quant's SimpleRng via a wrapper)
    struct TestRng(u64);
    fn test_rng_new() -> TestRng { TestRng(12345) }
    fn rng_normal(rng: &mut TestRng) -> f32 {
        rng.0 = rng.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u1 = ((rng.0 >> 40) as f32 / (1u64 << 24) as f32).max(1e-10);
        rng.0 = rng.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u2 = (rng.0 >> 40) as f32 / (1u64 << 24) as f32;
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }
}
