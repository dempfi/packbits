use packbits as _;

#[packbits::pack(bytes = 1)]
struct Bad {
  a: u8,
  #[bits(1)]
  b: u8,
}

fn main() {}
