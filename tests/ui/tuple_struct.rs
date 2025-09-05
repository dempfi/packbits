use packbits as _;

#[packbits::pack(bytes = 1)]
struct Bad(u8);

fn main() {}
