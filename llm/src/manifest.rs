//! Soma model manifest — the source of truth for what models should exist on disk.
//!
//! Each model has a canonical name, HuggingFace repo, target format, and expected size.
//! The manifest drives audit, clean, and fetch commands.

use std::path::{Path, PathBuf};

/// Where local models live
pub fn models_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("llm")
}

/// Quantization level for a model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quant {
    F16,
    Q8,
    Q4,
    Native, // bitnet 1.58-bit, fasttext .bin — keep as-is
}

impl std::fmt::Display for Quant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::F16 => write!(f, "f16"),
            Self::Q8 => write!(f, "q8"),
            Self::Q4 => write!(f, "q4"),
            Self::Native => write!(f, "native"),
        }
    }
}

/// Runtime format on disk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Safetensors,
    Onnx,
    Gguf,
    Ggml,
    Pytorch,
    Bin, // raw binary (fasttext, etc.)
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safetensors => write!(f, "safetensors"),
            Self::Onnx => write!(f, "onnx"),
            Self::Gguf => write!(f, "gguf"),
            Self::Ggml => write!(f, "ggml"),
            Self::Pytorch => write!(f, "pytorch"),
            Self::Bin => write!(f, "bin"),
        }
    }
}

/// Tier in the soma architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Always-on cognitive substrate (~1.5GB total)
    Tier0,
    /// Fast on-demand (<1s-2s load, <3GB each)
    Tier1,
    /// Quality on-demand (3-6s load, 5-8GB each)
    Tier2,
    /// Perception + generation media models
    Media,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tier0 => write!(f, "tier0"),
            Self::Tier1 => write!(f, "tier1"),
            Self::Tier2 => write!(f, "tier2"),
            Self::Media => write!(f, "media"),
        }
    }
}

/// A model in the soma manifest
#[derive(Debug, Clone)]
pub struct ModelSpec {
    /// Flat directory name in ~/llm/
    pub name: &'static str,
    /// HuggingFace repo id (empty if non-HF source)
    pub hf_repo: &'static str,
    /// Soma tier
    pub tier: Tier,
    /// Soma slot/role name
    pub role: &'static str,
    /// Current format on disk
    pub current_format: Format,
    /// Target quantization (for future conversion)
    pub target_quant: Quant,
    /// Expected size on disk in GB (after optimization)
    pub expected_gb: f32,
    /// Files that should be kept (glob patterns)
    pub keep_patterns: &'static [&'static str],
    /// Files to delete if present (duplicates)
    pub delete_patterns: &'static [&'static str],
    /// Whether to download from HF (vs custom source)
    pub hf_download: bool,
    /// Short description
    pub notes: &'static str,
}

