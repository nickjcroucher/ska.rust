//! ntHash is a hash function optimised for a DNA alphabet `{A, C, T, G}`.
//!
//! It works particularly well as a rolling hash, e.g. for k-mers in an input
//! sequence.
//!
//! This implementation based on ntHash [1.0.4](https://github.com/bcgsc/ntHash/releases/tag/v1.0.4).
//! Also with thanks to [@luizirber's](https://github.com/luizirber) ntHash [crate](https://crates.io/crates/nthash).
//!

use super::bit_encoding::encode_base;

const HASH_LOOKUP: [u64; 4] = [
    0x3c8b_fbb3_95c6_0474,
    0x3193_c185_62a0_2b4c,
    0x2955_49f5_4be2_4456,
    0x2032_3ed0_8257_2324,
];
const RC_HASH_LOOKUP: [u64; 4] = [
    0x2955_49f5_4be2_4456,
    0x2032_3ed0_8257_2324,
    0x3c8b_fbb3_95c6_0474,
    0x3193_c185_62a0_2b4c,
];
const MULTISHIFT: i32 = 27;
const MULTISEED: u64 = 0x90b4_5d39_fb6d_a1fa;

/// Stores forward and (optionally) reverse complement hashes of k-mers in a nucleotide sequence
#[derive(Debug)]
pub struct NtHashIterator {
    k: usize,
    fh: u64,
    rh: Option<u64>,
}

impl NtHashIterator {
    /// Creates a new iterator over a sequence with a given k-mer size
    pub fn new(seq: &[u8], k: usize, rc: bool) -> NtHashIterator {
        let mut fh = 0;
        for (i, v) in seq[0..k].iter().enumerate() {
            fh ^= HASH_LOOKUP[encode_base(*v) as usize].rotate_left((k - i - 1) as u32);
        }

        let rh = if rc {
            let mut h = 0;
            for (i, v) in seq[0..k].iter().rev().enumerate() {
                h ^= RC_HASH_LOOKUP[encode_base(*v) as usize].rotate_left((k - i - 1) as u32);
            }
            Some(h)
        } else {
            None
        };

        Self { k, fh, rh }
    }

    /// Move to the next k-mer by adding a new base, removing a base from the end, efficiently updating the hash.
    pub fn roll_fwd(&mut self, old_base: u8, new_base: u8) {
        self.fh = self.fh.rotate_left(1)
            ^ HASH_LOOKUP[old_base as usize].rotate_left(self.k as u32)
            ^ HASH_LOOKUP[new_base as usize];

        if let Some(rev) = self.rh {
            self.rh = Some(
                rev.rotate_right(1)
                    ^ RC_HASH_LOOKUP[old_base as usize].rotate_right(1)
                    ^ RC_HASH_LOOKUP[new_base as usize].rotate_left(self.k as u32 - 1),
            )
        };
    }

    /// Retrieve the current hash (minimum of forward and reverse complement hashes)
    pub fn curr_hash(&self) -> u64 {
        if let Some(rev) = self.rh {
            u64::min(self.fh, rev)
        } else {
            self.fh
        }
    }

    /// Generates more hashes of the sequence.
    ///
    /// This only operates on the hash value, not the full sequence, so won't resolve
    /// a full hash collision, but is suitable for use in the countmin table where
    /// most of the hash is masked off.
    pub fn extra_hash(&self, extra_idx: usize) -> u64 {
        let mut new_hash = self
            .curr_hash()
            .wrapping_mul(MULTISEED.wrapping_mul((extra_idx ^ self.k) as u64));
        new_hash ^= new_hash >> MULTISHIFT;
        new_hash
    }
}
