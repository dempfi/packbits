#[packbits::pack]
struct Lsb {
  #[bits(1)]
  b0: u8,
  #[bits(1)]
  b1: u8,
}

#[packbits::pack(msb)]
struct Msb {
  #[bits(1)]
  b0: u8,
  #[bits(1)]
  b1: u8,
}

fn main() {
  let lsb = Lsb { b0: 1, b1: 0 };
  let msb = Msb { b0: 1, b1: 0 };

  let a: [u8; 1] = lsb.into();
  let b: [u8; 1] = msb.into();

  println!("lsb={:#04x} msb={:#04x}", a[0], b[0]);
}
