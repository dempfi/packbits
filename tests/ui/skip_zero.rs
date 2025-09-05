use packbits as _;

#[packbits::pack(bytes = 1)]
struct Bad {
    #[skip(0)]
    a: u8,
}

fn main() {}
