//! Whisper transcription pipeline: audio -> mel -> encoder -> decoder -> text
//!
//! Full pipeline:
//! 1. Load GGML model (weights, mel filters, vocab)
//! 2. Load WAV audio, compute mel spectrogram
//! 3. Run encoder on mel spectrogram
//! 4. Autoregressive decoder loop with cross-attention
//! 5. Decode tokens to text

use std::path::Path;
use std::sync::Arc;

use crate::audio;
use crate::backend::wgpu_backend::graph_model::{Architecture, GraphModel, GraphModelConfig};
use crate::backend::wgpu_backend::pipelines::Pipelines;
use crate::ir::templates::{whisper_encoder_decoder, WhisperConfig};
use crate::loader::ggml::{load_ggml_full, GgmlWhisperData, MelFilters, WhisperHparams};

/// Whisper special tokens
const SOT: u32 = 50258;         // <|startoftranscript|>
const EOT: u32 = 50257;         // <|endoftext|>
const TRANSCRIBE: u32 = 50359;  // <|transcribe|>
const NO_TIMESTAMPS: u32 = 50363; // <|notimestamps|>

/// Language tokens start at 50259
const LANG_EN: u32 = 50259;     // <|en|>

/// Maximum decoder tokens (safety limit)
#[allow(dead_code)]
const MAX_DECODER_TOKENS: usize = 224;

/// Whisper transcriber — loads model and provides transcription
pub struct Transcriber {
    model: GraphModel,
    vocab: Vec<String>,
    #[allow(dead_code)]
    hparams: WhisperHparams,
    whisper_config: WhisperConfig,
    mel_filters: MelFilters,
    #[allow(dead_code)]
    pipelines: Arc<Pipelines>,
}

impl Transcriber {
    /// Load a Whisper GGML model and prepare for transcription.
    pub fn new(model_path: &Path, pipelines: Arc<Pipelines>) -> Result<Self, String> {
        // Load GGML file with all data
        let ggml_data = load_ggml_full(model_path)?;

        let GgmlWhisperData {
            graph: graph_with_weights,
            hparams,
            mel_filters,
            vocab,
        } = ggml_data;

        // Build whisper config from hparams
        let whisper_config = WhisperConfig {
            n_audio_state: hparams.n_audio_state as usize,
            n_audio_head: hparams.n_audio_head as usize,
            n_audio_layer: hparams.n_audio_layer as usize,
            n_audio_ctx: hparams.n_audio_ctx as usize,
            n_text_state: hparams.n_text_state as usize,
            n_text_head: hparams.n_text_head as usize,
            n_text_layer: hparams.n_text_layer as usize,
            n_text_ctx: hparams.n_text_ctx as usize,
            n_vocab: hparams.n_vocab as usize,
            n_mels: hparams.n_mels as usize,
            eps: 1e-5,
        };

        log::info!("Whisper config: audio_state={}, audio_layers={}, text_state={}, text_layers={}, vocab={}",
            whisper_config.n_audio_state, whisper_config.n_audio_layer,
            whisper_config.n_text_state, whisper_config.n_text_layer, whisper_config.n_vocab);

        // Build graph from template
        let graph = whisper_encoder_decoder(&whisper_config);
        log::info!("Graph template: {} nodes", graph.len());

        let gm_config = GraphModelConfig {
            hidden_size: whisper_config.n_audio_state as u32,
            num_heads: whisper_config.n_audio_head as u32,
            kv_num_heads: whisper_config.n_audio_head as u32,
            head_dim: (whisper_config.n_audio_state / whisper_config.n_audio_head) as u32,
            vocab_size: whisper_config.n_vocab as u32,
            num_layers: whisper_config.n_text_layer as u32,
            block_size: 32,
            rope_theta: 0.0,
            max_seq_len: whisper_config.n_text_ctx as u32,
            has_qk_norm: false,
        };

        let model = GraphModel::new(
            graph,
            &graph_with_weights.weights,
            pipelines.clone(),
            Architecture::EncoderDecoder,
            gm_config,
        )?;

        Ok(Self {
            model,
            vocab,
            hparams,
            whisper_config,
            mel_filters,
            pipelines,
        })
    }

