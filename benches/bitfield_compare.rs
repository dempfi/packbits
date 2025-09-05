use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

// 32-bit layout with reserved gaps to avoid natural alignment
// msb0 bit positions:
// 0..=2 (3)  reserved
// 3..=7 (5)  ver
// 8..=9 (2)  reserved
// 10..=16 (7) kind
// 17 (1)     reserved
// 18..=26 (9) len
// 27..=31 (5) flags
#[packbits::pack(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Header {
  #[skip(3)]
  #[bits(5)]
  ver: u8,
  #[skip(2)]
  #[bits(7)]
  kind: u8,
  #[skip(1)]
  #[bits(9)]
  len: u16,
  #[bits(5)]
  flags: u8,
}

// modular-bitfield equivalent
mod mb {
  use modular_bitfield::prelude::*;

  #[bitfield(bits = 32)]
  #[derive(Clone, Copy)]
  pub struct Header {
    #[skip]
    __: B3,
    pub ver: B5,
    #[skip]
    __2: B2,
    pub kind: B7,
    #[skip]
    __3: B1,
    pub len: B9,
    pub flags: B5,
  }
}

// packed_struct equivalent
mod ps {
  use packed_struct::prelude::*;

  #[derive(PackedStruct, Default, Clone, Copy)]
  #[packed_struct(bit_numbering = "msb0", size_bytes = "4", endian = "msb")]
  pub struct Header {
    // 0..=2 reserved
    #[packed_field(bits = "3..=7")]
    pub ver: Integer<u8, packed_bits::Bits<5>>,
    // 8..=9 reserved
    #[packed_field(bits = "10..=16")]
    pub kind: Integer<u8, packed_bits::Bits<7>>,
    // 17 reserved
    #[packed_field(bits = "18..=26")]
    pub len: Integer<u16, packed_bits::Bits<9>>,
    #[packed_field(bits = "27..=31")]
    pub flags: Integer<u8, packed_bits::Bits<5>>,
  }
}

// bitfield-struct equivalent
mod bfs {
  use bitfield_struct::bitfield;

  #[bitfield(u32)]
  pub struct Header {
    #[bits(3)]
    __: u8,
    #[bits(5)]
    pub ver: u8,
    #[bits(2)]
    __2: u8,
    #[bits(7)]
    pub kind: u8,
    #[bits(1)]
    __3: u8,
    #[bits(9)]
    pub len: u16,
    #[bits(5)]
    pub flags: u8,
  }
}

const REPS: usize = 16; // repeat set/get to include conversion overhead

fn build_sample() -> Header {
  Header { ver: 17, kind: 0b101_0011, len: 0x10A, flags: 0b1_0011 }
}

fn bench_pack(c: &mut Criterion) {
  let mut group = c.benchmark_group("pack");
  let sample = build_sample();

  group.throughput(Throughput::Bytes(4));

  // packbits via integer container Into<u32>
  group.bench_function(BenchmarkId::new("packbits", "u32"), |b| {
    b.iter_batched(
      || Header { ver: 0, kind: 0, len: 0, flags: 0 },
      |mut h| {
        for _ in 0..REPS {
          h.ver = sample.ver;
          black_box(h.ver);
          h.kind = sample.kind;
          black_box(h.kind);
          h.len = sample.len;
          black_box(h.len);
          h.flags = sample.flags;
          black_box(h.flags);
        }
        let raw: u32 = h.into();
        black_box(raw)
      },
      BatchSize::SmallInput,
    )
  });

  // modular-bitfield: set fields and into bytes
  group.bench_function(BenchmarkId::new("modular-bitfield", "u32"), |b| {
    b.iter_batched(
      mb::Header::new,
      |mut h| {
        for _ in 0..REPS {
          h.set_ver(sample.ver);
          black_box(h.ver());
          h.set_kind(sample.kind);
          black_box(h.kind());
          h.set_len(sample.len);
          black_box(h.len());
          h.set_flags(sample.flags);
          black_box(h.flags());
        }
        let bytes = h.into_bytes();
        let raw = u32::from_be_bytes(bytes);
        black_box(raw)
      },
      BatchSize::SmallInput,
    )
  });

  // packed_struct: pack() -> Vec<u8>
  group.bench_function(BenchmarkId::new("packed-struct", "u32"), |b| {
    use packed_struct::prelude::*;
    b.iter_batched(
      ps::Header::default,
      |mut h| {
        for _ in 0..REPS {
          h.ver = Integer::from_primitive(sample.ver);
          black_box(h.ver.to_primitive());
          h.kind = Integer::from_primitive(sample.kind);
          black_box(h.kind.to_primitive());
          h.len = Integer::from_primitive(sample.len);
          black_box(h.len.to_primitive());
          h.flags = Integer::from_primitive(sample.flags);
          black_box(h.flags.to_primitive());
        }
        let bytes = h.pack().unwrap();
        let raw = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        black_box(raw)
      },
      BatchSize::SmallInput,
    )
  });

  // bitfield-struct: set fields then to_bits
  group.bench_function(BenchmarkId::new("bitfield-struct", "u32"), |b| {
    b.iter_batched(
      bfs::Header::default,
      |mut h| {
        for _ in 0..REPS {
          h.set_ver(sample.ver);
          black_box(h.ver());
          h.set_kind(sample.kind);
          black_box(h.kind());
          h.set_len(sample.len);
          black_box(h.len());
          h.set_flags(sample.flags);
          black_box(h.flags());
        }
        let raw: u32 = h.into_bits();
        black_box(raw)
      },
      BatchSize::SmallInput,
    )
  });

  group.finish();
}

