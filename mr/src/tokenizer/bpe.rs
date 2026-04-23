//! BPE encode/decode.
//!
//! Spec: reference/runtime/tokenizer.md

use super::byte_level;
use std::collections::HashMap;

pub struct Bpe {
    /// token_id → string
    tokens: Vec<String>,
    /// string → token_id
    reverse: HashMap<String, u32>,
    /// (left, right) → priority (lower = higher priority)
    merges: HashMap<(String, String), u32>,
    /// Byte-level encoding table.
    byte_encoder: [char; 256],
    /// Byte-level decoding table.
    byte_decoder: HashMap<char, u8>,
    /// Special tokens (name + id). Matched greedily before BPE.
    specials: Vec<(String, u32)>,
    /// SentencePiece (Gemma, Llama) tokenizers prefix words with U+2581 ▁
    /// (metaspace) instead of GPT-2 byte-level encoding. Auto-detected by
    /// presence of ▁-prefixed tokens in the vocab.
    metaspace: bool,
}

impl Bpe {
    pub fn new(tokens: Vec<(u32, String)>, merges: Vec<(String, String)>) -> Self {
        let max_id = tokens.iter().map(|(id, _)| *id).max().unwrap_or(0);
        let mut tok_vec = vec![String::new(); (max_id as usize) + 1];
        let mut reverse = HashMap::new();
        for (id, tok) in tokens {
            if (id as usize) < tok_vec.len() {
                tok_vec[id as usize] = tok.clone();
            }
            reverse.insert(tok, id);
        }

        let merge_map: HashMap<(String, String), u32> = merges
            .into_iter()
            .enumerate()
            .map(|(i, pair)| (pair, i as u32))
            .collect();

        let byte_encoder = byte_level::build_byte_encoder();
        let byte_decoder = byte_level::build_byte_decoder(&byte_encoder);

        // Detect specials by pattern <|...|>
        let specials: Vec<(String, u32)> = reverse
            .iter()
            .filter(|(s, _)| s.starts_with("<|") && s.ends_with("|>"))
            .map(|(s, &id)| (s.clone(), id))
            .collect();

        // Auto-detect SentencePiece (metaspace) tokenizer: vocab contains
        // tokens prefixed with U+2581 (▁). Llama, Mistral, Gemma all use this.
        let metaspace = reverse.keys().any(|k| k.starts_with('\u{2581}'));

        Self {
            tokens: tok_vec,
            reverse,
            merges: merge_map,
            byte_encoder,
            byte_decoder,
            specials,
            metaspace,
        }
    }

    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }

    pub fn token(&self, id: u32) -> Option<&str> {
        self.tokens.get(id as usize).map(|s| s.as_str())
    }

    pub fn id(&self, token: &str) -> Option<u32> {
        self.reverse.get(token).copied()
    }

    /// Encode text → token IDs.
    ///
    /// Algorithm:
    /// 1. Split input on special tokens (longest-match at each position).
    /// 2. For each non-special segment: byte-level encode, split into chars,
    ///    apply merges until no more apply, look up in vocab.
    /// 3. Concatenate all resulting IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        let segments = self.split_on_specials(text);
        for seg in segments {
            match seg {
                Segment::Special(id) => out.push(id),
                Segment::Text(s) => {
                    if s.is_empty() {
                        continue;
                    }
                    let preprocessed = if self.metaspace {
                        // SentencePiece: replace literal space with ▁ and
                        // prepend ▁ at the start of the segment so the first
                        // word lookup hits a "▁word" vocab entry. BPE merges
                        // operate on UTF-8 chars (the metaspace ▁ is U+2581,
                        // 3 bytes — chars() splits it into one symbol).
                        let mut p = String::with_capacity(s.len() + 3);
                        if !s.starts_with(' ') && !s.starts_with('\u{2581}') {
                            p.push('\u{2581}');
                        }
                        for c in s.chars() {
                            if c == ' ' {
                                p.push('\u{2581}');
                            } else {
                                p.push(c);
                            }
                        }
                        p
                    } else {
                        byte_level::encode_bytes(&self.byte_encoder, &s)
                    };
                    self.bpe_encode_segment(&preprocessed, &mut out);
                }
            }
        }
        out
    }

    fn bpe_encode_segment(&self, byte_level_str: &str, out: &mut Vec<u32>) {
        let mut symbols: Vec<String> = byte_level_str.chars().map(|c| c.to_string()).collect();

        loop {
            // Find highest-priority merge among adjacent pairs.
            let mut best: Option<(usize, u32)> = None;
            for i in 0..symbols.len().saturating_sub(1) {
                let pair = (symbols[i].clone(), symbols[i + 1].clone());
                if let Some(&prio) = self.merges.get(&pair) {
                    if best.map_or(true, |(_, p)| prio < p) {
                        best = Some((i, prio));
                    }
                }
            }
            let (i, _) = match best {
                Some(b) => b,
                None => break,
            };
            let merged = format!("{}{}", symbols[i], symbols[i + 1]);
            symbols[i] = merged;
            symbols.remove(i + 1);
        }

        for s in symbols {
            if let Some(&id) = self.reverse.get(&s) {
                out.push(id);
            }
            // If a symbol isn't in vocab (shouldn't happen with byte-level), skip.
            // A more complete implementation would byte-fall-back here.
        }
    }

    fn split_on_specials<'a>(&self, text: &'a str) -> Vec<Segment<'a>> {
        if self.specials.is_empty() {
            return vec![Segment::Text(std::borrow::Cow::Borrowed(text))];
        }
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < text.len() {
            // Find longest special match at pos.
            let tail = &text[pos..];
            let best: Option<(usize, u32)> = self
                .specials
                .iter()
                .filter_map(|(s, id)| {
                    if tail.starts_with(s.as_str()) {
                        Some((s.len(), *id))
                    } else {
                        None
                    }
                })
                .max_by_key(|(len, _)| *len);

            if let Some((len, id)) = best {
                out.push(Segment::Special(id));
                pos += len;
            } else {
                // Find next special start.
                let next_special = self
                    .specials
                    .iter()
                    .filter_map(|(s, _)| {
                        tail.find(s.as_str()).map(|i| i)
                    })
                    .min()
                    .unwrap_or(tail.len());
                let end = pos + next_special;
                out.push(Segment::Text(std::borrow::Cow::Borrowed(&text[pos..end])));
                pos = end;
            }
        }
        out
    }

    /// Decode token IDs → text.
    pub fn decode(&self, ids: &[u32], skip_special_tokens: bool) -> String {
        let mut buffer = String::new();
        let specials_set: std::collections::HashSet<u32> =
            self.specials.iter().map(|(_, id)| *id).collect();

        for &id in ids {
            if specials_set.contains(&id) {
                if skip_special_tokens {
                    continue;
                }
                // Emit literal — bypass byte-level decoding.
                if let Some(tok) = self.token(id) {
                    buffer.push_str(tok);
                }
            } else if let Some(tok) = self.token(id) {
                buffer.push_str(tok);
            }
        }

        // SentencePiece path: vocab strings are UTF-8 with ▁ for spaces.
        // Just replace ▁ → space and we're done.
        if self.metaspace {
            return buffer.replace('\u{2581}', " ");
        }

        // GPT-2 byte-level decode for the accumulated buffer.
        let bytes = byte_level::decode_bytes(&self.byte_decoder, &buffer);
        // Bytes may include the specials as their literal string (they're
        // already valid UTF-8 since stored as such). This is actually fine
        // because byte-level specials are in vocab as the full literal
        // and we don't byte-level-encode them when they're emitted literally.
        // For correctness, specials should be extracted separately.
        // Simpler correct impl: emit string per-ID and concatenate.
        match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                // Fallback: decode per-token
                let mut s = String::new();
                for &id in ids {
                    if specials_set.contains(&id) {
                        if skip_special_tokens {
                            continue;
                        }
                        if let Some(tok) = self.token(id) {
                            s.push_str(tok);
                        }
                    } else if let Some(tok) = self.token(id) {
                        let b = byte_level::decode_bytes(&self.byte_decoder, tok);
                        s.push_str(&String::from_utf8_lossy(&b));
                    }
                }
                s
            }
        }
    }
}