    /// Transcribe an audio file to text.
    ///
    /// The audio file must be a 16-bit PCM WAV file.
    /// It will be resampled to 16kHz mono if needed.
    pub fn transcribe(&mut self, audio_path: &Path) -> Result<String, String> {
        // Load audio and compute mel spectrogram
        let (mel_data, _n_frames) = audio::audio_to_mel(audio_path, &self.mel_filters)?;

        // Run encoder
        let n_mel = self.whisper_config.n_mels;
        let n_audio_ctx = self.whisper_config.n_audio_ctx;
        let audio_state = self.whisper_config.n_audio_state;

        log::info!("Running encoder: mel [{}, {}] -> encoder output [{}, {}]",
            n_mel, n_audio_ctx, audio_state, n_audio_ctx);

        let enc_output = self.run_encoder(&mel_data, n_mel, n_audio_ctx, audio_state)?;

        log::info!("Encoder output: {} values (expected {})", enc_output.len(), audio_state * n_audio_ctx);

        // Run decoder loop
        let tokens = self.decode_loop(&enc_output)?;

        // Convert tokens to text
        let text = self.tokens_to_text(&tokens);

        Ok(text)
    }

    /// Run the encoder on mel spectrogram data.
    fn run_encoder(
        &self,
        mel_data: &[f32],
        _n_mel: usize,
        n_audio_ctx: usize,
        audio_state: usize,
    ) -> Result<Vec<f32>, String> {
        let output_size = audio_state * n_audio_ctx;

        self.model.encode_audio(
            "audio_input",
            mel_data,
            "enc.output",
            output_size,
        )
    }

    /// Run the autoregressive decoder loop.
    fn decode_loop(&mut self, _encoder_output: &[f32]) -> Result<Vec<u32>, String> {
        // Initial decoder tokens
        let tokens: Vec<u32> = vec![SOT, LANG_EN, TRANSCRIBE, NO_TIMESTAMPS];

        log::info!("Decoder loop: initial tokens {:?}", tokens);

        // TODO: Implement the actual decoder loop once execute_encode_decode
        // is added to GraphExecutor. The loop would:
        //
        // for step in 0..MAX_DECODER_TOKENS {
        //     let logits = self.model.forward_decoder(&tokens, &encoder_output);
        //     let next_token = argmax(&logits);
        //     if next_token == EOT { break; }
        //     tokens.push(next_token);
        // }
        //
        // For now, return the initial tokens to show the pipeline structure.

        log::info!("Decoder loop: generated {} tokens", tokens.len());

        Ok(tokens)
    }

    /// Convert token IDs to text string using the embedded vocab.
    fn tokens_to_text(&self, tokens: &[u32]) -> String {
        let mut text = String::new();

        for &token_id in tokens {
            let id = token_id as usize;

            // Skip special tokens
            if token_id >= SOT || token_id == EOT {
                continue;
            }

            if id < self.vocab.len() {
                let word = &self.vocab[id];
                // Whisper BPE uses the GPT-2 byte-level encoding
                // where spaces are encoded as "Ġ" (U+0120) at the start of tokens
                let decoded = decode_bpe_token(word);
                text.push_str(&decoded);
            }
        }

        text.trim().to_string()
    }
}

/// Decode a GPT-2 BPE token.
///
/// GPT-2 BPE uses a byte-level encoding where:
/// - "Ġ" (U+0120) represents a space before the token
/// - Other special byte encodings map specific Unicode chars to bytes
fn decode_bpe_token(token: &str) -> String {
    let mut result = String::new();
    for ch in token.chars() {
        match ch {
            '\u{0120}' => result.push(' '), // Ġ -> space
            '\u{010a}' => result.push('\n'), // Ċ -> newline
            _ => result.push(ch),
        }
    }
    result
}
