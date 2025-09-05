use packbits as _;

#[packbits::pack(bytes = 1)]
struct Bad {
    a: u64,
}

fn main() {}
