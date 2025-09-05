//! packbits — tiny, zero-boilerplate bit packing for your own structs
//!
//! Attach a single attribute to a normal Rust struct (named or tuple) to pack/unpack
//! directly to a fixed-size byte array (and optionally a single integer container).
//! You keep your type’s API; the macro only adds conversions.
//!
//! - One attribute: `#[pack(bytes = N)]` or `#[pack(u8|u16|u32|u64|u128)]`.
//!   - Shorthand: `#[pack]` is equivalent to `#[pack(u8)]`.
//!   - Optional bit order per byte: add `msb` or `lsb` (default `lsb`).
//! - Per-field directives:
//!   - `#[bits(W)]` → width override (1..=128). If omitted and the field type is
//!     one of {bool, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128}, its width is inferred
//!     (bool=1, integer types use their full width). Otherwise `#[bits]` is required.
//!   - `#[skip(N)]` → reserves N bits immediately before the field.
//! - Clean output: generated code uses straight-line byte ops (no runtime loops) and is no_std-friendly.
//! - Documentation candy: the macro appends an ASCII bit layout diagram into your struct’s docs.
//!
//! Conversions
//! - For structs with only primitive fields (bool/integers):
//!   - `From<T> for [u8; N]` and `From<[u8; N]> for T` are generated (infallible).
//! - If any field is a custom type:
//!   - Both directions use `TryFrom` instead, with `&'static str` errors.
//! - If an integer container form is used, e.g. `#[pack(u32)]`, matching `From`/`TryFrom` impls
//!   are provided to and from that integer as well. Multi-byte loads/stores are little-endian.
//!
//! Signed fields and masking
//! - Unsigned fields are masked to their declared width on write; on read, bits are assembled as-is.
//! - Signed fields narrower than their native width are sign-extended on read and masked on write.
//!
//! Custom field types
//! - Specify a width with `#[bits(W)]` and provide conversions to/from the minimal unsigned carrier
//!   type large enough to hold W bits (`u8`, `u16`, …, up to `u128`). On read, the macro expects
//!   `TryFrom<uN> for YourType`; on write, it expects `TryFrom<YourType> for uN`.
//!
//! Bit order and endianness
//! - Bit order controls numbering within a byte: `lsb` (default) means bit 0 is least-significant;
//!   `msb` means bit 0 is most-significant. Multi-byte, byte-aligned primitives use little-endian
//!   (`to_le_bytes`/`from_le_bytes`), as do integer container conversions.
//!
//! Limitations
//! - Maximum per-field width is 128 bits.
//! - The macro does not generate getters/setters or other mutation helpers—by design.
//!
//! Compile-time checks
//! - Missing `#[bits(W)]` for non-primitive field types.
//! - `#[bits(W)]` outside 1..=128.
//! - `#[skip(N)]` must be > 0 and within bounds.
//! - Not enough space for a field in the chosen container size.
//! - Invalid attribute arguments (only `bytes = N`, `u8|u16|u32|u64|u128`, `msb|lsb` are accepted).
//!
//! Example: keep your own API, get conversions for free
//! ```
//! use packbits as _;
//!
//! #[packbits::pack(u16)]
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! struct Header {
//!   #[bits(3)] ver: u8,
//!   #[bits(5)] kind: u8,
//!   #[bits(8)] len: u8,
//! }
//!
//! impl Header {
//!   pub fn is_control(&self) -> bool { self.kind == 0b101 }
//! }
//!
//! let h = Header { ver: 1, kind: 0b101, len: 42 };
//! let raw: u16 = h.into(); // From<Header> for u16
//! let back: Header = raw.into();
//! assert!(back.is_control());
//! ```

extern crate proc_macro;
use proc_macro::TokenStream;

mod pack;

#[proc_macro_attribute]
pub fn pack(args: TokenStream, input: TokenStream) -> TokenStream {
  pack::expand(args.into(), syn::parse_macro_input!(input as syn::ItemStruct)).into()
}
