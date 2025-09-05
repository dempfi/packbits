use packbits as _;

#[packbits::pack(bytes = 2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pair(u8, u8);

#[test]
fn tuple_roundtrip() {
  let p = Pair(0x12, 0x34);
  let bytes: [u8; 2] = p.into();
  assert_eq!(bytes, [0x12, 0x34]);
  let back: Pair = bytes.into();
  assert_eq!(back, p);
}

#[packbits::pack(bytes = 2)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkipBitsTuple(
  #[skip(4)]
  #[bits(4)]
  u8,
  #[bits(8)] u8,
);

#[test]
fn tuple_attrs_skip_and_bits() {
  // First field is 4 bits wide, placed after a 4-bit skip; second is full byte
  let v = SkipBitsTuple(0b1010, 0xFF);
  let bytes: [u8; 2] = v.into();
  // LSB0: skip(4) reserves low nibble, then 4-bit field occupies high nibble → 0xA0
  assert_eq!(bytes, [0xA0, 0xFF]);
  let back: SkipBitsTuple = bytes.into();
  assert_eq!(back, v);
}
