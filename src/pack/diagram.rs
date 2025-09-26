//! Markdown diagram for packed bit layouts used in generated docs.
//!
//! Simpler, more linear renderer:
//! - Group bytes into rows (top → bottom), BYTES_PER_ROW per row.
//! - Draw header and a single bracket line per row.
//! - For each field, draw its bracket spans per row and place at most one
//!   width label on the row where the field covers the most columns.
//!
//! This keeps the output stable for our tests while making the logic much
//! easier to follow and maintain.

use crate::pack::args::BitOrder;
use crate::pack::layout::FieldSpec;

// How many bytes are grouped together per rendered row.
const BYTES_PER_ROW: usize = 2;
// Minimum field width (in bits) to emit a numeric length label inside the bracket.
// Narrower runs remain unlabeled to avoid clutter.
const MIN_LABEL_BITS: usize = 2;
// Markdown code fence used to wrap the diagram.
const CODE_FENCE: &str = "```";
// Header separator between adjacent bytes (three spaces by design).
const HEADER_BYTE_SEP: &str = "   ";
// Separator between bit cells in the header (single space).
const HEADER_CELL_SEP: char = ' ';
// Characters for the bracket drawing.
const CH_DASH: char = '─';
const CH_CORNER_LEFT: char = '╰';
const CH_CORNER_RIGHT: char = '╯';
const CH_UNUSED: char = '°';
// Gap widths (number of dash columns) between cells and at a byte seam.
const GAP_WITHIN_CELL: usize = 1;
const GAP_AT_BYTE_SEAM: usize = 3;
// Padding added around the numeric label (e.g. " 24 ").
const LABEL_PAD: &str = " ";

// Label placement (row index and starting column) for a given field
#[derive(Clone, Debug)]
struct LabelPlacement {
  row_idx: usize,
  col: usize,
  text: String,
}

// Geometry helpers for a single row
struct RowCtx<'a> {
  w: usize,           // width of a bit cell in columns
  bytes: &'a [usize], // bytes included in this row (MSB row first)
  order: BitOrder,
}

impl RowCtx<'_> {
  fn write_header(&self, out: &mut String) {
    use core::fmt::Write;
    let mut first = true;
    for &b in self.bytes {
      if !first {
        out.push_str(HEADER_BYTE_SEP);
      }
      first = false;
      for bit in (0..8).rev() {
        let g = b * 8 + bit;
        let _ = write!(out, "{:>w$}", format!("{g:02}"), w = self.w);
        if bit != 0 {
          out.push(HEADER_CELL_SEP);
        }
      }
    }
    out.push('\n');
  }
}

// A field’s coverage within a single row expressed both in cell indices and
// absolute diagram columns (dash area excludes corner characters).
#[derive(Copy, Clone)]
struct RowSegment {
  a: usize,
  b: usize,
  left_cap: bool,
  right_cap: bool,
  dash_start: usize,
  dash_end: usize,
}

impl FieldSpec {
  fn row_segment(&self, ctx: &RowCtx<'_>) -> Option<RowSegment> {
    let row_len = ctx.row_len();
    if row_len == 0 {
      return None;
    }
    let (row_lo, row_hi) = ctx.bounds();
    let lo = self.start_bit;
    let hi = lo + self.width as usize - 1;
    if hi < row_lo || lo > row_hi {
      return None;
    }
    let seg_lo = lo.max(row_lo);
    let seg_hi = hi.min(row_hi);
    let a = ctx.bit_to_p(seg_hi)?;
    let b = ctx.bit_to_p(seg_lo)?;
    let left_cap = hi >= row_lo && hi <= row_hi;
    let right_cap = lo >= row_lo && lo <= row_hi;
    let dash_start = ctx.col(a) + if left_cap { 1 } else { 0 };
    let dash_end = ctx.col(b) + ctx.w - 1 - if right_cap { 1 } else { 0 };
    Some(RowSegment { a, b, left_cap, right_cap, dash_start, dash_end })
  }
}

