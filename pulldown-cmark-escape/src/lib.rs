// Copyright 2015 Google Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! Utility functions for HTML escaping. Only useful when building your own
//! HTML renderer.
#![warn(
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use alloc::string::String;

use core::fmt::{self, Arguments};
use core::str::from_utf8;
#[cfg(feature = "std")]
use std::io::{self, Write};

#[rustfmt::skip]
static HREF_SAFE: [u8; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0,
];

static HEX_CHARS: &[u8] = b"0123456789ABCDEF";
static AMP_ESCAPE: &str = "&amp;";
static SINGLE_QUOTE_ESCAPE: &str = "&#x27;";

/// This wrapper exists because we can't have both a blanket implementation
/// for all types implementing `Write` and types of the for `&mut W` where
/// `W: StrWrite`. Since we need the latter a lot, we choose to wrap
/// `Write` types.
#[derive(Debug)]
#[cfg(feature = "std")]
pub struct IoWriter<W>(pub W);

/// Trait that allows writing string slices. This is basically an extension
/// of `std::io::Write` in order to include `String`.
pub trait StrWrite {
    type Error;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error>;
    fn write_fmt(&mut self, args: Arguments) -> Result<(), Self::Error>;
}

#[cfg(feature = "std")]
impl<W> StrWrite for IoWriter<W>
where
    W: Write,
{
    type Error = io::Error;

    #[inline]
    fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.0.write_all(s.as_bytes())
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments) -> io::Result<()> {
        self.0.write_fmt(args)
    }
}

/// This wrapper exists because we can't have both a blanket implementation
/// for all types implementing `io::Write` and types of the form `&mut W` where
/// `W: StrWrite`. Since we need the latter a lot, we choose to wrap
/// `Write` types.
#[derive(Debug)]
pub struct FmtWriter<W>(pub W);

impl<W> StrWrite for FmtWriter<W>
where
    W: fmt::Write,
{
    type Error = fmt::Error;

    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments) -> fmt::Result {
        self.0.write_fmt(args)
    }
}

impl StrWrite for String {
    type Error = fmt::Error;

    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments) -> fmt::Result {
        fmt::Write::write_fmt(self, args)
    }
}

impl<W> StrWrite for &'_ mut W
where
    W: StrWrite,
{
    type Error = W::Error;

    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        (**self).write_str(s)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments) -> Result<(), Self::Error> {
        (**self).write_fmt(args)
    }
}

/// Writes an href to the buffer, escaping href unsafe bytes.
pub fn escape_href<W>(mut w: W, s: &str) -> Result<(), W::Error>
where
    W: StrWrite,
{
    let bytes = s.as_bytes();
    let mut mark = 0;
    for i in 0..bytes.len() {
        let c = bytes[i];
        if c >= 0x80 || HREF_SAFE[c as usize] == 0 {
            // character needing escape

            // write partial substring up to mark
            if mark < i {
                w.write_str(&s[mark..i])?;
            }
            match c {
                b'&' => {
                    w.write_str(AMP_ESCAPE)?;
                }
                b'\'' => {
                    w.write_str(SINGLE_QUOTE_ESCAPE)?;
                }
                _ => {
                    let mut buf = [0u8; 3];
                    buf[0] = b'%';
                    buf[1] = HEX_CHARS[((c as usize) >> 4) & 0xF];
                    buf[2] = HEX_CHARS[(c as usize) & 0xF];
                    let escaped = from_utf8(&buf).unwrap();
                    w.write_str(escaped)?;
                }
            }
            mark = i + 1; // all escaped characters are ASCII
        }
    }
    w.write_str(&s[mark..])
}

const fn create_html_escape_table(body: bool) -> [u8; 256] {
    let mut table = [0; 256];
    table[b'&' as usize] = 1;
    table[b'<' as usize] = 2;
    table[b'>' as usize] = 3;
    if !body {
        table[b'"' as usize] = 4;
        table[b'\'' as usize] = 5;
    }
    table
}

static HTML_ESCAPE_TABLE: [u8; 256] = create_html_escape_table(false);
static HTML_BODY_TEXT_ESCAPE_TABLE: [u8; 256] = create_html_escape_table(true);

static HTML_ESCAPES: [&str; 6] = ["", "&amp;", "&lt;", "&gt;", "&quot;", "&#39;"];

