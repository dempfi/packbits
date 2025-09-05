#![allow(dead_code)]

use packbits as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flags {
  A,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
}

impl From<Flags> for u8 {
  fn from(f: Flags) -> Self {
    match f {
      Flags::A => 0,
      Flags::B => 1,
      Flags::C => 2,
      Flags::D => 3,
      Flags::E => 4,
      Flags::F => 5,
      Flags::G => 6,
      Flags::H => 7,
    }
  }
}

impl From<u8> for Flags {
  fn from(v: u8) -> Self {
    match v {
      0 => Flags::A,
      1 => Flags::B,
      2 => Flags::C,
      3 => Flags::D,
      4 => Flags::E,
      5 => Flags::F,
      6 => Flags::G,
      7 => Flags::H,
      _ => panic!("invalid flag value"),
    }
  }
}

#[packbits::pack(bytes = 2)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Nested {
  #[bits(3)]
  a: Flags,
  b: u8,
}

#[test]
fn roundtrip_nested() {
  let s = Nested { a: Flags::C, b: 42 };
  let bytes: [u8; 2] = s.try_into().unwrap();
  let got = Nested::try_from(bytes).unwrap();
  assert_eq!(got, s, "bytes={:02x?}", bytes);
}
