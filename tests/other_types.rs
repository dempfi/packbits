#![allow(dead_code)]

use packbits as _;

// A small enum that only accepts values 0..=3 when converting from u8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
  M0,
  M1,
  M2,
  M3,
}

impl core::convert::From<Mode> for u8 {
  fn from(m: Mode) -> Self {
    match m {
      Mode::M0 => 0,
      Mode::M1 => 1,
      Mode::M2 => 2,
      Mode::M3 => 3,
    }
  }
}

impl core::convert::TryFrom<u8> for Mode {
  type Error = &'static str;
  fn try_from(v: u8) -> Result<Self, Self::Error> {
    Ok(match v {
      0 => Mode::M0,
      1 => Mode::M1,
      2 => Mode::M2,
      3 => Mode::M3,
      _ => return Err("invalid mode"),
    })
  }
}

// bytes=1, first field is 3-bit Mode; second is a 5-bit payload.
#[packbits::pack(bytes = 1)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WithMode {
  #[bits(3)]
  mode: Mode,
  #[bits(5)]
  payload: u8,
}

#[test]
fn try_from_error_bubbles_up_for_other_type() {
  // Construct bytes where low 3 bits (mode) = 0b111 = 7 (invalid for Mode)
  let bytes: [u8; 1] = [0b0000_0111];
  let err = <WithMode as core::convert::TryFrom<[u8; 1]>>::try_from(bytes).unwrap_err();
  assert_eq!(err, "pack: field conversion failed: Mode");
}

// A 9-bit-like enum crossing the byte boundary; use enum so casts like `as u16` work in write path
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum NineE {
  V_0x155 = 0x0155, // 0b1_0101_0101
  V_0x0AA = 0x00AA, // 0b0_1010_1010 (another optional value)
}

impl From<NineE> for u16 {
  fn from(v: NineE) -> Self {
    v as u16
  }
}

impl core::convert::TryFrom<u16> for NineE {
  type Error = &'static str;
  fn try_from(v: u16) -> Result<Self, Self::Error> {
    match v & 0x01FF {
      // only lower 9 bits are relevant
      0x0155 => Ok(NineE::V_0x155),
      0x00AA => Ok(NineE::V_0x0AA),
      _ => Err("invalid NineE"),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TwoE {
  Variant0 = 0x0,
  Variant1 = 0x1,
  Variant2 = 0x3,
}

impl core::convert::TryFrom<u8> for TwoE {
  type Error = &'static str;
  fn try_from(v: u8) -> Result<Self, Self::Error> {
    match v {
      0x0 => Ok(TwoE::Variant0),
      0x1 => Ok(TwoE::Variant1),
      0x3 => Ok(TwoE::Variant2),
      _ => Err("invalid TwoE"),
    }
  }
}

impl core::convert::From<TwoE> for u8 {
  fn from(v: TwoE) -> u8 {
    v as u8
  }
}

#[packbits::pack(bytes = 3)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CrossByte(#[bits(5)] u8, #[bits(16)] NineE, #[bits(2)] TwoE);

#[test]
fn other_type_crosses_byte_boundary() {
  let v = CrossByte(0b1_0101, NineE::V_0x155, TwoE::Variant2);
  let bytes: [u8; 3] = v.try_into().unwrap();
  // Reconstruct and compare (fallible because of Other field)
  let back = CrossByte::try_from(bytes).unwrap();
  assert_eq!(back, v);
}

// MSB order with an Other type occupying the top bits of the byte
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tri {
  Z,
  O,
  T,
}

impl core::convert::From<Tri> for u8 {
  fn from(t: Tri) -> Self {
    match t {
      Tri::Z => 0,
      Tri::O => 1,
      Tri::T => 2,
    }
  }
}
impl core::convert::From<u8> for Tri {
  fn from(v: u8) -> Self {
    match v {
      0 => Tri::Z,
      1 => Tri::O,
      2 => Tri::T,
      _ => Tri::Z,
    }
  }
}

#[packbits::pack(bytes = 1, msb)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MsbOther {
  #[bits(3)]
  t: Tri, // occupies the top 3 bits of the byte (msb order)
  #[bits(5)]
  lo: u8, // occupies the low 5 bits
}

#[test]
fn msb_order_other_bits_positioning() {
  let s = MsbOther { t: Tri::T, lo: 0b1_0110 }; // Tri::T = 2 (0b010)
  let arr: [u8; 1] = s.try_into().unwrap();
  // In MSB order: high 3 bits store 0b010 -> 0b0100_0000; low 5 bits are 0b1_0110 -> 0b0010_110
  assert_eq!(arr[0], (2u8 << 5) | (0b1_0110 & 0b1_1111));
  let back = MsbOther::try_from(arr).unwrap();
  assert_eq!(back, s);
}
