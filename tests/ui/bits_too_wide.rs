use packbits as _;

#[packbits::pack(bytes = 1)]
struct Bad {
    #[bits(33)]
    a: u32,
}

fn main() {}
