#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AlignedI64 {
  #[bits(64)]
  x: i64,
}

#[test]
fn aligned_i64_endianness_and_sign() {
  let cases = [0i64, -1, 1, i64::MIN, i64::MAX, -0x0123_4567_89AB_CDEF];
  for &x in &cases {
    let arr: [u8; 8] = AlignedI64 { x }.into();
    assert_eq!(arr, x.to_le_bytes());
    let back: AlignedI64 = arr.into();
    assert_eq!(back.x, x);
  }
}

#[packbits::pack(bytes = 6)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NarrowI64 {
  #[bits(40)]
  a: i64, // 40-bit signed, stored unaligned
  #[bits(8)]
  b: u8,
}

#[test]
fn narrow_i64_unaligned_sign_extension() {
  let cases = [-(1i64 << 39), -1, 0, 1, (1i64 << 39) - 1];
  for &a in &cases {
    let s = NarrowI64 { a, b: 0xAA };
    let bytes: [u8; 6] = s.into();
    let got: NarrowI64 = bytes.into();
    assert_eq!(got, s, "a={}, bytes={:02x?}", a, bytes);
  }
}
