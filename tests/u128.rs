#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AlignedU128 {
  #[bits(128)]
  x: u128,
}

#[test]
fn aligned_u128_endianness() {
  let cases: [u128; 5] = [
    0u128,
    1u128,
    0x00FF_0000_0000_0000_0000_0000_0000_0000u128,
    0xDEAD_BEEF_F00D_BAAD_FEED_FACE_BEEF_1234u128,
    u128::MAX,
  ];
  for &x in &cases {
    let arr: [u8; 16] = AlignedU128 { x }.into();
    assert_eq!(arr, x.to_le_bytes());
    let back: AlignedU128 = arr.into();
    assert_eq!(back.x, x);
  }
}

#[packbits::pack(bytes = 17)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mixed128 {
  #[bits(9)]
  a: u16,
  #[bits(120)]
  b: u128,
}

#[test]
fn mixed_u128_unaligned() {
  // Use a value that fits within 120 bits (top 8 bits zero)
  let s = Mixed128 { a: 0x1AB, b: 0x0023_4567_89AB_CDEF_FEDC_BA98_7654_3210u128 };
  let arr: [u8; 17] = s.into();
  let back: Mixed128 = arr.into();
  assert_eq!(back, s);
}