/// Writes the given string to the Write sink, replacing special HTML bytes
/// (<, >, &, ", ') by escape sequences.
///
/// Use this function to write output to quoted HTML attributes.
/// Since this function doesn't escape spaces, unquoted attributes
/// cannot be used. For example:
///
/// ```rust
/// let mut value = String::new();
/// pulldown_cmark_escape::escape_html(&mut value, "two words")
///     .expect("writing to a string is infallible");
/// // This is okay.
/// let ok = format!("<a title='{value}'>test</a>");
/// // This is not okay.
/// //let not_ok = format!("<a title={value}>test</a>");
/// ````
pub fn escape_html<W: StrWrite>(w: W, s: &str) -> Result<(), W::Error> {
    #[cfg(feature = "simd")]
    {
        simd::escape_html(w, s, &HTML_ESCAPE_TABLE)
    }
    #[cfg(not(feature = "simd"))]
    {
        escape_html_scalar(w, s, &HTML_ESCAPE_TABLE)
    }
}

/// For use in HTML body text, writes the given string to the Write sink,
/// replacing special HTML bytes (<, >, &) by escape sequences.
///
/// <div class="warning">
///
/// This function should be used for escaping text nodes, not attributes.
/// In the below example, the word "foo" is an attribute, and the word
/// "bar" is an text node. The word "bar" could be escaped by this function,
/// but the word "foo" must be escaped using [`escape_html`].
///
/// ```html
/// <span class="foo">bar</span>
/// ```
///
/// If you aren't sure what the difference is, use [`escape_html`].
/// It should always be correct, but will produce larger output.
///
/// </div>
pub fn escape_html_body_text<W: StrWrite>(w: W, s: &str) -> Result<(), W::Error> {
    #[cfg(feature = "simd")]
    {
        simd::escape_html(w, s, &HTML_BODY_TEXT_ESCAPE_TABLE)
    }
    #[cfg(not(feature = "simd"))]
    {
        escape_html_scalar(w, s, &HTML_BODY_TEXT_ESCAPE_TABLE)
    }
}

fn escape_html_scalar<W: StrWrite>(
    mut w: W,
    s: &str,
    table: &'static [u8; 256],
) -> Result<(), W::Error> {
    let bytes = s.as_bytes();
    let mut mark = 0;
    let mut i = 0;
    while i < s.len() {
        match bytes[i..].iter().position(|&c| table[c as usize] != 0) {
            Some(pos) => {
                i += pos;
            }
            None => break,
        }
        let c = bytes[i];
        let escape = table[c as usize];
        let escape_seq = HTML_ESCAPES[escape as usize];
        w.write_str(&s[mark..i])?;
        w.write_str(escape_seq)?;
        i += 1;
        mark = i; // all escaped characters are ASCII
    }
    w.write_str(&s[mark..])
}

#[cfg(feature = "simd")]
mod simd {
    use super::StrWrite;
    use fearless_simd::{Level, Simd, SimdBase, dispatch, mask8x16, u8x16};

    const VECTOR_SIZE: usize = 16;

    pub(super) fn escape_html<W: StrWrite>(
        w: W,
        s: &str,
        table: &'static [u8; 256],
    ) -> Result<(), W::Error> {
        let level = Level::new();
        dispatch!(level, simd => escape_html_impl(simd, w, s, table))
    }

