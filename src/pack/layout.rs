use syn::{spanned::Spanned, Fields, Ident, ItemStruct, Type};

use crate::pack::attrs::{parse_bits_attr, parse_skip_attr};

#[derive(Clone)]
pub(super) struct FieldSpec {
  pub ident: Ident,
  pub ty: Type,
  pub width: u16,
  pub start_bit: usize,
}

pub(super) struct Layout {
  pub fields: Vec<FieldSpec>,
  pub reserved: Vec<bool>, // bits reserved by #[skip], length = total_bits
}

fn span_fits(used: &[u8], start_bit: usize, width: u16, total_bits: usize) -> bool {
  if start_bit + width as usize > total_bits {
    return false;
  }
  let mut remain = width;
  let mut b = start_bit / 8;
  let mut off = (start_bit % 8) as u8;
  while remain > 0 {
    let take = core::cmp::min(8 - off, remain as u8) as u8;
    let mask: u8 = (((1u16 << take as u16) - 1) as u8) << off;
    if (used[b] & mask) != 0 {
      return false;
    }
    remain -= take as u16;
    b += 1;
    off = 0;
  }
  true
}

fn reserve_bits(used: &mut [u8], start_bit: usize, width: u16) {
  let mut remain = width;
  let mut b = start_bit / 8;
  let mut off = (start_bit % 8) as u8;
  while remain > 0 {
    let take = core::cmp::min(8 - off, remain as u8) as u8;
    let mask: u8 = (((1u16 << take as u16) - 1) as u8) << off;
    used[b] |= mask;
    remain -= take as u16;
    b += 1;
    off = 0;
  }
}

fn find_slot(used: &[u8], start: usize, width: u16, total_bits: usize) -> Option<usize> {
  let mut pos = start;
  while pos + width as usize <= total_bits {
    if span_fits(used, pos, width, total_bits) {
      return Some(pos);
    }
    pos += 1;
  }
  None
}

pub(super) fn compute(item: &ItemStruct, nbytes: usize, errors: &mut Vec<syn::Error>) -> Layout {
  let named = match &item.fields {
    Fields::Named(n) => &n.named,
    _ => {
      errors.push(syn::Error::new(item.span(), "#[pack] requires a struct with named fields"));
      return Layout { fields: vec![], reserved: vec![] };
    }
  };
  let total_bits = nbytes.max(1) * 8;
  let mut used = vec![0u8; nbytes.max(1)];
  let mut reserved = vec![false; total_bits];
  let mut auto_bit: usize = 0;
  let mut out = Vec::new();

  for field in named {
    let ident = field.ident.clone().expect("named");

    // Parse attrs
    let bits_attr = parse_bits_attr(&field.attrs);
    let skip_attr = parse_skip_attr(&field.attrs);

    // Determine width: bits override, else inferred for known types
    let width = match bits_attr {
      Some(Ok(b)) => b.width,
      Some(Err(e)) => {
        errors.push(e);
        continue;
      }
      None => match super::inferred_width(&field.ty) {
        Some(w) => w,
        None => {
          errors.push(syn::Error::new(field.span(), "missing #[bits(W)] for this field type"));
          continue;
        }
      },
    };

    // Apply skip (reserve pre-gap) if present
    if let Some(Ok(skip_bits)) = skip_attr {
      let w = skip_bits as u16;
      if !span_fits(&used, auto_bit, w, total_bits) {
        errors.push(syn::Error::new(field.span(), "skip range exceeds or overlaps existing bits"));
        continue;
      }
      reserve_bits(&mut used, auto_bit, w);
      for i in 0..(w as usize) {
        reserved[auto_bit + i] = true;
      }
      auto_bit += w as usize;
    } else if let Some(Err(e)) = skip_attr {
      errors.push(e);
      continue;
    }

    // Place field starting at auto cursor
    let start_bit = match find_slot(&used, auto_bit, width, total_bits) {
      Some(p) => p,
      None => {
        errors.push(syn::Error::new(field.span(), "not enough space for field"));
        continue;
      }
    };

    reserve_bits(&mut used, start_bit, width);
    out.push(FieldSpec { ident, ty: field.ty.clone(), width, start_bit });
    auto_bit = start_bit + width as usize;
  }

  Layout { fields: out, reserved }
}

