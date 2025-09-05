use crate::pack::args::BitOrder;
use crate::pack::layout::{FieldKind, FieldSpec, Layout};

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::ItemStruct;

#[derive(Clone, Copy, Debug)]
pub(super) struct Chunk {
  pub byte_idx: usize,
  pub bit_off: u8,
  pub take: u8,
  pub src_shift: u16,
}

impl Chunk {
  // Compute chunks for a (width, start_bit) field.
  pub(super) fn for_field(width: u16, start_bit: usize) -> Vec<Chunk> {
    let mut v = Vec::new();
    let mut left = width;
    let mut pos = start_bit;
    let mut shift = 0u16;

    while left > 0 {
      let off = (pos % 8) as u8;
      let take = core::cmp::min(8 - off, left as u8);
      v.push(Chunk { byte_idx: pos / 8, bit_off: off, take, src_shift: shift });
      pos += take as usize;
      left -= take as u16;
      shift += take as u16;
    }
    v
  }
}

pub(crate) struct Generator<'a> {
  struct_name: &'a Ident,
  nbytes: usize,
  order: BitOrder,
  layout: &'a Layout,
  cleaned: ItemStruct,
  int_ty: Option<TokenStream>,
}

impl<'a> Generator<'a> {
  pub(super) fn new(
    struct_name: &'a Ident,
    nbytes: usize,
    int_ident: Option<&'a syn::Ident>,
    order: BitOrder,
    layout: &'a Layout,
    cleaned: ItemStruct,
  ) -> Self {
    let int_ty = int_ident.map(|id| quote! { #id });
    Self { struct_name, nbytes, order, layout, cleaned, int_ty }
  }

  fn append_layout_doc(&mut self) {
    use syn::parse_quote;
    self.cleaned.attrs.push(parse_quote!(#[doc = ""]));
    self.cleaned.attrs.push(parse_quote!(#[doc = "_Bit layout_"]));
    let md = super::diagram::Diagram::new(self.nbytes, self.order, &self.layout.fields).render();
    self
      .cleaned
      .attrs
      .extend(md.lines().map(|line| parse_quote!(#[doc = #line])));
  }

  // Destructure an input struct value into local bindings matching field idents
  // so subsequent code can reference them uniformly.
  fn destructure_bindings(&self) -> TokenStream {
    let struct_name = self.struct_name;
    let fields = &self.layout.fields;
    if self.layout.is_tuple {
      let ids: Vec<syn::Ident> = (0..fields.len())
        .map(|i| syn::Ident::new(&format!("__t{}", i), proc_macro2::Span::call_site()))
        .collect();
      let locals = fields.iter().enumerate().map(|(i, f)| {
        let n = &f.ident;
        let s = syn::Ident::new(&format!("__t{}", i), proc_macro2::Span::call_site());
        quote!( let #n = #s; )
      });
      quote!( let #struct_name( #( #ids ),* ) = value; #( #locals )* )
    } else {
      let pat = fields.iter().map(|f| {
        let id = &f.ident;
        quote!( #id )
      });
      quote!( let #struct_name { #( #pat ),* } = value; )
    }
  }

  // Build a plain struct construction expression (no Ok wrapping).
  fn struct_expr_from_bytes(&self) -> TokenStream {
    let fields = &self.layout.fields;
    let struct_name = self.struct_name;
    if self.layout.is_tuple {
      let elems = fields.iter().map(|f| self.gen_from_bytes_expr(f));
      quote! { #struct_name( #( #elems ),* ) }
    } else {
      let inits = fields.iter().map(|f| {
        let n = &f.ident;
        let e = self.gen_from_bytes_expr(f);
        quote!( #n: #e )
      });
      quote! { Self { #( #inits, )* } }
    }
  }

  // Implement either From or TryFrom depending on fallibility.
  fn impl_conv(&self, from_ty: TokenStream, to_ty: TokenStream, body: TokenStream) -> TokenStream {
    if self.layout.fallible {
      quote! { impl core::convert::TryFrom<#from_ty> for #to_ty { type Error = &'static str; fn try_from(value: #from_ty) -> core::result::Result<Self, Self::Error> { #body } } }
    } else {
      quote! { impl core::convert::From<#from_ty> for #to_ty { fn from(value: #from_ty) -> Self { #body } } }
    }
  }

  fn carriers(&self, width: u16) -> (TokenStream, TokenStream, TokenStream) {
    let (u_ty, i_ty) = match width {
      1..=8 => (quote!(u8), quote!(i8)),
      9..=16 => (quote!(u16), quote!(i16)),
      17..=32 => (quote!(u32), quote!(i32)),
      33..=64 => (quote!(u64), quote!(i64)),
      _ => (quote!(u128), quote!(i128)),
    };
    let mask = match width {
      8 => quote!(u8::MAX),
      16 => quote!(u16::MAX),
      32 => quote!(u32::MAX),
      64 => quote!(u64::MAX),
      128 => quote!(u128::MAX),
      _ => {
        let w = width as u32;
        quote!(((1 as #u_ty) << #w) - 1)
      }
    };
    (u_ty, i_ty, mask)
  }

  fn custom_try_from(
    &self,
    ty: &syn::Type,
    from_ty: &TokenStream,
    to_ty: &TokenStream,
    value_expr: &TokenStream,
  ) -> TokenStream {
    let fname = format!("{}", quote::quote!(#ty));
    quote! {
      <#to_ty as core::convert::TryFrom<#from_ty>>::try_from(#value_expr)
        .map_err(|_| concat!("pack: field conversion failed: ", #fname))?
    }
  }

  fn signed_conversion(&self, raw: &TokenStream, width: u32, target_bits: u32) -> TokenStream {
    if width == target_bits {
      let (raw_ty, signed_ty, _) = self.carriers(target_bits as u16);
      quote! {{ let raw: #raw_ty = #raw; raw as #signed_ty }}
    } else {
      let carrier_bits = if target_bits <= 32 { 32 } else { target_bits };
      let shift = carrier_bits.saturating_sub(width);
      let (carrier_ty, signed_carrier_ty, _) = self.carriers(carrier_bits as u16);
      quote! {{
        let raw: #carrier_ty = (#raw) as #carrier_ty;
        ((raw << #shift) as #signed_carrier_ty >> #shift) as #carrier_ty
      }}
    }
  }

  fn raw_expr_for_field(&self, f: &FieldSpec) -> TokenStream {
    let (u, _i, _mask) = self.carriers(f.width);
    let terms = Chunk::for_field(f.width, f.start_bit).into_iter().map(|c| {
      let i = c.byte_idx;
      let take = c.take as u32;
      let sh = c.src_shift as u32;
      let off_adj = self.order.adjust_in_byte(c.bit_off, c.take) as u32;
      if take == 8 {
        quote! { (bytes[#i] as #u) << #sh }
      } else {
        let mask = (1u8 << c.take) - 1;
        quote! { (((bytes[#i] >> #off_adj) & #mask) as #u) << #sh }
      }
    });
    terms.reduce(|a, b| quote! {#a | #b}).unwrap_or_else(|| quote!(0))
  }

  fn gen_byte_chunk_operation(&self, i: usize, val_expr: &TokenStream, take: u32, off_adj: u32) -> TokenStream {
    if take == 8 {
      quote! { bytes[#i] = #val_expr; }
    } else {
      let mask = (1u8 << take) - 1;
      let inv_mask = !(mask << off_adj);
      quote! { bytes[#i] = (bytes[#i] & #inv_mask) | ((#val_expr & #mask) << #off_adj); }
    }
  }

  fn gen_from_bytes_expr(&self, f: &FieldSpec) -> TokenStream {
    let ty = &f.ty;
    let raw = self.raw_expr_for_field(f);
    let (u, _i, _mask_unused) = self.carriers(f.width);
    let start = f.start_byte();
    let k = f.kind;
    if let Some(n) = f.aligned_primitive_len() {
      let elems = (0..n).map(|i| quote! { bytes[#start + #i] });
      return quote! { <#ty>::from_le_bytes([#(#elems),*]) };
    }
    match k {
      FieldKind::Bool => quote!(#raw != 0),
      FieldKind::Int { signed: false, .. } => quote!(#raw),
      FieldKind::Int { signed: true, bytes } => {
        let target_bits: u32 = (bytes as u32) * 8;
        let signed_result = self.signed_conversion(&raw, f.width as u32, target_bits);
        quote!({ let ext = #signed_result; ext as #ty })
      }
      FieldKind::Custom => self.custom_try_from(ty, &u, &quote!(#ty), &quote!(#raw as #u)),
    }
  }

  fn gen_to_bytes_stmt(&self, f: &FieldSpec) -> TokenStream {
    let name = &f.ident;
    let (u, _i, mask) = self.carriers(f.width);
    let start = f.start_byte();
    if let Some(n) = f.aligned_primitive_len() {
      let end = start + n;
      return quote! {
        let le_bytes = #name.to_le_bytes();
        bytes[#start..#end].copy_from_slice(&le_bytes);
      };
    }
    let parts = Chunk::for_field(f.width, f.start_bit).into_iter().map(|c| {
      let i = c.byte_idx;
      let take = c.take as u32;
      let sh = c.src_shift as u32;
      let off_adj = self.order.adjust_in_byte(c.bit_off, c.take) as u32;
      self.gen_byte_chunk_operation(i, &quote! { (val >> #sh) as u8 }, take, off_adj)
    });
    let ty = &f.ty;
    let into_val = match f.kind {
      FieldKind::Custom => self.custom_try_from(ty, &quote!(#ty), &u, &quote!(#name)),
      _ => quote! { #name as #u },
    };
    quote! {
      let val: #u = (#into_val) & #mask;
      #(#parts)*
    }
  }

  pub(super) fn build(mut self) -> TokenStream {
    self.append_layout_doc();
    let cleaned = &self.cleaned;
    let mut out = quote! { #cleaned };

    let to_bytes_stmts: Vec<_> = self.layout.fields.iter().map(|f| self.gen_to_bytes_stmt(f)).collect();
    let destructure = self.destructure_bindings();
    let nbytes = self.nbytes;
    let array_ty = quote! { [u8; #nbytes] };
    let to_bytes_body = if self.layout.fallible {
      quote! {
        let mut bytes: #array_ty = [0u8; #nbytes];
        #destructure
        #( #to_bytes_stmts )*
        Ok(bytes)
      }
    } else {
      quote! {
        let mut bytes: #array_ty = [0u8; #nbytes];
        #destructure
        #( #to_bytes_stmts )*
        bytes
      }
    };
    let struct_name = self.struct_name;
    out.extend(self.impl_conv(quote! { #struct_name }, array_ty.clone(), to_bytes_body));

    let from_bytes_body = {
      let s_expr = self.struct_expr_from_bytes();
      if self.layout.fallible {
        quote! { let bytes = value; Ok(#s_expr) }
      } else {
        quote! { let bytes = value; #s_expr }
      }
    };
    out.extend(self.impl_conv(array_ty.clone(), quote! { #struct_name }, from_bytes_body));

    if let Some(int_ty) = &self.int_ty {
      let to_int_body = if self.layout.fallible {
        quote! {
          let bytes: #array_ty = <#array_ty as core::convert::TryFrom<#struct_name>>::try_from(value)?;
          Ok(<#int_ty>::from_le_bytes(bytes))
        }
      } else {
        quote! {
          let bytes: #array_ty = <#array_ty as core::convert::From<#struct_name>>::from(value);
          <#int_ty>::from_le_bytes(bytes)
        }
      };
      out.extend(self.impl_conv(quote! { #struct_name }, int_ty.clone(), to_int_body));

      let from_int_body = if self.layout.fallible {
        quote! {
          let bytes: #array_ty = value.to_le_bytes();
          <#struct_name as core::convert::TryFrom<#array_ty>>::try_from(bytes)
        }
      } else {
        quote! {
          let bytes: #array_ty = value.to_le_bytes();
          <#struct_name as core::convert::From<#array_ty>>::from(bytes)
        }
      };
      out.extend(self.impl_conv(int_ty.clone(), quote! { #struct_name }, from_int_body));
    }

    out
  }
}