    #[inline(always)]
    fn escape_html_impl<S: Simd, W: StrWrite>(
        simd: S,
        mut w: W,
        s: &str,
        table: &'static [u8; 256],
    ) -> Result<(), W::Error> {
        let bytes = s.as_bytes();

        // Fall back to scalar code if the buffer is shorter than one vector.
        if s.len() < VECTOR_SIZE {
            return super::escape_html_scalar(w, s, table);
        }

        let mut offset = 0;
        let mut mark = 0;
        let upperbound = bytes.len() - VECTOR_SIZE;

        // The strategy here is to walk the byte buffer in chunks of VECTOR_SIZE (16)
        // bytes at a time starting at the given offset. For each chunk, we compute a
        // a bitmask indicating whether the corresponding byte is a HTML special byte.
        // We then iterate over all the 1 bits in this mask and call the callback function
        // with the corresponding index in the buffer.
        // When the number of HTML special bytes in the buffer is relatively low, this
        // allows us to quickly go through the buffer without a lookup and for every
        // single byte.

        // Process full chunks of VECTOR_SIZE bytes.
        while offset < upperbound {
            let chunk = u8x16::from_slice(simd, &bytes[offset..offset + VECTOR_SIZE]);
            let mask = detect_special_bytes(simd, chunk);
            let mask_arr: &[i8; 16] = simd.as_array_ref_mask8x16(&mask);
            for i in 0..VECTOR_SIZE {
                if mask_arr[i] == -1 {
                    let escape_ix = bytes[offset + i] as usize;
                    let entry = table[escape_ix] as usize;
                    w.write_str(&s[mark..offset + i])?;
                    mark = offset + i + 1; // all escaped characters are ASCII
                    if entry == 0 {
                        w.write_str(&s[offset + i..mark])?;
                    } else {
                        let replacement = super::HTML_ESCAPES[entry];
                        w.write_str(replacement)?;
                    }
                }
            }
            offset += VECTOR_SIZE;
        }

        // Final iteration. We align the read with the end of the slice and
        // shift off the bytes at start we have already scanned.
        let chunk = u8x16::from_slice(simd, &bytes[upperbound..]);
        let mask = detect_special_bytes(simd, chunk);
        let mask_arr: &[i8; 16] = simd.as_array_ref_mask8x16(&mask);
        let skip = offset - upperbound;
        for i in skip..VECTOR_SIZE {
            if mask_arr[i] == -1 {
                let escape_ix = bytes[upperbound + i] as usize;
                let entry = table[escape_ix] as usize;
                w.write_str(&s[mark..upperbound + i])?;
                mark = upperbound + i + 1; // all escaped characters are ASCII
                if entry == 0 {
                    w.write_str(&s[upperbound + i..mark])?;
                } else {
                    let replacement = super::HTML_ESCAPES[entry];
                    w.write_str(replacement)?;
                }
            }
        }

        w.write_str(&s[mark..])
    }

    /// Detect which bytes in a 16-byte chunk are HTML special bytes.
    ///
    /// Returns a mask where each all-ones lane (`-1` as `i8`) indicates a
    /// special byte (`&`, `<`, `>`, `"`, `'`). Uses only `Simd` trait
    /// methods for portability across architectures.
    #[inline(always)]
    fn detect_special_bytes<S: Simd>(simd: S, chunk: u8x16<S>) -> mask8x16<S> {
        // Compare the chunk against each of the 5 HTML special bytes.
        let amp = simd.simd_eq_u8x16(chunk, simd.splat_u8x16(b'&'));
        let lt = simd.simd_eq_u8x16(chunk, simd.splat_u8x16(b'<'));
        let gt = simd.simd_eq_u8x16(chunk, simd.splat_u8x16(b'>'));
        let quot = simd.simd_eq_u8x16(chunk, simd.splat_u8x16(b'"'));
        let apos = simd.simd_eq_u8x16(chunk, simd.splat_u8x16(b'\''));

        // Combine all comparison masks with bitwise OR.
        let m1 = simd.or_mask8x16(amp, lt);
        let m2 = simd.or_mask8x16(gt, quot);
        let m3 = simd.or_mask8x16(m1, m2);
        simd.or_mask8x16(m3, apos)
    }
}

#[cfg(test)]
mod test {
    use alloc::string::String;

    pub use super::{escape_href, escape_html, escape_html_body_text};

    #[test]
    fn check_href_escape() {
        let mut s = String::new();
        escape_href(&mut s, "&^_").unwrap();
        assert_eq!(s.as_str(), "&amp;^_");
    }

    #[test]
    #[cfg(feature = "simd")]
    fn check_attr_escape() {
        let mut s = String::new();
        escape_html(&mut s, r##"&^"'_&^"'_&^"'_&^"'_&^"'_&^"'_&^"'_&^"'_&^"'_"##).unwrap();
        assert_eq!(
            s.as_str(),
            "&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_&amp;^&quot;&#39;_"
        );
    }

    #[test]
    fn check_body_escape() {
        let mut s = String::new();
        escape_html_body_text(&mut s, r##"&^"'_"##).unwrap();
        assert_eq!(s.as_str(), r##"&amp;^"'_"##);
    }
}
