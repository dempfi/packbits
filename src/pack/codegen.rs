use crate::pack::layout::{FieldSpec, Layout};
use proc_macro2::Ident;
use quote::quote;
use syn::ItemStruct;

// ── bit-chunk helpers ─────────────────────────────────────────────────────
struct Chunk {
    byte_idx: usize,
    bit_off: u8,
    take: u8,
    src_shift: u16,
}

fn chunks(width: u16, start_bit: usize) -> Vec<Chunk> {
    let mut v = Vec::new();
    let mut left = width;
    let mut pos = start_bit;
    let mut shift = 0u16;

    while left > 0 {
        let off = (pos % 8) as u8;
        let take = core::cmp::min(8 - off, left as u8);
        v.push(Chunk {
            byte_idx: pos / 8,
            bit_off: off,
            take,
            src_shift: shift,
        });
        pos += take as usize;
        left -= take as u16;
        shift += take as u16;
    }
    v
}

#[inline]
fn carrier_ty(width: u16) -> proc_macro2::TokenStream {
    if width <= 8 {
        quote!(u8)
    } else if width <= 16 {
        quote!(u16)
    } else {
        quote!(u32)
    }
}

#[inline]
fn is_infallible_ty(ty: &syn::Type) -> bool {
    super::is_bool(ty)
        || super::is_u8(ty)
        || super::is_u16(ty)
        || super::is_u32(ty)
        || super::is_i8(ty)
        || super::is_i16(ty)
        || super::is_i32(ty)
}

// ── layout rendering (for docs) ───────────────────────────────────────────
fn render_layout(nbytes: usize, fields: &[FieldSpec], reserved: &[bool]) -> String {
    use core::fmt::Write;

    const E: char = '◦';
    const R: char = '•';
    const PAL: &[char] = &[
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U',
        'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k', 'm', 'n', 'p',
        'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    let tb = nbytes * 8;
    let mut cell = vec![E; tb];
    let mut used = [false; 128];
    #[inline]
    fn take(u: &mut [bool; 128], c: char) -> bool {
        let i = c as usize;
        i < 128 && !core::mem::replace(&mut u[i], true)
    }
    #[inline]
    fn pick(name: &str, used: &mut [bool; 128]) -> char {
        name.chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_uppercase())
            .find(|&c| !matches!(c, 'I' | 'L' | 'O') && take(used, c))
            .or_else(|| PAL.iter().copied().find(|&c| take(used, c)))
            .unwrap_or('?')
    }
    for f in fields {
        let tag = pick(&f.ident.to_string(), &mut used);
        for i in f.start_bit..(f.start_bit + f.width as usize) {
            cell[i] = tag;
        }
    }
    for (i, &r) in reserved.iter().take(tb).enumerate() {
        if r && cell[i] == E {
            cell[i] = R;
        }
    }
    let w = (tb.saturating_sub(1)).to_string().len().max(2);
    let mut out = String::from("```\n");
    for hi in (0..nbytes).rev().step_by(2) {
        let bytes: &[usize] = if hi > 0 { &[hi, hi - 1] } else { &[hi] };
        let mut first = true;
        for &b in bytes {
            if !first {
                out.push_str("  ");
            }
            first = false;
            for bit in (0..8).rev() {
                let g = b * 8 + bit;
                let _ = write!(out, "{:>w$}", format!("{g:02}"));
                if bit != 0 {
                    out.push(' ');
                }
            }
        }
        out.push('\n');
        first = true;
        for &b in bytes {
            if !first {
                out.push_str("  ");
            }
            first = false;
            for bit in (0..8).rev() {
                let g = b * 8 + bit;
                let _ = write!(out, "{:>w$}", cell[g]);
                if bit != 0 {
                    out.push(' ');
                }
            }
        }
        out.push('\n');
    }
    out.push_str("```");
    out
}

