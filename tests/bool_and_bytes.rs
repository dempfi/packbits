#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 1)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flags {
    // Test inferred width for bool
    a: bool,
    #[bits(3)]
    b: u8,
    #[bits(4)]
    c: u8,
}

#[test]
fn roundtrip_flags_from_u8_and_back() {
    // For all possible u8 values, ensure roundtrip via From/Into exists
    for v in 0u16..=255 {
        let v8 = v as u8;
        let f: Flags = <Flags as From<u8>>::from(v8);
        let back: u8 = <u8 as From<Flags>>::from(f);
        assert_eq!(back, v8);
    }
}

#[packbits::pack]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DefaultBytesOne {
    #[bits(3)]
    lo: u8,
    #[bits(5)]
    hi: u8,
}

#[test]
fn default_bytes_is_one() {
    // No bytes specified → defaults to 1
    for lo in 0u8..=7 {
        for hi in 0u8..=31 {
            let s = DefaultBytesOne { lo, hi };
            let b: u8 = <u8 as From<DefaultBytesOne>>::from(s);
            let s2: DefaultBytesOne = <DefaultBytesOne as From<u8>>::from(b);
            assert_eq!(s2, DefaultBytesOne { lo, hi });
        }
    }
}
