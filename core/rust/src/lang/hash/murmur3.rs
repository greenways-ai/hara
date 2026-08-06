//! Bit-for-bit port of `hara.lang.base.primitive.Murmur3`
//! (the Clojure-variant 32-bit Murmur3).
//!
//! All arithmetic is wrapping `i32`, matching Java `int` semantics.
//! `hash_chars` operates on UTF-16 code units (Java `char`s), packing
//! two units per 32-bit block exactly like the Java implementation.

pub const C1: i32 = 0xcc9e2d51u32 as i32;
pub const C2: i32 = 0x1b873593;

pub const SEED: i32 = 0;

#[inline]
pub fn mix_k1(k1: i32) -> i32 {
    k1.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2)
}

#[inline]
pub fn mix_h1(h1: i32, k1: i32) -> i32 {
    (h1 ^ k1)
        .rotate_left(13)
        .wrapping_mul(5)
        .wrapping_add(0xe6546b64u32 as i32)
}

#[inline]
pub fn mix_full(h1: i32, length: i32) -> i32 {
    let mut h1 = h1 ^ length;
    h1 ^= (h1 as u32 >> 16) as i32;
    h1 = h1.wrapping_mul(0x85ebca6bu32 as i32);
    h1 ^= (h1 as u32 >> 13) as i32;
    h1 = h1.wrapping_mul(0xc2b2ae35u32 as i32);
    h1 ^= (h1 as u32 >> 16) as i32;
    h1
}

/// `Murmur3.hashChars` over a slice of UTF-16 code units.
pub fn hash_chars_units(units: &[u16]) -> i32 {
    let mut h1 = SEED;

    // step through 2 chars at a time
    let mut i = 1usize;
    while i < units.len() {
        let k1 = (units[i - 1] as i32) | ((units[i] as i32) << 16);
        h1 = mix_h1(h1, mix_k1(k1));
        i += 2;
    }

    // remaining odd char
    if units.len() & 1 == 1 {
        let k1 = mix_k1(units[units.len() - 1] as i32);
        h1 ^= k1;
    }

    mix_full(h1, (units.len() as i32).wrapping_mul(2))
}

/// `Murmur3.hashChars` over a Rust string's UTF-16 encoding.
pub fn hash_chars(s: &str) -> i32 {
    let units: Vec<u16> = s.encode_utf16().collect();
    hash_chars_units(&units)
}

/// `Murmur3.hashInt`.
pub fn hash_int(input: i32) -> i32 {
    if input == 0 {
        return 0;
    }
    mix_full(mix_h1(SEED, mix_k1(input)), 4)
}

/// `Murmur3.hashLong`.
pub fn hash_long(input: i64) -> i32 {
    if input == 0 {
        return 0;
    }
    let low = input as i32;
    let high = ((input as u64) >> 32) as i32;

    let mut h1 = mix_h1(SEED, mix_k1(low));
    h1 = mix_h1(h1, mix_k1(high));

    mix_full(h1, 8)
}

/// `Murmur3.mixHash`.
pub fn mix_hash(hash: i32, count: i32) -> i32 {
    let h1 = mix_h1(SEED, mix_k1(hash));
    mix_full(h1, count)
}
