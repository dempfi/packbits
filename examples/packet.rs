use core::convert::{From, TryFrom};

#[allow(dead_code)]
enum Prio {
  Low,
  Med,
  Hi,
}

impl From<Prio> for u8 {
  fn from(p: Prio) -> u8 {
    match p {
      Prio::Low => 0,
      Prio::Med => 1,
      Prio::Hi => 2,
    }
  }
}

impl TryFrom<u8> for Prio {
  type Error = &'static str;
  fn try_from(v: u8) -> Result<Self, Self::Error> {
    Ok(match v & 0b11 {
      0 => Prio::Low,
      1 => Prio::Med,
      2 => Prio::Hi,
      _ => unreachable!(),
    })
  }
}

#[packbits::pack(u8)]
struct Kind {
  #[bits(3)]
  class: u8,
  #[bits(5)]
  code: u8,
}

#[packbits::pack(u32)]
struct Packet {
  #[bits(3)]
  ver: u8,
  #[skip(1)]
  #[bits(8)]
  kind: Kind,
  #[bits(8)]
  priority: Prio,
  #[bits(12)]
  delta: i16,
}

fn main() {
  let p = Packet { ver: 1, kind: Kind { class: 0b101, code: 0x12 }, priority: Prio::Med, delta: -7 };

  // Integer container roundtrip (little-endian on the wire)
  let word: u32 = p.try_into().unwrap();
  let back_from_word: Packet = word.try_into().unwrap();
  let _ = back_from_word; // silence unused var

  // Or via a fixed-size byte array (derive from the integer container to avoid moving `p` again)
  let bytes: [u8; 4] = word.to_le_bytes();
  let back_from_bytes: Packet = bytes.try_into().unwrap();
  let _ = back_from_bytes;

  println!("word=0x{word:08x} bytes={bytes:?}");
}
