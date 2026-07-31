//! Bit-for-bit port of `hara.lang.base.primitive.SipHash` (SipHash-2-4).
//!
//! All arithmetic is unsigned 64-bit wrapping, matching Java `long`
//! bit semantics. The fixed HARA key is reproduced from the Java source.

/// Default C compression rounds.
pub const DEFAULT_C: usize = 2;
/// Default D compression rounds.
pub const DEFAULT_D: usize = 4;

/// The fixed HARA key (byte values from the Java source, sign-extended there).
pub const HARA: [u8; 16] = [
    55, 89, 144, 233, 121, 98, 219, 61, 24, 85, 109, 194, 47, 241, 32, 17,
];

const INITIAL_V0: u64 = 0x736f6d6570736575;
const INITIAL_V1: u64 = 0x646f72616e646f6d;
const INITIAL_V2: u64 = 0x6c7967656e657261;
const INITIAL_V3: u64 = 0x7465646279746573;

#[inline]
fn bytes_to_long(bytes: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(buf)
}

#[inline]
fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v2 = v2.wrapping_add(*v3);
    *v1 = v1.rotate_left(13);
    *v3 = v3.rotate_left(16);

    *v1 ^= *v0;
    *v3 ^= *v2;
    *v0 = v0.rotate_left(32);

    *v2 = v2.wrapping_add(*v1);
    *v0 = v0.wrapping_add(*v3);
    *v1 = v1.rotate_left(17);
    *v3 = v3.rotate_left(21);

    *v1 ^= *v2;
    *v3 ^= *v0;
    *v2 = v2.rotate_left(32);
}

/// `SipHash.hash(key, data)` with the default 2-4 rounds.
pub fn hash(key: &[u8; 16], data: &[u8]) -> u64 {
    hash_with_rounds(key, data, DEFAULT_C, DEFAULT_D)
}

/// `SipHash.hash(key, data, c, d)`.
pub fn hash_with_rounds(key: &[u8; 16], data: &[u8], c: usize, d: usize) -> u64 {
    let k0 = bytes_to_long(key, 0);
    let k1 = bytes_to_long(key, 8);

    let mut v0 = INITIAL_V0 ^ k0;
    let mut v1 = INITIAL_V1 ^ k1;
    let mut v2 = INITIAL_V2 ^ k0;
    let mut v3 = INITIAL_V3 ^ k1;

    let last = data.len() / 8 * 8;
    let mut i = 0usize;
    while i < last {
        let m = bytes_to_long(data, i);
        i += 8;

        v3 ^= m;
        for _ in 0..c {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
    }

    let mut m: u64 = 0;
    for (j, b) in data[last..].iter().enumerate() {
        m |= (*b as u64) << (8 * j);
    }
    m |= (data.len() as u64) << 56;

    v3 ^= m;
    for _ in 0..c {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= m;

    v2 ^= 0xff;
    for _ in 0..d {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// SipHash of a UTF-8 string's bytes with the fixed HARA key.
pub fn hash_str(s: &str) -> u64 {
    hash(&HARA, s.as_bytes())
}
