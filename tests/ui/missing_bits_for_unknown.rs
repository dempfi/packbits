use packbits as _;

struct Field {}

#[packbits::pack(bytes = 1)]
struct Bad {
  a: Field,
}

fn main() {}