impl<'a> RowCtx<'a> {
  fn row_len(&self) -> usize {
    self.bytes.len() * 8
  }
  fn bounds(&self) -> (usize, usize) {
    let row_hi = self.bytes.iter().copied().max().unwrap_or(0) * 8 + 7;
    let row_lo = self.bytes.iter().copied().min().unwrap_or(0) * 8;
    (row_lo, row_hi)
  }
  #[inline]
  fn col(&self, p: usize) -> usize {
    // Base advance per bit cell plus additional columns for each prior byte seam
    let seam_extra = GAP_AT_BYTE_SEAM.saturating_sub(GAP_WITHIN_CELL);
    let seam_count = p / 8; // number of seams passed before cell p
    p * (self.w + GAP_WITHIN_CELL) + seam_count * seam_extra
  }
  #[inline]
  fn gap_after(&self, p: usize) -> usize {
    let row_len = self.row_len();
    if p + 1 >= row_len {
      0
    } else if (p + 1) % 8 == 0 {
      GAP_AT_BYTE_SEAM
    } else {
      GAP_WITHIN_CELL
    }
  }
  #[inline]
  fn bit_to_p(&self, g: usize) -> Option<usize> {
    let byte = g / 8;
    let bit = g % 8; // logical within-byte bit index (LSB0 numbering)
    self
      .bytes
      .iter()
      .position(|&b| b == byte)
      .map(|j| j * 8 + self.order.display_within(bit))
  }
  #[inline]
  fn total_cols(&self) -> usize {
    let row_len = self.row_len();
    if row_len == 0 {
      return 0;
    }
    let within_gaps = row_len.saturating_sub(1) * GAP_WITHIN_CELL;
    let seam_extra = (GAP_AT_BYTE_SEAM.saturating_sub(GAP_WITHIN_CELL)) * self.bytes.len().saturating_sub(1);
    row_len * self.w + within_gaps + seam_extra
  }
}

pub(super) struct Diagram {
  order: BitOrder,
  w: usize,
  rows: Vec<Vec<usize>>, // MSB row first
  fields: Vec<FieldSpec>,
}

impl Diagram {
  pub(super) fn new(nbytes: usize, order: BitOrder, fields: &[FieldSpec]) -> Self {
    let tb = nbytes * 8;
    let w = (tb.saturating_sub(1)).to_string().len().max(2);
    let rows = if nbytes == 0 {
      Vec::new()
    } else {
      (0..nbytes)
        .rev()
        .collect::<Vec<_>>()
        .chunks(BYTES_PER_ROW)
        .map(|c| c.to_vec())
        .collect()
    };
    Self { order, w, rows, fields: fields.to_vec() }
  }

  // Choose a single row for each field to place its width label: pick the row
  // where the field covers the most dash columns and center the label within
  // that row’s dash span. Skip labels that cannot fit.
  fn plan_labels(&self) -> Vec<Option<LabelPlacement>> {
    let mut plan = vec![None; self.fields.len()];
    for (fi, f) in self.fields.iter().enumerate() {
      let label = format!("{}{}{}", LABEL_PAD, f.width, LABEL_PAD);
      let lw = label.len();
      if (f.width as usize) < MIN_LABEL_BITS || (f.width as usize) < lw {
        continue;
      }

      let mut best: Option<(usize, usize, usize)> = None; // (row_idx, dash_start, dash_end)
      for (ri, bytes) in self.rows.iter().enumerate() {
        let ctx = RowCtx { w: self.w, bytes, order: self.order };
        if let Some(seg) = f.row_segment(&ctx)
          && seg.dash_end >= seg.dash_start
        {
          let len = seg.dash_end + 1 - seg.dash_start;
          if best.as_ref().map(|&(_, s, e)| (e + 1 - s)).unwrap_or(0) < len {
            best = Some((ri, seg.dash_start, seg.dash_end));
          }
        }
      }
      if let Some((ri, s, e)) = best
        && (e + 1).saturating_sub(s) >= lw
      {
        let place = s + ((e + 1 - s) - lw) / 2;
        plan[fi] = Some(LabelPlacement { row_idx: ri, col: place, text: label });
      }
    }
    plan
  }