fn bench_unpack(c: &mut Criterion) {
  let mut group = c.benchmark_group("unpack");

  let h = build_sample();
  let raw: u32 = h.into();
  let bytes_be = raw.to_be_bytes();

  group.throughput(Throughput::Bytes(4));

  // packbits from integer container
  group.bench_function(BenchmarkId::new("packbits", "u32"), |b| {
    b.iter(|| {
      let back: Header = Header::from(black_box(raw));
      let mut acc = 0u32;
      for _ in 0..REPS {
        acc = acc.wrapping_add(back.ver as u32);
        acc = acc.wrapping_add(back.kind as u32);
        acc = acc.wrapping_add(back.len as u32);
        acc = acc.wrapping_add(back.flags as u32);
      }
      black_box(acc)
    })
  });

  // modular-bitfield from bytes
  group.bench_function(BenchmarkId::new("modular-bitfield", "u32"), |b| {
    b.iter(|| {
      let h = mb::Header::from_bytes(black_box(bytes_be));
      let mut acc = 0u32;
      for _ in 0..REPS {
        acc = acc.wrapping_add(h.ver() as u32);
        acc = acc.wrapping_add(h.kind() as u32);
        acc = acc.wrapping_add(h.len() as u32);
        acc = acc.wrapping_add(h.flags() as u32);
      }
      black_box(acc)
    })
  });

  // packed_struct from bytes
  group.bench_function(BenchmarkId::new("packed-struct", "u32"), |b| {
    use packed_struct::prelude::*;
    b.iter(|| {
      let parsed: ps::Header = ps::Header::unpack(&black_box(bytes_be)).unwrap();
      let mut acc = 0u32;
      for _ in 0..REPS {
        acc = acc.wrapping_add(parsed.ver.to_primitive() as u32);
        acc = acc.wrapping_add(parsed.kind.to_primitive() as u32);
        acc = acc.wrapping_add(parsed.len.to_primitive() as u32);
        acc = acc.wrapping_add(parsed.flags.to_primitive() as u32);
      }
      black_box(acc)
    })
  });

  // bitfield-struct from integer
  group.bench_function(BenchmarkId::new("bitfield-struct", "u32"), |b| {
    b.iter(|| {
      let parsed = bfs::Header::from_bits(black_box(raw));
      let mut acc = 0u32;
      for _ in 0..REPS {
        acc = acc.wrapping_add(parsed.ver() as u32);
        acc = acc.wrapping_add(parsed.kind() as u32);
        acc = acc.wrapping_add(parsed.len() as u32);
        acc = acc.wrapping_add(parsed.flags() as u32);
      }
      black_box(acc)
    })
  });

  group.finish();
}

criterion_group!(benches, bench_pack, bench_unpack);
criterion_main!(benches);
