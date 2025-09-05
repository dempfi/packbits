#![allow(dead_code)]

use packbits as _;

#[packbits::pack(bytes = 2)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NarrowSigned {
  #[bits(5)]
  a: i8, // range [-16..15]
  #[bits(11)]
  b: i16, // range [-1024..1023]
}

#[test]
fn roundtrip_narrow_signed() {
  let a_vals: &[i8] = &[-16, -1, 0, 1, 15];
  let b_vals: &[i16] = &[-1024, -257, -1, 0, 1, 1023];
  for &a in a_vals {
    for &b in b_vals {
      let s = NarrowSigned { a, b };
      let bytes: [u8; 2] = s.into();
      let got: NarrowSigned = bytes.into();
      assert_eq!(got, s, "a={a}, b={b}, bytes={:02x?}", bytes);
    }
  }
}

#[packbits::pack(bytes = 4)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AlignedU32 {
  #[bits(32)]
  x: u32,
}

#[test]
fn aligned_u32_endianness() {
  let cases = [0u32, 1, 0x00FF_0000, 0xDEAD_BEEF, u32::MAX];
  for &x in &cases {
    let arr: [u8; 4] = AlignedU32 { x }.into();
    assert_eq!(arr, x.to_le_bytes());
    let back: AlignedU32 = arr.into();
    assert_eq!(back.x, x);
  }
}
