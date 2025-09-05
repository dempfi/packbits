use syn::{spanned::Spanned, Attribute, Fields, ItemStruct, LitInt, Result as SynResult};

#[derive(Copy, Clone)]
pub(super) struct Bits {
  pub width: u16,
}

pub(super) fn parse_bits_attr(attrs: &[Attribute]) -> Option<SynResult<Bits>> {
  for a in attrs {
    if a.path().is_ident("bits") {
      return Some(a.parse_args_with(|input: syn::parse::ParseStream| {
        let w_lit: LitInt = input.parse()?;
        let width = w_lit.base10_parse::<u32>()? as u16;
        if !(1..=32).contains(&width) {
          return Err(syn::Error::new(a.span(), "bits width must be 1..=32"));
        }
        Ok(Bits { width })
      }));
    }
  }
  None
}

pub(super) fn parse_skip_attr(attrs: &[Attribute]) -> Option<SynResult<u32>> {
  for a in attrs {
    if a.path().is_ident("skip") {
      return Some(a.parse_args_with(|input: syn::parse::ParseStream| {
        let n: LitInt = input.parse()?;
        let v = n.base10_parse::<u32>()?;
        if v == 0 {
          return Err(syn::Error::new(a.span(), "`skip` must be > 0"));
        }
        Ok(v)
      }));
    }
  }
  None
}

pub(super) fn strip_field_attrs(mut item: ItemStruct) -> ItemStruct {
  if let Fields::Named(named) = &mut item.fields {
    for f in named.named.iter_mut() {
      f.attrs
        .retain(|a| !(a.path().is_ident("bits") || a.path().is_ident("skip")));
    }
  }
  item
}
