#![allow(dead_code)]

use packbits as _;

// Default (lsb) layout: bit 0 is least-significant within each byte
#[packbits::pack(bytes = 1)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LsbByte {
  #[bits(1)]
  b0: u8, // occupies 0x01 when set
  #[bits(1)]
  b1: u8, // occupies 0x02 when set
  #[bits(1)]
  b2: u8, // 0x04
  #[bits(1)]
  b3: u8, // 0x08
}

// MSB layout: bit 0 is the most-significant within each byte
#[packbits::pack(bytes = 1, msb)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MsbByte {
  #[bits(1)]
  b0: u8, // occupies 0x80 when set
  #[bits(1)]
  b1: u8, // 0x40
  #[bits(1)]
  b2: u8, // 0x20
  #[bits(1)]
  b3: u8, // 0x10
}

#[test]
fn lsb_order_roundtrip() {
  // Set b0 and b2 -> 0x01 | 0x04 = 0x05
  let s = LsbByte { b0: 1, b1: 0, b2: 1, b3: 0 };
  let arr: [u8; 1] = s.into();
  assert_eq!(arr[0], 0x05);
  let back: LsbByte = arr.into();
  assert_eq!(back, s);
}

#[test]
fn msb_order_roundtrip() {
  // Set b0 and b2 in msb numbering -> 0x80 | 0x20 = 0xA0
  let s = MsbByte { b0: 1, b1: 0, b2: 1, b3: 0 };
  let arr: [u8; 1] = s.into();
  assert_eq!(arr[0], 0xA0);
  let back: MsbByte = arr.into();
  assert_eq!(back, s);
}