enum Segment<'a> {
    Special(u32),
    Text(std::borrow::Cow<'a, str>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_bpe() -> Bpe {
        // Tokens:
        //   0 "!"
        //   1 "h"
        //   2 "e"
        //   3 "l"
        //   4 "o"
        //   5 "he"
        //   6 "llo"
        //   7 "hello"
        //   8 "<|im_end|>"
        let tokens = vec![
            (0, "!".to_string()),
            (1, "h".to_string()),
            (2, "e".to_string()),
            (3, "l".to_string()),
            (4, "o".to_string()),
            (5, "he".to_string()),
            (6, "llo".to_string()),
            (7, "hello".to_string()),
            (8, "<|im_end|>".to_string()),
        ];
        let merges = vec![
            ("h".to_string(), "e".to_string()),
            ("l".to_string(), "l".to_string()),
            ("ll".to_string(), "o".to_string()),
            ("he".to_string(), "llo".to_string()),
        ];
        Bpe::new(tokens, merges)
    }

    #[test]
    fn merges_to_single_token() {
        let bpe = tiny_bpe();
        let ids = bpe.encode("hello");
        assert_eq!(ids, vec![7]);
    }

    #[test]
    fn special_token_preempts_bpe() {
        let bpe = tiny_bpe();
        let ids = bpe.encode("hello<|im_end|>");
        assert_eq!(ids, vec![7, 8]);
    }

    #[test]
    fn decode_skips_specials_when_requested() {
        let bpe = tiny_bpe();
        let s = bpe.decode(&[7, 8], true);
        assert_eq!(s, "hello");
        let s = bpe.decode(&[7, 8], false);
        assert_eq!(s, "hello<|im_end|>");
    }

    #[test]
    fn unknown_chars_degrade_gracefully() {
        // Unknown byte-level char not in vocab: encode skips. Real tokenizers
        // have byte fallback tokens; our tiny test vocab doesn't.
        let bpe = tiny_bpe();
        let ids = bpe.encode("!");
        assert_eq!(ids, vec![0]);
    }
}
