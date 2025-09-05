#![allow(dead_code)]

use packbits as _; // just to ensure the crate is in scope

#[packbits::pack(bytes = 2)]
struct S10 {
  #[skip(3)]
  #[bits(10)]
  v: i16,
  #[bits(3)]
  pad: u8,
}

#[test]
fn roundtrip_i16_10bit_unaligned() {
  for &v in &[-512, -256, -1, 0, 1, 255, 511] {
    let s = S10 { v, pad: 0 };
    let bytes: [u8; 2] = s.into();
    let s2: S10 = bytes.into();
    assert_eq!(s2.v, v, "v={}, bytes={:02x?}", v, bytes);
    assert_eq!(s2.pad, 0);
  }
}

#[packbits::pack(bytes = 6)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Xyz {
  #[bits(16)]
  x: i16,
  #[bits(16)]
  y: i16,
  #[bits(16)]
  z: i16,
}

#[test]
fn roundtrip_xyz_i16_16bit_aligned() {
  let cases = [
    Xyz { x: 0, y: 0, z: 0 },
    Xyz { x: -1, y: 1, z: -2 },
    Xyz { x: i16::MIN, y: i16::MAX, z: -1234 },
  ];
  for &xyz in &cases {
    let bytes: [u8; 6] = xyz.into();
    let got: Xyz = bytes.into();
    assert_eq!(got, xyz);
    // Spot-check endianness: LSB of x goes first
    assert_eq!(bytes[0], (xyz.x as u16 & 0x00FF) as u8);
    assert_eq!(bytes[1], ((xyz.x as u16 >> 8) & 0x00FF) as u8);
  }
}
