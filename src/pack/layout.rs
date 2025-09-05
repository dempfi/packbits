use syn::{Fields, Ident, ItemStruct, Type, spanned::Spanned};

use crate::pack::attrs::Attrs;

#[derive(Copy, Clone, Eq, PartialEq)]
pub(super) enum FieldKind {
  Bool,
  Int { signed: bool, bytes: u8 },
  Custom,
}

impl FieldKind {
  pub(super) fn from_type(ty: &Type) -> Self {
    let ident = match ty {
      Type::Path(tp) => tp.path.get_ident().map(|i| i.to_string()),
      _ => None,
    };
    match ident.as_deref() {
      Some("bool") => FieldKind::Bool,
      Some("u8") => FieldKind::Int { signed: false, bytes: 1 },
      Some("i8") => FieldKind::Int { signed: true, bytes: 1 },
      Some("u16") => FieldKind::Int { signed: false, bytes: 2 },
      Some("i16") => FieldKind::Int { signed: true, bytes: 2 },
      Some("u32") => FieldKind::Int { signed: false, bytes: 4 },
      Some("i32") => FieldKind::Int { signed: true, bytes: 4 },
      Some("u64") => FieldKind::Int { signed: false, bytes: 8 },
      Some("i64") => FieldKind::Int { signed: true, bytes: 8 },
      Some("u128") => FieldKind::Int { signed: false, bytes: 16 },
      Some("i128") => FieldKind::Int { signed: true, bytes: 16 },
      _ => FieldKind::Custom,
    }
  }

  pub(super) fn byte_len(self) -> Option<usize> {
    match self {
      FieldKind::Int { bytes, .. } => Some(bytes as usize),
      _ => None,
    }
  }

  pub(super) fn full_bits(self) -> Option<u16> {
    self.byte_len().map(|b| (b * 8) as u16)
  }

  pub(super) fn inferred_width(ty: &Type) -> Option<u16> {
    match Self::from_type(ty) {
      FieldKind::Bool => Some(1),
      kind => kind.full_bits(),
    }
  }
}

#[derive(Clone)]
pub(super) struct FieldSpec {
  pub ident: Ident,
  pub ty: Type,
  pub width: u16,
  pub start_bit: usize,
  pub kind: FieldKind,
}

impl FieldSpec {
  pub(super) fn is_byte_aligned(&self) -> bool {
    self.start_bit % 8 == 0 && self.width % 8 == 0
  }
  pub(super) fn start_byte(&self) -> usize {
    self.start_bit / 8
  }
  pub(super) fn aligned_primitive_len(&self) -> Option<usize> {
    if self.is_byte_aligned() && self.kind.full_bits() == Some(self.width) {
      self.kind.byte_len()
    } else {
      None
    }
  }
}

pub(super) struct Layout {
  pub fields: Vec<FieldSpec>,
  pub is_tuple: bool,
  pub fallible: bool,
}

impl Layout {
  pub(super) fn new(item: &ItemStruct, nbytes: usize, errors: &mut Vec<syn::Error>) -> Self {
    // Linear planner: walk fields left-to-right with a single bit cursor.
    let total_bits = nbytes.saturating_mul(8);
    let mut cursor_bit: usize = 0;
    let (iter, is_tuple) = match &item.fields {
      Fields::Named(n) => (n.named.iter().enumerate().map(|(i, f)| (f, i)).collect::<Vec<_>>(), false),
      Fields::Unnamed(u) => (u.unnamed.iter().enumerate().map(|(i, f)| (f, i)).collect::<Vec<_>>(), true),
      Fields::Unit => (Vec::new(), false),
    };
    let mut fields: Vec<FieldSpec> = Vec::with_capacity(iter.len());
    for (field, idx) in iter {
      if let Some(spec) = Self::process_field(total_bits, &mut cursor_bit, field, idx, item, errors) {
        fields.push(spec);
      }
    }
    let fallible = fields.iter().any(|f| matches!(f.kind, FieldKind::Custom));
    Layout { fields, is_tuple, fallible }
  }

  fn process_field(
    total_bits: usize,
    cursor_bit: &mut usize,
    field: &syn::Field,
    idx: usize,
    item: &ItemStruct,
    errors: &mut Vec<syn::Error>,
  ) -> Option<FieldSpec> {
    let ident = match (&item.fields, &field.ident) {
      (Fields::Named(_), Some(id)) => id.clone(),
      (Fields::Unnamed(_), _) => syn::Ident::new(&format!("__f{}", idx), field.span()),
      _ => syn::Ident::new("_", field.span()),
    };

    // width
    let width = match Attrs::parse_bits(&field.attrs) {
      Some(Ok(bits)) => bits.width,
      Some(Err(e)) => {
        errors.push(e);
        return None;
      }
      None => match FieldKind::inferred_width(&field.ty) {
        Some(w) => w,
        None => {
          errors.push(syn::Error::new(field.span(), "missing #[bits(W)] for this field type"));
          return None;
        }
      },
    };

    // optional skip
    if let Some(skip) = Attrs::parse_skip(&field.attrs) {
      match skip {
        Ok(bits) => {
          let add = bits as usize;
          if cursor_bit.saturating_add(add) > total_bits {
            errors.push(syn::Error::new(field.span(), "skip range exceeds or overlaps existing bits"));
            return None;
          }
          *cursor_bit += add;
        }
        Err(e) => {
          errors.push(e);
          return None;
        }
      }
    }

    // allocate sequentially
    if cursor_bit.saturating_add(width as usize) > total_bits {
      errors.push(syn::Error::new(field.span(), "not enough space for field"));
      return None;
    }
    let start_bit = *cursor_bit;
    *cursor_bit += width as usize;

    let kind = FieldKind::from_type(&field.ty);
    Some(FieldSpec { ident, ty: field.ty.clone(), width, start_bit, kind })
  }
}