  fn render_row(&self, out: &mut String, row_idx: usize, labels: &[Option<LabelPlacement>]) {
    let bytes = &self.rows[row_idx];
    let ctx = RowCtx { w: self.w, bytes, order: self.order };

    // Header
    ctx.write_header(out);

    // Brackets line
    let total_cols = ctx.total_cols();
    if total_cols == 0 {
      out.push('\n');
      return;
    }
    let mut line: Vec<char> = vec![' '; total_cols];

    // draw a run inside [a..=b]
    let mut draw_run = |a: usize, b: usize, left_cap: bool, right_cap: bool| {
      if a > b {
        return;
      }
      if left_cap {
        line[ctx.col(a)] = CH_CORNER_LEFT;
      }
      for p in a..=b {
        let start = ctx.col(p);
        let end = start + ctx.w;
        let from = if p == a && left_cap { start + 1 } else { start };
        let to = if p == b {
          if right_cap { end - 1 } else { end }
        } else {
          end
        };
        for ch in &mut line[from..to] {
          *ch = CH_DASH;
        }
        if p < b {
          let g = ctx.gap_after(p);
          for ch in &mut line[end..(end + g)] {
            *ch = CH_DASH;
          }
        }
      }
      if right_cap {
        let rb = ctx.col(b) + ctx.w - 1;
        line[rb] = CH_CORNER_RIGHT;
      }
    };

    for f in &self.fields {
      if let Some(seg) = f.row_segment(&ctx) {
        draw_run(seg.a, seg.b, seg.left_cap, seg.right_cap);
      }
    }

    // Overlay labels for this row
    for LabelPlacement { row_idx: r, col, text } in labels.iter().flatten() {
      if *r == row_idx {
        for (i, ch) in text.chars().enumerate() {
          let idx = *col + i;
          if idx < line.len() {
            line[idx] = ch;
          }
        }
      }
    }

    // Bullets in empty cells
    for p in 0..ctx.row_len() {
      let cs = ctx.col(p);
      if !line[cs..(cs + ctx.w)].iter().any(|&c| c != ' ') {
        line[cs + ctx.w - 1] = CH_UNUSED;
      }
    }

    for ch in line {
      out.push(ch);
    }
    out.push('\n');
  }

  pub(super) fn render(&self) -> String {
    let mut out = String::new();
    out.push_str(CODE_FENCE);
    out.push_str("no_run\n");
    let labels = self.plan_labels();
    for row_idx in 0..self.rows.len() {
      self.render_row(&mut out, row_idx, &labels);
    }
    out.push_str(CODE_FENCE);
    out
  }
}

#[cfg(test)]
mod tests {
  use super::Diagram;
  use crate::pack::args::BitOrder;
  use crate::pack::layout::{FieldKind, FieldSpec};
  use quote::format_ident;
  use syn::parse_quote;

  fn fs(name: &str, ty: syn::Type, width: u16, start_bit: usize) -> FieldSpec {
    FieldSpec { ident: format_ident!("{}", name), ty: ty.clone(), width, start_bit, kind: FieldKind::from_type(&ty) }
  }

  fn bracket_lines(md: &str) -> Vec<&str> {
    let lines: Vec<&str> = md.lines().collect();
    assert!(lines.first().map(|s| s.starts_with(super::CODE_FENCE)).unwrap_or(false), "diagram starts with code fence");
    assert!(lines.last() == Some(&super::CODE_FENCE), "diagram ends with code fence");
    // After skipping first line and enumerating: even indices are bracket lines (2,4,6,...)
    lines
      .iter()
      .enumerate()
      .take(lines.len() - 1)
      .skip(1)
      .filter(|(i, _)| i % 2 == 0)
      .map(|(_, &s)| s)
      .collect()
  }

