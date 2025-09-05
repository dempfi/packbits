use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::ItemStruct;

mod args;
mod attrs;
mod diagram;
mod generator;
mod layout;

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
  use args::ContainerOpts;
  use attrs::Attrs;
  use generator::Generator;
  use layout::Layout;

  let mut errors = Vec::new();
  let opts = ContainerOpts::parse(args, &mut errors);
  let name = item.ident.clone();

  let layout = Layout::new(&item, opts.bytes_len, &mut errors);
  if layout.fields.is_empty() && !errors.is_empty() {
    return emit_or_compile_errors(quote! {}, errors);
  }

  let cleaned = Attrs::strip_field_attrs(item);
  let tokens = Generator::new(&name, opts.bytes_len, opts.int_ident.as_ref(), opts.bit_order, &layout, cleaned).build();
  emit_or_compile_errors(tokens, errors)
}
