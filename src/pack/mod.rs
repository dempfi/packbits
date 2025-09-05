use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{ItemStruct, Type};

pub(super) mod args;
pub(super) mod attrs;
pub(super) mod layout;
pub(super) mod codegen;

// ── small type/introspection helpers ──────────────────────────────────────
#[inline]
fn is(ty: &Type, name: &str) -> bool {
  matches!(ty, Type::Path(tp) if tp.path.is_ident(name))
}
#[inline]
fn is_bool(ty: &Type) -> bool {
  is(ty, "bool")
}
#[inline]
fn is_u8(ty: &Type) -> bool {
  is(ty, "u8")
}
#[inline]
fn is_u16(ty: &Type) -> bool {
  is(ty, "u16")
}
#[inline]
fn is_u32(ty: &Type) -> bool {
  is(ty, "u32")
}
#[inline]
fn is_i8(ty: &Type) -> bool {
  is(ty, "i8")
}
#[inline]
fn is_i16(ty: &Type) -> bool {
  is(ty, "i16")
}
#[inline]
fn is_i32(ty: &Type) -> bool {
  is(ty, "i32")
}
fn inferred_width(ty: &Type) -> Option<u16> {
  if is_bool(ty) {
    Some(1)
  } else if is_u8(ty) {
    Some(8)
  } else if is_u16(ty) {
    Some(16)
  } else if is_u32(ty) {
    Some(32)
  } else {
    None
  }
}

// ── driver ────────────────────────────────────────────────────────────────
fn emit_or_compile_errors(tokens: impl ToTokens, errors: Vec<syn::Error>) -> TokenStream2 {
  if errors.is_empty() {
    quote! { #tokens }
  } else {
    let es = errors.into_iter().map(|e| e.to_compile_error());
    quote! { #( #es )* }
  }
}

pub(super) fn expand(args: TokenStream2, item: ItemStruct) -> TokenStream2 {
  use args::parse_bytes;
  use attrs::strip_field_attrs;
  use codegen::generate;
  use layout::compute;

  let mut errors = Vec::new();
  let bytes_len = parse_bytes(args, &mut errors);
  let name = item.ident.clone();

  let layout = compute(&item, bytes_len, &mut errors);
  if layout.fields.is_empty() && !errors.is_empty() {
    return emit_or_compile_errors(quote! {}, errors);
  }

  let cleaned = strip_field_attrs(item);
  let tokens = generate(&name, bytes_len, &layout, cleaned);
  emit_or_compile_errors(tokens, errors)
}

