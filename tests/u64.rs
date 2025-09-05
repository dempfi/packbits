#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AlignedU64 {
  #[bits(64)]
  x: u64,
}

#[test]
fn aligned_u64_endianness() {
  let cases = [0u64, 1, 0x00FF_0000_0000_0000, 0xDEAD_BEEF_F00D_BAAD, u64::MAX];
  for &x in &cases {
    let arr: [u8; 8] = AlignedU64 { x }.into();
    assert_eq!(arr, x.to_le_bytes());
    let back: AlignedU64 = arr.into();
    assert_eq!(back.x, x);
  }
}