/// The canonical model manifest — all soma models.
///
/// Quality-preserving quantization strategy:
/// - Router/intent (<1B, critical path): FP16 — errors cascade, keep precision
/// - Classifiers/encoders: Q8 — accuracy-sensitive but small
/// - General LLMs (1-14B): Q4 — standard, minimal quality loss
/// - Math reasoning (mimo): Q8 — math most sensitive to quantization
/// - Small vision (moondream2): Q8 — small VLM degrades at Q4
/// - Large vision (qwen2.5-vl): Q8 — vision encoder sensitive
/// - Native formats: as-is (bitnet 1.58-bit, fasttext, GGML whisper)
pub const MANIFEST: &[ModelSpec] = &[
    // ── Tier 0: cognitive substrate (always-on) ──────────────────
    ModelSpec {
        name: "qwen3-0.6b-abl",
        hf_repo: "huihui-ai/Qwen3-0.6B-abliterated",
        tier: Tier::Tier0,
        role: "0.1 router",
        current_format: Format::Safetensors,
        target_quant: Quant::F16,
        expected_gb: 0.6,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "LLM router, dual-mode. Small — FP16 preserves quality",
    },
    ModelSpec {
        name: "jina-v5-nano",
        hf_repo: "jinaai/jina-embeddings-v5-text-nano-retrieval",
        tier: Tier::Tier0,
        role: "0.2 embedding",
        current_format: Format::Onnx,
        target_quant: Quant::Q4,
        expected_gb: 0.15,
        keep_patterns: &["onnx/*", "*.json"],
        delete_patterns: &["*.gguf"],
        hf_download: true,
        notes: "239M, 768-dim, matryoshka. Already has ONNX Q4",
    },
    ModelSpec {
        name: "deberta-zeroshot",
        hf_repo: "MoritzLaurer/deberta-v3-base-zeroshot-v2.0",
        tier: Tier::Tier0,
        role: "0.3 urgency",
        current_format: Format::Onnx,
        target_quant: Quant::Q8,
        expected_gb: 0.18,
        keep_patterns: &["onnx/*", "*.json", "spm.model"],
        delete_patterns: &["model.safetensors"],
        hf_download: true,
        notes: "Zero-shot NLI classifier. Q8 for accuracy",
    },
    ModelSpec {
        name: "glotlid",
        hf_repo: "cis-lmu/glotlid",
        tier: Tier::Tier0,
        role: "0.4 language",
        current_format: Format::Bin,
        target_quant: Quant::Native,
        expected_gb: 1.6,
        keep_patterns: &["model.bin"],
        delete_patterns: &[],
        hf_download: true,
        notes: "fasttext-rs loads .bin directly. 2102 natural langs",
    },
    ModelSpec {
        name: "qwen2.5-0.5b-abl",
        hf_repo: "huihui-ai/Qwen2.5-0.5B-Instruct-abliterated-v3",
        tier: Tier::Tier0,
        role: "0.5 intent",
        current_format: Format::Safetensors,
        target_quant: Quant::F16,
        expected_gb: 0.5,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Intent extraction, constrained JSON. Small — FP16",
    },
    ModelSpec {
        name: "modernbert",
        hf_repo: "answerdotai/ModernBERT-base",
        tier: Tier::Tier0,
        role: "0.6 anomaly",
        current_format: Format::Onnx,
        target_quant: Quant::Q4,
        expected_gb: 0.22,
        keep_patterns: &["onnx/*", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Anomaly detection backbone. Already has ONNX Q4",
    },
    ModelSpec {
        name: "smollm2-360m",
        hf_repo: "HuggingFaceTB/SmolLM2-360M-Instruct",
        tier: Tier::Tier0,
        role: "0.7 splitter",
        current_format: Format::Onnx,
        target_quant: Quant::F16,
        expected_gb: 0.85,
        keep_patterns: &["onnx/*", "*.json"],
        delete_patterns: &["model.safetensors"],
        hf_download: true,
        notes: "Generative splitting with priority labels",
    },
    ModelSpec {
        name: "granite-hap-125m",
        hf_repo: "ibm-granite/granite-guardian-hap-125m",
        tier: Tier::Tier0,
        role: "0.8a injection",
        current_format: Format::Onnx,
        target_quant: Quant::Q8,
        expected_gb: 0.12,
        keep_patterns: &["onnx_q8/*", "*.json"],
        delete_patterns: &["model.safetensors", "onnx/model.onnx"],
        hf_download: true,
        notes: "Binary classifier. Q8 for accuracy. Keep only Q8 ONNX",
    },
    ModelSpec {
        name: "granite-hap-38m",
        hf_repo: "ibm-granite/granite-guardian-hap-38m",
        tier: Tier::Tier0,
        role: "0.8b injection",
        current_format: Format::Onnx,
        target_quant: Quant::Q8,
        expected_gb: 0.04,
        keep_patterns: &["onnx_q8/*", "*.json"],
        delete_patterns: &["model.safetensors", "onnx/model.onnx"],
        hf_download: true,
        notes: "Binary classifier. Q8 for accuracy. Keep only Q8 ONNX",
    },
    // ── Tier 1: fast on-demand ───────────────────────────────────
    ModelSpec {
        name: "bitnet-2b",
        hf_repo: "microsoft/bitnet-b1.58-2B-4T",
        tier: Tier::Tier1,
        role: "workhorse",
        current_format: Format::Safetensors,
        target_quant: Quant::Native,
        expected_gb: 1.1,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Native 1.58-bit, 4T tokens. No conversion needed",
    },
    ModelSpec {
        name: "qwen3.5-4b-abl",
        hf_repo: "huihui-ai/Huihui-Qwen3.5-4B-abliterated",
        tier: Tier::Tier1,
        role: "generalist",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 2.5,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "4B generalist. Q4 for disk, runtime loads natively",
    },
    ModelSpec {
        name: "nuextract-1.5",
        hf_repo: "numind/NuExtract-1.5",
        tier: Tier::Tier1,
        role: "extraction",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 2.0,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Entity extraction specialist. Beats GPT-4o on extraction",
    },
    ModelSpec {
        name: "qwen2.5-coder-1.5b-abl",
        hf_repo: "huihui-ai/Qwen2.5-Coder-1.5B-Instruct-abliterated",
        tier: Tier::Tier1,
        role: "coder-small",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 1.0,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Code specialist, small. Abliterated",
    },
    // ── Tier 2: quality on-demand ────────────────────────────────
    ModelSpec {
        name: "qwen3.5-9b-abl",
        hf_repo: "huihui-ai/Huihui-Qwen3.5-9B-abliterated",
        tier: Tier::Tier2,
        role: "generalist",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 5.5,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "General reasoning, MMLU-Pro 82.5. Q4 standard for 9B",
    },
    ModelSpec {
        name: "qwen2.5-coder-14b-abl",
        hf_repo: "huihui-ai/Qwen2.5-Coder-14B-Instruct-abliterated",
        tier: Tier::Tier2,
        role: "coder",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 5.0,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Code generation, SQL. Q4 standard for 14B",
    },
    ModelSpec {
        name: "mimo-7b-rl",
        hf_repo: "XiaomiMiMo/MiMo-7B-RL",
        tier: Tier::Tier2,
        role: "reasoning-math",
        current_format: Format::Safetensors,
        target_quant: Quant::Q8,
        expected_gb: 8.0,
        keep_patterns: &["*.safetensors", "*.json", "*.py"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Math reasoning, AIME 55.4. Q8 — math most sensitive to quant",
    },
    ModelSpec {
        name: "deepseek-r1-8b-abl",
        hf_repo: "huihui-ai/DeepSeek-R1-0528-Qwen3-8B-abliterated",
        tier: Tier::Tier2,
        role: "reasoning-cot",
        current_format: Format::Safetensors,
        target_quant: Quant::Q4,
        expected_gb: 5.0,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Chain-of-thought reasoning. Q4 ok — reasoning in tokens not weights",
    },
    ModelSpec {
        name: "qwen2.5-vl-7b-abl",
        hf_repo: "huihui-ai/Qwen2.5-VL-7B-Instruct-abliterated",
        tier: Tier::Tier2,
        role: "vision",
        current_format: Format::Safetensors,
        target_quant: Quant::Q8,
        expected_gb: 8.5,
        keep_patterns: &["*.safetensors", "*.json"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Vision + video, OCR. Q8 — vision encoder sensitive to quant",
    },
    // ── Media: always-on perception ──────────────────────────────
    ModelSpec {
        name: "whisper-small",
        hf_repo: "",
        tier: Tier::Media,
        role: "stt",
        current_format: Format::Ggml,
        target_quant: Quant::Native,
        expected_gb: 0.46,
        keep_patterns: &["*.bin"],
        delete_patterns: &[],
        hf_download: false,
        notes: "whisper.cpp GGML format. Downloaded separately",
    },
    ModelSpec {
        name: "piper-tts",
        hf_repo: "",
        tier: Tier::Media,
        role: "tts",
        current_format: Format::Onnx,
        target_quant: Quant::Native,
        expected_gb: 0.18,
        keep_patterns: &["**/*.onnx", "**/*.json"],
        delete_patterns: &[],
        hf_download: false,
        notes: "VITS ONNX voices. Downloaded from piper releases",
    },
    ModelSpec {
        name: "yolo11n",
        hf_repo: "",
        tier: Tier::Media,
        role: "detection",
        current_format: Format::Pytorch,
        target_quant: Quant::Native,
        expected_gb: 0.005,
        keep_patterns: &["*.pt"],
        delete_patterns: &[],
        hf_download: false,
        notes: "YOLOv11 nano. 3.2M params",
    },
    ModelSpec {
        name: "beats",
        hf_repo: "camenduru/beats",
        tier: Tier::Media,
        role: "audio-events",
        current_format: Format::Pytorch,
        target_quant: Quant::Native,
        expected_gb: 0.35,
        keep_patterns: &["*.pt"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Audio event detection, 527 AudioSet classes",
    },
    // ── Media: on-demand generation ──────────────────────────────
    ModelSpec {
        name: "moondream2",
        hf_repo: "vikhyatk/moondream2",
        tier: Tier::Media,
        role: "vision-light",
        current_format: Format::Safetensors,
        target_quant: Quant::Q8,
        expected_gb: 2.0,
        keep_patterns: &["*.safetensors", "*.json", "*.py"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Lightweight vision Q&A. Q8 — small VLM sensitive to Q4",
    },
    ModelSpec {
        name: "xtts-v2",
        hf_repo: "coqui/XTTS-v2",
        tier: Tier::Media,
        role: "voice-clone",
        current_format: Format::Pytorch,
        target_quant: Quant::Native,
        expected_gb: 1.9,
        keep_patterns: &["*.pth", "*.json", "vocab.*"],
        delete_patterns: &[],
        hf_download: true,
        notes: "Voice cloning EN+RU. Complex arch, keep PyTorch",
    },
    ModelSpec {
        name: "wan22-video",
        hf_repo: "QuantStack/Wan2.2-TI2V-5B-GGUF",
        tier: Tier::Media,
        role: "video-gen",
        current_format: Format::Gguf,
        target_quant: Quant::Q4,
        expected_gb: 4.5,
        keep_patterns: &["*.gguf", "**/*.safetensors"],
        delete_patterns: &[],
        hf_download: true,
        notes: "GGUF Q4_K_M + VAE safetensors. Both needed",
    },
];

// ── Disk inspection ──────────────────────────────────────────────

/// Status of a single model on disk
#[derive(Debug)]
pub struct ModelStatus {
    pub spec: &'static ModelSpec,
    pub exists: bool,
    pub size_bytes: u64,
    pub has_safetensors: bool,
    pub has_onnx: bool,
    pub has_gguf: bool,
    pub has_ggml: bool,
    pub has_pytorch: bool,
    pub missing_shards: Option<String>,
    pub junk_bytes: u64,
}

impl ModelStatus {
    /// Human-readable issues
    pub fn issues(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.exists {
            out.push("MISSING".into());
            return out;
        }
        if let Some(ref shard) = self.missing_shards {
            out.push(format!("INCOMPLETE: {shard}"));
        }
        // Check for duplicate formats
        let format_count = [
            self.has_safetensors,
            self.has_onnx,
            self.has_gguf,
            self.has_pytorch,
        ]
        .iter()
        .filter(|&&x| x)
        .count();
        if format_count > 1 && self.spec.target_quant != Quant::Native {
            let mut fmts = Vec::new();
            if self.has_safetensors {
                fmts.push("safetensors");
            }
            if self.has_onnx {
                fmts.push("onnx");
            }
            if self.has_gguf {
                fmts.push("gguf");
            }
            if self.has_pytorch {
                fmts.push("pytorch");
            }
            out.push(format!("DUPES: {}", fmts.join("+")));
        }
        // Check bloat
        let current_gb = self.size_bytes as f64 / 1e9;
        if self.spec.expected_gb > 0.0 && current_gb > self.spec.expected_gb as f64 * 2.0 {
            out.push(format!(
                "BLOATED ({:.1}x)",
                current_gb / self.spec.expected_gb as f64
            ));
        }
        if self.junk_bytes > 1_000_000 {
            out.push(format!("JUNK: {}M", self.junk_bytes / 1_000_000));
        }
        out
    }

    pub fn is_ok(&self) -> bool {
        self.exists && self.issues().is_empty()
    }
}

/// Inspect a single model directory
pub fn inspect_model(spec: &'static ModelSpec) -> ModelStatus {
    let dir = models_dir().join(spec.name);
    if !dir.exists() {
        return ModelStatus {
            spec,
            exists: false,
            size_bytes: 0,
            has_safetensors: false,
            has_onnx: false,
            has_gguf: false,
            has_ggml: false,
            has_pytorch: false,
            missing_shards: None,
            junk_bytes: 0,
        };
    }

    let mut size_bytes = 0u64;
    let mut has_safetensors = false;
    let mut has_onnx = false;
    let mut has_gguf = false;
    let mut has_ggml = false;
    let mut has_pytorch = false;
    let mut junk_bytes = 0u64;

    for entry in walkdir(&dir) {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let fsize = meta.len();
        size_bytes += fsize;

        let name = entry
            .file_name()
            .to_str()
            .unwrap_or("")
            .to_lowercase();
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "safetensors" if !name.contains("index") => has_safetensors = true,
            "onnx" => has_onnx = true,
            "gguf" => has_gguf = true,
            "bin" if name.starts_with("ggml") => has_ggml = true,
            "pt" | "pth" => has_pytorch = true,
            _ => {}
        }

        // Detect junk
        if is_junk(&entry) {
            junk_bytes += fsize;
        }
    }

    let missing_shards = check_shards(&dir);

    ModelStatus {
        spec,
        exists: true,
        size_bytes,
        has_safetensors,
        has_onnx,
        has_gguf,
        has_ggml,
        has_pytorch,
        missing_shards,
        junk_bytes,
    }
}

/// Inspect all models
pub fn inspect_all() -> Vec<ModelStatus> {
    MANIFEST.iter().map(|s| inspect_model(s)).collect()
}

/// Find directories in ~/llm/ not in the manifest
pub fn find_unknown() -> Vec<(String, u64)> {
    let base = models_dir();
    let known: std::collections::HashSet<&str> = MANIFEST.iter().map(|m| m.name).collect();
    let mut unknown = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if known.contains(name.as_str()) {
                continue;
            }
            let size = dir_size(&entry.path());
            if size > 1_000_000 {
                unknown.push((name, size));
            }
        }
    }
    unknown.sort_by(|a, b| b.1.cmp(&a.1));
    unknown
}

// ── Helpers ──────────────────────────────────────────────────────

fn dir_size(path: &Path) -> u64 {
    walkdir(path)
        .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

fn walkdir(path: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
    WalkDir::new(path).into_iter()
}

struct WalkDir {
    stack: Vec<std::fs::ReadDir>,
}

impl WalkDir {
    fn new(path: &Path) -> Self {
        let stack = match std::fs::read_dir(path) {
            Ok(rd) => vec![rd],
            Err(_) => vec![],
        };
        Self { stack }
    }

    fn into_iter(self) -> WalkDirIter {
        WalkDirIter { stack: self.stack }
    }
}

struct WalkDirIter {
    stack: Vec<std::fs::ReadDir>,
}

impl Iterator for WalkDirIter {
    type Item = std::fs::DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let rd = self.stack.last_mut()?;
            match rd.next() {
                Some(Ok(entry)) => {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        if let Ok(sub) = std::fs::read_dir(entry.path()) {
                            self.stack.push(sub);
                        }
                    }
                    return Some(entry);
                }
                Some(Err(_)) => continue,
                None => {
                    self.stack.pop();
                }
            }
        }
    }
}

