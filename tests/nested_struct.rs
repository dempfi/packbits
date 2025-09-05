#![allow(dead_code)]

use packbits as _;

#[packbits::pack(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Inner {
  #[skip(2)]
  #[bits(5)]
  a: u8,
}

#[packbits::pack(bytes = 4)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Outer {
  #[skip(1)]
  #[bits(16)]
  inner: Inner,
}

#[test]
fn roundtrip_nested_structs() {
  let s = Outer { inner: Inner { a: 24 } };
  let bytes: [u8; 4] = s.try_into().unwrap();
  let got: Outer = bytes.try_into().unwrap();
  assert_eq!(got, s, "bytes={:02x?}", bytes);
}
