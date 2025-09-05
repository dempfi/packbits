//! `#[pack(bytes = N)]` on a struct with named fields.
//!
//! Per-field directives:
//!   - `#[skip(N)]`   → reserves N bits right before this field.
//!   - `#[bits(W)]`   → optional width override (1..=32). If omitted and the
//!                      field type is one of {bool, u8, u16, u32}, its width is
//!                      inferred as 1/8/16/32 respectively; otherwise `#[bits]` is required.
//!
//! Clean output: generated code uses straight-line byte ops (no runtime loops).
//! The macro appends an ASCII table showing the packed layout (up to 4 bytes/row).

extern crate proc_macro;
use proc_macro::TokenStream;

mod pack;

#[proc_macro_attribute]
pub fn pack(args: TokenStream, input: TokenStream) -> TokenStream {
  pack::expand(args.into(), syn::parse_macro_input!(input as syn::ItemStruct)).into()
}