fn is_junk(entry: &std::fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name = name.to_str().unwrap_or("");
    let path = entry.path();

    // Junk directories (checked via path components)
    for component in path.components() {
        let s = component.as_os_str().to_str().unwrap_or("");
        if matches!(s, ".huggingface" | ".cache" | "__pycache__" | ".git") {
            return true;
        }
    }

    // Junk files
    matches!(
        name,
        ".gitattributes" | "README.md" | "LICENSE" | "LICENSE.md" | "USE_POLICY.md" | "NOTICE"
    )
}

fn check_shards(dir: &Path) -> Option<String> {
    for entry in walkdir(dir) {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".safetensors.index.json"))
            .unwrap_or(false)
        {
            let data = match std::fs::read_to_string(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let json: serde_json::Value = match serde_json::from_str(&data) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if let Some(weight_map) = json.get("weight_map").and_then(|m| m.as_object()) {
                let shards: std::collections::HashSet<&str> =
                    weight_map.values().filter_map(|v| v.as_str()).collect();
                for shard_name in shards {
                    let shard_path = path.parent().unwrap_or(dir).join(shard_name);
                    if !shard_path.exists() {
                        return Some(format!("missing shard: {shard_name}"));
                    }
                }
            }
        }
    }
    None
}

/// Format bytes as human-readable
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1}G", bytes as f64 / 1e9)
    } else if bytes >= 1_000_000 {
        format!("{}M", bytes / 1_000_000)
    } else if bytes >= 1_000 {
        format!("{}K", bytes / 1_000)
    } else {
        format!("{}B", bytes)
    }
}
