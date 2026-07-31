//! Bit-for-bit port of `hara.lang.base.primitive.RapidHash` (rapidhash).
//!
//! All arithmetic is unsigned 64-bit wrapping, matching Java's `long`
//! (Java signed overflow has identical bit semantics). `rapid_mix`
//! reproduces `A * B` (low 64) XOR `Math.unsignedMultiplyHigh(A, B)`
//! via a 128-bit product.

pub const RAPID_SEED: u64 = 0xbdd89aa982704029;

pub const RAPID_SECRET_0: u64 = 0x2d358dccaa6c78a5;
pub const RAPID_SECRET_1: u64 = 0x8bb84b93962eacc9;
pub const RAPID_SECRET_2: u64 = 0x4b33a62ed433d4a3;

#[inline]
pub fn rapid_mix(a: u64, b: u64) -> u64 {
    let product = (a as u128).wrapping_mul(b as u128);
    (product as u64) ^ ((product >> 64) as u64)
}

#[inline]
fn read64(data: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(buf)
}

#[inline]
fn read32(data: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&data[offset..offset + 4]);
    u32::from_le_bytes(buf) as u64
}

#[inline]
fn read_small(data: &[u8], offset: usize, k: usize) -> u64 {
    ((data[offset] as u64) << 56)
        | ((data[offset + (k >> 1)] as u64) << 32)
        | (data[offset + k - 1] as u64)
}

/// RapidHash of a UTF-8 string's bytes (matches `RapidHash.hash(String)`).
pub fn hash_str(s: &str) -> u64 {
    hash(s.as_bytes())
}

/// RapidHash with the default seed (matches `RapidHash.hash(byte[])`).
pub fn hash(data: &[u8]) -> u64 {
    hash_with_seed(data, RAPID_SEED)
}

/// RapidHash with an explicit seed (matches `RapidHash.hash(byte[], long)`).
pub fn hash_with_seed(data: &[u8], seed: u64) -> u64 {
    let len = data.len();
    let mut seed = seed ^ (rapid_mix(seed ^ RAPID_SECRET_0, RAPID_SECRET_1) ^ (len as u64));
    let a: u64;
    let b: u64;

    if len <= 16 {
        if len >= 4 {
            let plast = len - 4;
            a = (read32(data, 0) << 32) | read32(data, plast);
            // delta = (len & 24) >> (len >> 3)
            let delta = (len & 24) >> (len >> 3);
            b = (read32(data, delta) << 32) | read32(data, plast - delta);
        } else if len > 0 {
            a = read_small(data, 0, len);
            b = 0;
        } else {
            a = 0;
            b = 0;
        }
    } else {
        let mut i = len;
        let mut p = 0usize;
        if i > 48 {
            let mut see1 = seed;
            let mut see2 = seed;
            loop {
                seed = rapid_mix(read64(data, p) ^ RAPID_SECRET_0, read64(data, p + 8) ^ seed);
                see1 = rapid_mix(read64(data, p + 16) ^ RAPID_SECRET_1, read64(data, p + 24) ^ see1);
                see2 = rapid_mix(read64(data, p + 32) ^ RAPID_SECRET_2, read64(data, p + 40) ^ see2);
                p += 48;
                i -= 48;
                if i < 48 {
                    break;
                }
            }
            seed ^= see1 ^ see2;
        }
        if i > 16 {
            seed = rapid_mix(
                read64(data, p) ^ RAPID_SECRET_2,
                read64(data, p + 8) ^ seed ^ RAPID_SECRET_1,
            );
            if i > 32 {
                seed = rapid_mix(read64(data, p + 16) ^ RAPID_SECRET_2, read64(data, p + 24) ^ seed);
            }
        }
        a = read64(data, p + i - 16);
        b = read64(data, p + i - 8);
    }

    let a = a ^ RAPID_SECRET_1;
    let b = b ^ seed;

    let product = (a as u128).wrapping_mul(b as u128);
    let low = product as u64;
    let high = (product >> 64) as u64;

    rapid_mix(low ^ RAPID_SECRET_0 ^ (len as u64), high ^ RAPID_SECRET_1)
}
