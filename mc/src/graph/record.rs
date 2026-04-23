//! Cyberlink record: fixed 128-byte layout per cyb-graph spec §cyberlinks.
//!
//! ```text
//! [0..32]    ν   neuron id (hemera hash, 32 B)
//! [32..64]   p   source particle id (hemera hash, 32 B)
//! [64..96]   q   target particle id (hemera hash, 32 B)
//! [96..100]  τ   token denomination index (u32 little-endian)
//! [100..116] a   stake amount (u128 little-endian, smallest unit)
//! [116..117] v   valence (i8: -1, 0, +1)
//! [117..125] t   block height (u64 little-endian)
//! [125..128] _   padding (zero)
//! ```

pub const RECORD_SIZE: usize = 128;

#[derive(Debug, Clone, Copy)]
pub struct Cyberlink {
    pub neuron: [u8; 32],
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub token: u32,
    pub amount: u128,
    pub valence: i8,
    pub block: u64,
}

impl Cyberlink {
    pub fn decode(bytes: &[u8; RECORD_SIZE]) -> Self {
        let mut neuron = [0u8; 32];
        neuron.copy_from_slice(&bytes[0..32]);
        let mut from = [0u8; 32];
        from.copy_from_slice(&bytes[32..64]);
        let mut to = [0u8; 32];
        to.copy_from_slice(&bytes[64..96]);
        let token = u32::from_le_bytes(bytes[96..100].try_into().unwrap());
        let amount = u128::from_le_bytes(bytes[100..116].try_into().unwrap());
        let valence = bytes[116] as i8;
        let block = u64::from_le_bytes(bytes[117..125].try_into().unwrap());
        Self { neuron, from, to, token, amount, valence, block }
    }
}

/// Iterate fixed-size records from the mmap'd `cyberlinks` section.
pub struct CyberlinkIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> CyberlinkIter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
}

impl<'a> Iterator for CyberlinkIter<'a> {
    type Item = Cyberlink;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + RECORD_SIZE > self.bytes.len() {
            return None;
        }
        let chunk: &[u8; RECORD_SIZE] = self.bytes[self.offset..self.offset + RECORD_SIZE]
            .try_into()
            .unwrap();
        self.offset += RECORD_SIZE;
        Some(Cyberlink::decode(chunk))
    }
}

impl<'a> ExactSizeIterator for CyberlinkIter<'a> {
    fn len(&self) -> usize {
        (self.bytes.len() - self.offset) / RECORD_SIZE
    }
}