// ── generation helpers ────────────────────────────────────────────────────
#[inline]
fn array_ty(nbytes: usize) -> proc_macro2::TokenStream {
    let n = nbytes;
    quote! { [u8; #n] }
}

fn append_layout_doc(cleaned: &mut ItemStruct, nbytes: usize, layout: &Layout) {
    use syn::parse_quote;
    cleaned.attrs.push(parse_quote!(#[doc = ""]));
    cleaned.attrs.push(parse_quote!(#[doc = "_Bit layout_"]));
    let md = render_layout(nbytes, &layout.fields, &layout.reserved);
    let attr: syn::Attribute = parse_quote!(#[doc = #md]);
    cleaned.attrs.push(attr);
}

// Build the expression that extracts a u32 “raw” value of the field from `bytes`.
fn raw_expr_for_field(f: &FieldSpec) -> proc_macro2::TokenStream {
    let terms = chunks(f.width, f.start_bit).into_iter().map(|c| {
        let i = c.byte_idx;
        let off = c.bit_off as u32;
        let take = c.take as u32;
        let sh = c.src_shift as u32;
        quote! {{
          let low: u8 = ((1u16 << #take as u16) - 1) as u8;
          (((bytes[#i] >> #off) & low) as u32) << #sh
        }}
    });
    terms
        .reduce(|a, b| quote! {(#a)|(#b)})
        .unwrap_or_else(|| quote!(0))
}

// Generate the statements that write a field’s bits into `bytes` for T -> [u8;N].
fn gen_to_bytes_stmt(f: &FieldSpec) -> proc_macro2::TokenStream {
    let name = &f.ident;
    let mask = if f.width == 32 {
        quote!(u32::MAX)
    } else {
        let w = f.width as u32;
        quote!((1u32 << #w) - 1)
    };
    let parts = chunks(f.width, f.start_bit).into_iter().map(|c| {
        let i = c.byte_idx;
        let off = c.bit_off as u32;
        let take = c.take as u32;
        let sh = c.src_shift as u32;
        quote! {{
          let low: u8 = ((1u16 << #take as u16) - 1) as u8;
          let part: u8 = ((val >> #sh) as u8) & low;
          let mask: u8 = low << #off;
          bytes[#i] = (bytes[#i] & !mask) | ((part << #off) & mask);
        }}
    });
    let ty = &f.ty;
    let is_byte_aligned = f.start_bit % 8 == 0 && f.width % 8 == 0;
    let start = f.start_bit / 8;
    if is_byte_aligned && super::is_u8(ty) && f.width == 8 {
        quote! {{ bytes[#start] = value.#name.to_le_bytes()[0]; }}
    } else if is_byte_aligned && super::is_i8(ty) && f.width == 8 {
        quote! {{ bytes[#start] = value.#name.to_le_bytes()[0]; }}
    } else if is_byte_aligned && super::is_u16(ty) && f.width == 16 {
        quote! {{
          let le = value.#name.to_le_bytes();
          bytes[#start + 0] = le[0];
          bytes[#start + 1] = le[1];
        }}
    } else if is_byte_aligned && super::is_i16(ty) && f.width == 16 {
        quote! {{
          let le = value.#name.to_le_bytes();
          bytes[#start + 0] = le[0];
          bytes[#start + 1] = le[1];
        }}
    } else if is_byte_aligned && super::is_u32(ty) && f.width == 32 {
        quote! {{
          let le = value.#name.to_le_bytes();
          bytes[#start + 0] = le[0];
          bytes[#start + 1] = le[1];
          bytes[#start + 2] = le[2];
          bytes[#start + 3] = le[3];
        }}
    } else if is_byte_aligned && super::is_i32(ty) && f.width == 32 {
        quote! {{
          let le = value.#name.to_le_bytes();
          bytes[#start + 0] = le[0];
          bytes[#start + 1] = le[1];
          bytes[#start + 2] = le[2];
          bytes[#start + 3] = le[3];
        }}
    } else if super::is_i8(ty) || super::is_i16(ty) || super::is_i32(ty) {
        quote! {{
          // Build raw two's-complement bits from signed value, then splat into bytes.
          let mut val: u32 = (value.#name as u32) & #mask;
          #(#parts)*
        }}
    } else {
        // Fallback: mask and scatter via chunk writer.
        quote! {{
          let mut val: u32 = (value.#name as u32) & #mask;
          #(#parts)*
        }}
    }
}

#[derive(Copy, Clone)]
enum FieldInitKind {
    Infallible,
    Fallible,
}

// Generate the field initializer from `bytes` for [u8;N] -> T.
fn gen_from_bytes_field(f: &FieldSpec, kind: FieldInitKind) -> proc_macro2::TokenStream {
    let name = &f.ident;
    let ty = &f.ty;
    let raw = raw_expr_for_field(f);
    let is_byte_aligned = f.start_bit % 8 == 0 && f.width % 8 == 0;
    let start = f.start_bit / 8;

    // Common primitive and aligned paths
    let common = if super::is_bool(ty) {
        quote!( #name: { let raw: u32 = #raw; raw != 0 } )
    } else if is_byte_aligned && super::is_u8(ty) && f.width == 8 {
        quote!( #name: { u8::from_le_bytes([bytes[#start]]) } )
    } else if is_byte_aligned && super::is_i8(ty) && f.width == 8 {
        quote!( #name: { i8::from_le_bytes([bytes[#start]]) } )
    } else if is_byte_aligned && super::is_u16(ty) && f.width == 16 {
        quote!( #name: { u16::from_le_bytes([bytes[#start + 0], bytes[#start + 1]]) } )
    } else if is_byte_aligned && super::is_i16(ty) && f.width == 16 {
        quote!( #name: { i16::from_le_bytes([bytes[#start + 0], bytes[#start + 1]]) } )
    } else if is_byte_aligned && super::is_u32(ty) && f.width == 32 {
        quote!( #name: { u32::from_le_bytes([bytes[#start + 0], bytes[#start + 1], bytes[#start + 2], bytes[#start + 3]]) } )
    } else if is_byte_aligned && super::is_i32(ty) && f.width == 32 {
        quote!( #name: { i32::from_le_bytes([bytes[#start + 0], bytes[#start + 1], bytes[#start + 2], bytes[#start + 3]]) } )
    } else if super::is_u8(ty) {
        quote!( #name: { let raw: u32 = #raw; u8::from_le_bytes([raw as u8]) } )
    } else if super::is_u16(ty) {
        quote!( #name: { let raw: u32 = #raw; u16::from_le_bytes([(raw >> 0) as u8, (raw >> 8) as u8]) } )
    } else if super::is_u32(ty) {
        quote!( #name: { let raw: u32 = #raw; u32::from_le_bytes([(raw >> 0) as u8, (raw >> 8) as u8, (raw >> 16) as u8, (raw >> 24) as u8]) } )
    } else if super::is_i8(ty) {
        let w = f.width as u32;
        quote!( #name: {
          let raw: u32 = #raw;
          let ext: u32 = if #w < 8 && ((raw >> (#w - 1)) & 1) != 0 { raw | (!((1u32 << #w) - 1)) } else { raw };
          i8::from_le_bytes([ext as u8])
        } )
    } else if super::is_i16(ty) {
        let w = f.width as u32;
        quote!( #name: {
          let raw: u32 = #raw;
          let ext: u32 = if #w < 16 && ((raw >> (#w - 1)) & 1) != 0 { raw | (!((1u32 << #w) - 1)) } else { raw };
          i16::from_le_bytes([(ext >> 0) as u8, (ext >> 8) as u8])
        } )
    } else if super::is_i32(ty) {
        let w = f.width as u32;
        quote!( #name: {
          let raw: u32 = #raw;
          let ext: u32 = if #w < 32 && ((raw >> (#w - 1)) & 1) != 0 { raw | (!((1u32 << #w) - 1)) } else { raw };
          i32::from_le_bytes([(ext >> 0) as u8, (ext >> 8) as u8, (ext >> 16) as u8, (ext >> 24) as u8])
        } )
    } else {
        // Defer to kind-specific fallback below
        quote!()
    };

    // If we matched any of the common branches, use them.
    if !common.is_empty() {
        return common;
    }

    // Kind-specific fallback for non-primitive fields
    match kind {
        FieldInitKind::Infallible => {
            quote!( #name: { let raw: u32 = #raw; raw } )
        }
        FieldInitKind::Fallible => {
            let u = carrier_ty(f.width);
            let fname = name.to_string();
            quote! {
              #name: <#ty as core::convert::TryFrom<#u>>::try_from((#raw) as #u)
                  .map_err(|_| concat!("pack: field `", #fname, "` conversion failed"))?
            }
        }
    }
}

#[inline]
fn fields_all_infallible(fields: &[FieldSpec]) -> bool {
    fields.iter().all(|f| is_infallible_ty(&f.ty))
}

pub(super) fn generate(
    struct_name: &Ident,
    nbytes: usize,
    layout: &Layout,
    mut cleaned: ItemStruct,
) -> proc_macro2::TokenStream {
    // Common tokens
    let array_ty = array_ty(nbytes);

    // Append Markdown doc with bit layout
    append_layout_doc(&mut cleaned, nbytes, layout);

    let fields = &layout.fields;

    // T -> [u8; N]
    let to_stmts = fields.iter().map(gen_to_bytes_stmt);

    // Infallible and fallible field builders
    let try_fields = fields
        .iter()
        .map(|f| gen_from_bytes_field(f, FieldInitKind::Fallible));
    let from_fields_infall = fields
        .iter()
        .map(|f| gen_from_bytes_field(f, FieldInitKind::Infallible));

    // Can we safely emit From<[u8; N]>?
    let all_infallible = fields_all_infallible(fields);

    // --- Emit tokens ---
    let mut ts = quote! {
      #cleaned

      // T -> [u8; N] (always)
      impl core::convert::From<#struct_name> for #array_ty {
        fn from(value: #struct_name) -> Self {
          let mut bytes: #array_ty = [0u8; #nbytes];
          #( #to_stmts )*
          bytes
        }
      }
    };

    if all_infallible {
        // [u8; N] -> T (infallible). `TryFrom` will be auto-available via the std blanket impl.
        ts.extend(quote! {
          impl core::convert::From<#array_ty> for #struct_name {
            fn from(bytes: #array_ty) -> Self {
              Self { #( #from_fields_infall, )* }
            }
          }
        });
    } else {
        // [u8; N] -> T (fallible). We DO NOT also emit `From`.
        ts.extend(quote! {
          impl core::convert::TryFrom<#array_ty> for #struct_name {
            type Error = &'static str;
            fn try_from(bytes: #array_ty) -> core::result::Result<Self, Self::Error> {
              Ok(Self { #( #try_fields, )* })
            }
          }
        });
    }

    // Convenience u8 conversions when N == 1 (mirror the same rule)
    if nbytes == 1 {
        ts.extend(quote! {
          impl core::convert::From<#struct_name> for u8 {
            fn from(value: #struct_name) -> Self {
              <#array_ty as core::convert::From<#struct_name>>::from(value)[0]
            }
          }
        });

        if all_infallible {
            ts.extend(quote! {
              impl core::convert::From<u8> for #struct_name {
                fn from(b: u8) -> Self { <#struct_name as core::convert::From<[u8;1]>>::from([b]) }
              }
            });
            // No TryFrom<u8>—it’s auto via the blanket impl because From<u8> exists.
        } else {
            ts.extend(quote! {
              impl core::convert::TryFrom<u8> for #struct_name {
                type Error = &'static str;
                fn try_from(b: u8) -> core::result::Result<Self, Self::Error> {
                  <#struct_name as core::convert::TryFrom<[u8;1]>>::try_from([b])
                }
              }
            });
        }
    }

    ts
}
