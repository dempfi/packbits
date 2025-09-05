use proc_macro2::TokenStream as TokenStream2;
use syn::{Ident, LitInt, Result as SynResult, Token};

pub(super) struct RegArgs {
  pub bytes: usize,
}

impl syn::parse::Parse for RegArgs {
  fn parse(input: syn::parse::ParseStream) -> SynResult<Self> {
    if input.is_empty() {
      return Ok(Self { bytes: 1 });
    }
    let key: Ident = input.parse()?;
    if key != "bytes" {
      return Err(syn::Error::new(key.span(), "expected `bytes = <int>`"));
    }
    let _eq: Token![=] = input.parse()?;
    let lit: LitInt = input.parse()?;
    Ok(Self { bytes: lit.base10_parse::<usize>()? })
  }
}

pub(super) fn parse_bytes(tokens: TokenStream2, errors: &mut Vec<syn::Error>) -> usize {
  match syn::parse2::<RegArgs>(tokens) {
    Ok(parsed) => parsed.bytes,
    Err(err) => {
      errors.push(err);
      1
    }
  }
}