  #[test]
  fn label_full_byte_single_row() {
    // One byte, one field covering 8 bits → label "8" appears once on bracket line
    let fields = vec![fs("x", parse_quote!(u8), 8, 0)];
    let md = Diagram::new(1, BitOrder::Lsb0, &fields).render();
    let bl = bracket_lines(&md);
    assert_eq!(bl.len(), 1);
    let line = bl[0];
    assert!(line.contains(super::CH_CORNER_LEFT) && line.contains(super::CH_CORNER_RIGHT));
    let count_8 = line.matches('8').count();
    assert_eq!(count_8, 1, "expected one centered '8' label in bracket line: {}", line);
  }

  #[test]
  fn label_spans_rows_is_total_width() {
    // Four bytes form two rows: [3,2], [1,0]. Field spans bytes 1..3 (24 bits).
    // The label should be "24" and appear exactly once among bracket lines.
    let fields = vec![fs("w", parse_quote!(u32), 24, 8)]; // bits 8..31
    let md = Diagram::new(4, BitOrder::Lsb0, &fields).render();
    let bl = bracket_lines(&md);
    assert_eq!(bl.len(), 2);
    let total = bl.iter().map(|l| l.matches("24").count()).sum::<usize>();
    assert_eq!(total, 1, "label should be total width and appear once across rows\n{}", md);
    // Corners should appear across rows: left cap on top row, right cap on bottom row
    assert!(bl[0].contains(super::CH_CORNER_LEFT));
    assert!(bl[1].contains(super::CH_CORNER_RIGHT));
  }

  #[test]
  fn bullets_mark_unused_cells() {
    // Two bytes, field uses only 1 bit at LSB → expect bullets for unused cells
    let fields = vec![fs("b", parse_quote!(u8), 1, 0)];
    let md = Diagram::new(2, BitOrder::Lsb0, &fields).render();
    let bl = bracket_lines(&md);
    assert_eq!(bl.len(), 1);
    assert!(bl[0].contains(super::CH_UNUSED), "expected bullets in unused cells: {}", bl[0]);
  }

  fn header_lines(md: &str) -> Vec<&str> {
    let lines: Vec<&str> = md.lines().collect();
    assert!(lines.first().map(|s| s.starts_with(super::CODE_FENCE)).unwrap_or(false));
    assert!(lines.last() == Some(&super::CODE_FENCE));
    lines
      .iter()
      .enumerate()
      .take(lines.len() - 1)
      .skip(1)
      .filter(|(i, _)| i % 2 == 1)
      .map(|(_, &s)| s)
      .collect()
  }

  fn expected_header_for_row(bytes: &[usize], w: usize) -> String {
    let mut s = String::new();
    let mut first = true;
    for &b in bytes {
      if !first {
        s.push_str(super::HEADER_BYTE_SEP);
      }
      first = false;
      for bit in (0..8).rev() {
        let g = b * 8 + bit;
        let cell = format!("{g:02}");
        s.push_str(&format!("{:>width$}", cell, width = w));
        if bit != 0 {
          s.push(super::HEADER_CELL_SEP);
        }
      }
    }
    s
  }

  #[test]
  fn bit_index_headers_align() {
    // 4 bytes → two rows [3,2] and [1,0]. Ensure headers are exact.
    let md = Diagram::new(4, BitOrder::Lsb0, &[]).render();
    let hs = header_lines(&md);
    assert_eq!(hs.len(), 2);
    let w = (32usize.saturating_sub(1)).to_string().len().max(2); // matches render_layout
    let top = expected_header_for_row(&[3, 2], w);
    let bot = expected_header_for_row(&[1, 0], w);
    assert_eq!(hs[0], top, "top header misaligned\n{}", md);
    assert_eq!(hs[1], bot, "bottom header misaligned\n{}", md);
  }
}
