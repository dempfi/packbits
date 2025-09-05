#![allow(dead_code)]

use packbits as _;

#[packbits::pack(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct S16 {
  #[bits(4)]
  a: u8,
  #[bits(4)]
  b: u8,
  #[bits(8)]
  c: u8,
}

#[test]
fn roundtrip_u16_container() {
  let s = S16 { a: 0xF, b: 0x1, c: 0xAB };
  let arr: [u8; 2] = s.into();
  let n: u16 = s.into();
  assert_eq!(arr, n.to_le_bytes());

  // Infallible: fields are primitive
  let back: S16 = arr.into();
  let back2: S16 = n.into();
  assert_eq!(back, s);
  assert_eq!(back2, s);
}

#[packbits::pack(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct S32 {
  #[bits(12)]
  id: u16,
  #[bits(20)]
  payload: u32,
}

#[test]
fn roundtrip_u32_container() {
  let s = S32 { id: 0xABC, payload: 0xABCDE };
  let arr: [u8; 4] = s.into();
  let n: u32 = s.into();
  assert_eq!(arr, n.to_le_bytes());

  let back: S32 = arr.into();
  let back2: S32 = n.into();
  assert_eq!(back, s);
  assert_eq!(back2, s);
}

#[packbits::pack(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct S8 {
  #[bits(3)]
  a: u8,
  #[bits(5)]
  b: u8,
}

#[test]
fn avoids_duplicate_u8_impls() {
  let s = S8 { a: 0x7, b: 0x1F };
  let b1: u8 = s.into();
  let b2: u8 = <[u8; 1] as From<S8>>::from(s)[0];
  assert_eq!(b1, b2);
  let s1: S8 = [b1].into();
  let s2: S8 = b1.into();
  assert_eq!(s1, s2);
}

#[packbits::pack(u128)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct S128 {
  #[skip(3)]
  a: u64,
  b: u32,
  c: u16,
  d: u8,
}

#[test]
fn roundtrip_u128_container() {
  let s = S128 { a: 0xDEAD_BEEF_F00D_BAAD, b: 0xFEED_FACE, c: 0xBEEF, d: 0xAA };
  let arr: [u8; 16] = s.into();
  let n: u128 = s.into();
  assert_eq!(arr, n.to_le_bytes());

  let back: S128 = arr.into();
  let back2: S128 = n.into();
  assert_eq!(back, s);
  assert_eq!(back2, s);
}
