use packbits as _;

#[packbits::pack(b = 1)]
struct Bad {
  a: u8,
}

fn main() {}
