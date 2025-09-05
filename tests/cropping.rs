#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 2)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UnsignedCrop {
  #[bits(9)]
  a: u16,
  #[bits(7)]
  b: u8,
}

#[test]
fn values_are_masked_on_write_and_read() {
  let a_mask: u16 = (1 << 9) - 1; // 0x01FF
  let b_mask: u8 = (1 << 7) - 1; // 0x7F
  let a_vals: &[u16] = &[0, 1, 0x01FF, 0x0200, 0x03FF, 0xFFFF];
  let b_vals: &[u8] = &[0, 1, 0x7F, 0x80, 0xFF];
  for &a in a_vals {
    for &b in b_vals {
      let s = UnsignedCrop { a, b };
      let bytes: [u8; 2] = s.into();
      let got: UnsignedCrop = bytes.into();
      assert_eq!(got.a, a & a_mask);
      assert_eq!(got.b, b & b_mask);
    }
  }
}
