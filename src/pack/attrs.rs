use syn::{Attribute, Fields, ItemStruct, LitInt, Result, spanned::Spanned};

// Parsed value for #[bits(W)]
#[derive(Copy, Clone)]
pub(super) struct Bits {
  pub width: u16,
}

pub(super) struct Attrs;

impl Attrs {
  fn find_attr<'a>(attrs: &'a [Attribute], ident: &str) -> Option<&'a Attribute> {
    attrs.iter().find(|a| a.path().is_ident(ident))
  }

  // Find and parse #[bits(W)] attribute on a field, validating the width range.
  pub(super) fn parse_bits(attrs: &[Attribute]) -> Option<Result<Bits>> {
    Self::find_attr(attrs, "bits").map(|a| {
      a.parse_args_with(|input: syn::parse::ParseStream| {
        let width = input.parse::<LitInt>()?.base10_parse::<u32>()? as u16;
        if !(1..=128).contains(&width) {
          Err(syn::Error::new(a.span(), "bits width must be 1..=128"))
        } else {
          Ok(Bits { width })
        }
      })
    })
  }

  // Find and parse #[skip(N)] attribute on a field; N must be > 0.
  pub(super) fn parse_skip(attrs: &[Attribute]) -> Option<Result<u32>> {
    Self::find_attr(attrs, "skip").map(|a| {
      a.parse_args_with(|input: syn::parse::ParseStream| {
        let v = input.parse::<LitInt>()?.base10_parse::<u32>()?;
        if v == 0 {
          Err(syn::Error::new(a.span(), "`skip` must be > 0"))
        } else {
          Ok(v)
        }
      })
    })
  }

  // Remove pack-related field attributes from the generated struct (doc clarity).
  fn is_pack_attr(a: &Attribute) -> bool {
    a.path().is_ident("bits") || a.path().is_ident("skip")
  }

  pub(super) fn strip_field_attrs(mut item: ItemStruct) -> ItemStruct {
    match &mut item.fields {
      Fields::Named(named) => {
        for f in named.named.iter_mut() {
          f.attrs.retain(|a| !Self::is_pack_attr(a));
        }
      }
      Fields::Unnamed(unnamed) => {
        for f in unnamed.unnamed.iter_mut() {
          f.attrs.retain(|a| !Self::is_pack_attr(a));
        }
      }
      Fields::Unit => {}
    }
    item
  }
}
