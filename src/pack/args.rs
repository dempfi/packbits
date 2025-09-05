use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{Ident, LitInt, Result as SynResult, Token};

// Bit order within a byte: LSB0 (bit 0 is least-significant) or MSB0 (bit 0 is most-significant)
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum BitOrder {
  Lsb0,
  Msb0,
}

impl BitOrder {
  pub(super) fn adjust_in_byte(self, off: u8, take: u8) -> u8 {
    match self {
      BitOrder::Lsb0 => off,
      BitOrder::Msb0 => 8 - off - take,
    }
  }

  // Map logical LSB0 bit index within a byte to display column index (MSB on left)
  pub(super) fn display_within(self, bit: usize) -> usize {
    match self {
      BitOrder::Lsb0 => 7 - bit,
      BitOrder::Msb0 => bit,
    }
  }
}

// Argument variants for #[pack(...)] macro
// Parsed options for #[pack(...)]
pub(super) struct ContainerOpts {
  pub bytes_len: usize,
  pub int_ident: Option<Ident>,
  pub bit_order: BitOrder,
}

// Helper to parse comma-separated arguments supporting: bytes=N, u{8,16,32,64,128}, msb|lsb
struct RawArgs {
  bytes: Option<usize>,
  int_ident: Option<(Ident, usize)>,
  bit_order: Option<BitOrder>,
}

impl syn::parse::Parse for RawArgs {
  fn parse(input: syn::parse::ParseStream) -> SynResult<Self> {
    if input.is_empty() {
      return Ok(Self { bytes: None, int_ident: None, bit_order: None });
    }
    let mut bytes: Option<usize> = None;
    let mut int_ident: Option<(Ident, usize)> = None;
    let mut bit_order: Option<BitOrder> = None;
    while !input.is_empty() {
      let ident: Ident = input.parse()?;
      if ident == "bytes" {
        let _eq: Token![=] = input.parse()?;
        let lit: LitInt = input.parse()?;
        bytes = Some(lit.base10_parse::<usize>()?);
      } else if ident == "msb" {
        bit_order = Some(BitOrder::Msb0);
      } else if ident == "lsb" {
        bit_order = Some(BitOrder::Lsb0);
      } else {
        // Accept integer type shorthands: u8/u16/u32/u64/u128
        let (ty, by): (Ident, usize) = match ident.to_string().as_str() {
          "u8" => (ident, 1usize),
          "u16" => (ident, 2usize),
          "u32" => (ident, 4usize),
          "u64" => (ident, 8usize),
          "u128" => (ident, 16usize),
          _ => {
            return Err(syn::Error::new(
              ident.span(),
              "expected `bytes = <int>`, integer type (u8/u16/u32/u64/u128), or `msb`/`lsb`",
            ));
          }
        };
        int_ident = Some((ty, by));
      }
      // Optional trailing comma
      let _ = input.parse::<Token![,]>().ok();
    }
    Ok(Self { bytes, int_ident, bit_order })
  }
}

impl ContainerOpts {
  // Parse container args, recording errors and defaulting to 1 byte, int u8, and LSB bit order.
  pub(super) fn parse(tokens: TokenStream2, errors: &mut Vec<syn::Error>) -> ContainerOpts {
    if tokens.is_empty() {
      return ContainerOpts {
        bytes_len: 1,
        int_ident: Some(Ident::new("u8", Span::call_site())),
        bit_order: BitOrder::Lsb0,
      };
    }

    match syn::parse2::<RawArgs>(tokens) {
      Ok(RawArgs { bytes, int_ident, bit_order }) => {
        // Validate combinations
        if bytes.is_some() && int_ident.is_some() {
          errors
            .push(syn::Error::new(Span::call_site(), "`bytes = N` and an integer container cannot be used together"));
        }
        let (bytes_len, int_ident_final) = match (bytes, int_ident) {
          (Some(n), None) => (n, None),
          (None, Some((id, by))) => (by, Some(id)),
          (None, None) => (1, Some(Ident::new("u8", Span::call_site()))),
          (Some(n), Some((_id, _by))) => (n, None), // error above; ignore int container
        };
        ContainerOpts { bytes_len, int_ident: int_ident_final, bit_order: bit_order.unwrap_or(BitOrder::Lsb0) }
      }
      Err(err) => {
        errors.push(err);
        ContainerOpts { bytes_len: 1, int_ident: None, bit_order: BitOrder::Lsb0 }
      }
    }
  }
}
