//! Parsing of `.lcd` `LC Raw Data/Chromatogram ChN` streams - a
//! conventional LC detector channel (UV/RID-style), structurally
//! unrelated to `PDA 3D Raw Data` despite sharing the same outer
//! `RC\x00\x00` segment header.
//!
//! See `docs/format/04-lcd-chromatogram-pda.md`'s "LC Raw Data
//! Chromatogram Ch5/Ch6 decode" session (Sigilweaver/OpenSZRaw#21) for the
//! full derivation and evidence this implements.

use byteorder::{ByteOrder, LittleEndian};

/// Magic for the 24-byte `RC\x00\x00` segment header shared by
/// `PDA 3D Raw Data` and `LC Raw Data` streams (see
/// `docs/format/04-lcd-chromatogram-pda.md`'s "Segment Header" section).
const SEGMENT_MAGIC: u32 = 17234;
const SEGMENT_HEADER_SIZE: usize = 24;

/// Byte value at/above which a sample token widens from 1 byte to 2
/// bytes. The unique zero-exception result of an exhaustive
/// `(threshold in 0..=255) x (wide_width in 2..=4)` sweep requiring every
/// page of a stream to decode to exactly its declared point count with no
/// leftover bytes - see docs/format/04. Confirmed with zero exceptions
/// across every page of `Chromatogram Ch5`/`Ch6` in all 5 locally
/// available `PXD020792` files.
const WIDE_TOKEN_THRESHOLD: u8 = 0x20;

/// Nominal LC channel sample interval, in seconds (2 Hz). Derived from
/// cross-referencing the fixed 7200-sample stream length against
/// `TTFL Raw Data/Retention Time`'s own max value (consistently just
/// under, never over, 3600s / 60 minutes across all 5 corpus files) - see
/// docs/format/04. Not itself read from any field in the `LC Raw Data`
/// stream; a corpus-derived constant, like `docs/format/03`'s RLE
/// decoder constants.
pub const SAMPLE_INTERVAL_SEC: f64 = 0.5;

/// One decoded `LC Raw Data/Chromatogram ChN` stream: a dense,
/// evenly-spaced (RT, intensity) time series.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LcChromatogram {
    pub time_sec: Vec<f32>,
    pub intensity: Vec<f32>,
}

/// Split a `LC Raw Data/Chromatogram ChN` payload into its internal
/// "pages": each page is a `u16` LE byte-length prefix, that many data
/// bytes, then a `u16` LE suffix equal to the prefix - back to back, no
/// gap between one page's suffix and the next page's prefix. This is the
/// internal sub-segment structure the PDA payload doc's "LC Raw Data"
/// section flagged as not yet characterized: the whole stream is one
/// giant `RC\x00\x00` segment (`u32[3]` spans the entire rest of the
/// stream), but *within* that segment's body, this length-prefix/suffix
/// framing repeats - the same "first/last length word" wrapper style
/// already documented for the PDA payload's "symmetric" envelope form,
/// just applied per-page rather than per-segment. Verified zero-exception
/// (every page's prefix equals its suffix, every stream fully consumed)
/// against every one of the 5 `PXD020792` corpus files' `Ch5` and `Ch6`
/// streams (290 pages total).
///
/// Returns `None` if the payload is not an exact, zero-leftover sequence
/// of such pages.
fn parse_pages(payload: &[u8]) -> Option<Vec<&[u8]>> {
    let mut pages = Vec::new();
    let mut off = 0usize;
    let n = payload.len();
    while off < n {
        if off + 2 > n {
            return None;
        }
        let plen = LittleEndian::read_u16(&payload[off..off + 2]) as usize;
        let start = off + 2;
        let end = start.checked_add(plen)?;
        if end + 2 > n {
            return None;
        }
        let suffix = LittleEndian::read_u16(&payload[end..end + 2]) as usize;
        if suffix != plen {
            return None;
        }
        pages.push(&payload[start..end]);
        off = end + 2;
    }
    Some(pages)
}

/// Decode one page's bytes into signed per-sample deltas.
///
/// Tokenization: a byte `b < 0x20` is a 1-byte literal, read as a signed
/// 5-bit two's-complement value (`0..=15` -> `+0..=+15`, `16..=31` ->
/// `-16..=-1`). A byte `b >= 0x20` (`WIDE_TOKEN_THRESHOLD`) starts a
/// 2-byte "wide" token: `b`'s low 5 bits become the high bits of a signed
/// 13-bit value, the following byte the low 8 bits. This
/// `(threshold, width) = (0x20, 2)` rule is the *only* combination (of an
/// exhaustive 256x3 sweep) that decodes every page of every locally
/// available `Ch5`/`Ch6` stream to exactly its expected sample count with
/// zero leftover bytes - a fully verified structural fact.
///
/// The signed-value interpretation (5-bit literal, 13-bit wide) is a
/// strong but not byte-exact-certain hypothesis: cumulative-summing the
/// decoded deltas produces a smooth, low-magnitude, physically plausible
/// chromatogram trace (baseline near zero, a single broad rise, then a
/// stable plateau of similar absolute magnitude) consistently across all
/// 5 corpus files, clearly outperforming (by an order of magnitude in
/// mean per-sample delta size) every alternative byte-order/bit-width
/// layout tried - see docs/format/04 for the comparison. This is a soft,
/// physical-plausibility validator (the kind docs/format/04's own
/// "Further avenues" section recommends), not a byte-exact proof.
fn decode_page_tokens(page: &[u8]) -> Option<Vec<i32>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let n = page.len();
    while i < n {
        let b = page[i];
        if b >= WIDE_TOKEN_THRESHOLD {
            if i + 1 >= n {
                return None; // truncated wide token
            }
            let b1 = page[i + 1];
            let raw = (i32::from(b & 0x1F) << 8) | i32::from(b1);
            let signed = if raw >= 4096 { raw - 8192 } else { raw };
            out.push(signed);
            i += 2;
        } else {
            let signed = if b < 16 {
                i32::from(b)
            } else {
                i32::from(b) - 32
            };
            out.push(signed);
            i += 1;
        }
    }
    Some(out)
}

/// Decode a full `LC Raw Data/Chromatogram ChN` stream.
///
/// Returns `None` when:
/// - the stream is too short or its 24-byte header magic doesn't match
///   (not a `RC\x00\x00` segment at all);
/// - the page framing or tokenization doesn't cleanly consume the whole
///   payload (a malformed or differently-encoded stream);
/// - the decoded sample count doesn't match the segment header's own
///   declared point count (`u32[2]`);
/// - **every decoded delta is identical** (fewer than 2 distinct decoded
///   token values across the whole stream). This is deliberate, not a
///   parsing failure: every locally available file's `Chromatogram Ch5`
///   decodes cleanly under the rule above but is a single repeated token
///   for its entire ~1-hour run, which makes it impossible to tell
///   whether the delta+cumulative-sum interpretation established from
///   `Ch6`'s real, varying signal actually applies to it, or whether it
///   is some other convention (e.g. a plain absolute reading) that just
///   happens to look identical when the value never changes - see
///   docs/format/04. Emitting a delta-integrated trace for such a stream
///   risks shipping a fabricated, possibly-wrong unbounded ramp instead
///   of the flat channel the source data almost certainly represents;
///   skipping it is the honest choice per `CONTRIBUTING.md`'s clean-room
///   policy. A future file whose `Ch5` (or any other channel) actually
///   varies would decode normally under this same rule.
pub fn decode_stream(data: &[u8]) -> Option<LcChromatogram> {
    if data.len() < SEGMENT_HEADER_SIZE {
        return None;
    }
    let magic = LittleEndian::read_u32(&data[0..4]);
    if magic != SEGMENT_MAGIC {
        return None;
    }
    let npts = LittleEndian::read_u32(&data[8..12]) as usize;
    let blocksz = LittleEndian::read_u32(&data[12..16]) as usize;
    if blocksz < SEGMENT_HEADER_SIZE || blocksz > data.len() {
        return None;
    }
    let payload = &data[SEGMENT_HEADER_SIZE..blocksz];
    let pages = parse_pages(payload)?;

    let mut cumulative: i64 = 0;
    let mut time_sec = Vec::with_capacity(npts);
    let mut intensity = Vec::with_capacity(npts);
    let mut distinct_deltas = std::collections::HashSet::new();
    let mut sample_idx: usize = 0;
    for page in pages {
        let deltas = decode_page_tokens(page)?;
        for d in deltas {
            distinct_deltas.insert(d);
            cumulative += i64::from(d);
            #[allow(clippy::cast_precision_loss)]
            let t = sample_idx as f64 * SAMPLE_INTERVAL_SEC;
            time_sec.push(t as f32);
            #[allow(clippy::cast_precision_loss)]
            intensity.push(cumulative as f32);
            sample_idx += 1;
        }
    }
    if sample_idx != npts {
        return None; // didn't consume exactly the declared point count
    }
    if distinct_deltas.len() < 2 {
        return None; // single repeated value: grammar untestable, see doc comment above
    }
    Some(LcChromatogram {
        time_sec,
        intensity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_header(buf: &mut Vec<u8>, npts: u32, blocksz: u32) {
        buf.extend_from_slice(&SEGMENT_MAGIC.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // version
        buf.extend_from_slice(&npts.to_le_bytes());
        buf.extend_from_slice(&blocksz.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // padding
        buf.extend_from_slice(&0u32.to_le_bytes()); // padding
    }

    fn write_page(buf: &mut Vec<u8>, page_bytes: &[u8]) {
        let len = page_bytes.len() as u16;
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(page_bytes);
        buf.extend_from_slice(&len.to_le_bytes());
    }

    #[test]
    fn parse_pages_splits_back_to_back_length_wrapped_pages() {
        let mut payload = Vec::new();
        write_page(&mut payload, &[1, 2, 3]);
        write_page(&mut payload, &[4, 5]);
        let pages = parse_pages(&payload).expect("parse");
        assert_eq!(pages, vec![&[1u8, 2, 3][..], &[4u8, 5][..]]);
    }

    #[test]
    fn parse_pages_rejects_mismatched_suffix() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&3u16.to_le_bytes());
        payload.extend_from_slice(&[1, 2, 3]);
        payload.extend_from_slice(&4u16.to_le_bytes()); // wrong suffix
        assert!(parse_pages(&payload).is_none());
    }

    #[test]
    fn decode_page_tokens_reads_signed_5bit_literals() {
        // 0 -> 0, 1 -> +1, 15 -> +15, 16 -> -16, 31 -> -1.
        let page = [0u8, 1, 15, 16, 31];
        let out = decode_page_tokens(&page).expect("decode");
        assert_eq!(out, vec![0, 1, 15, -16, -1]);
    }

    #[test]
    fn decode_page_tokens_reads_signed_13bit_wide_tokens() {
        // b=0x20 (low 5 bits 0), b1=0x01 -> raw 1 -> +1.
        // b=0x3F (low 5 bits 0x1F), b1=0xFF -> raw 0x1FFF (8191) -> -1.
        let page = [0x20u8, 0x01, 0x3F, 0xFF];
        let out = decode_page_tokens(&page).expect("decode");
        assert_eq!(out, vec![1, -1]);
    }

    #[test]
    fn decode_page_tokens_rejects_truncated_wide_token() {
        let page = [0x20u8];
        assert!(decode_page_tokens(&page).is_none());
    }

    #[test]
    fn decode_stream_rejects_bad_magic() {
        let mut data = vec![0u8; SEGMENT_HEADER_SIZE];
        LittleEndian::write_u32(&mut data[0..4], 0xDEAD_BEEF);
        assert!(decode_stream(&data).is_none());
    }

    #[test]
    fn decode_stream_rejects_too_short_input() {
        assert!(decode_stream(&[0u8; 4]).is_none());
    }

    #[test]
    fn decode_stream_skips_a_single_valued_stream() {
        // Reproduces the real Ch5 signature: every sample is the same
        // wide token (here, delta +1 repeated) for the whole stream -
        // grammar untestable, must be skipped rather than integrated
        // into a fabricated ramp.
        let mut payload = Vec::new();
        let page: Vec<u8> = std::iter::repeat_n([0x20u8, 0x01], 4).flatten().collect();
        write_page(&mut payload, &page);

        let mut data = Vec::new();
        let blocksz = (SEGMENT_HEADER_SIZE + payload.len()) as u32;
        write_header(&mut data, 4, blocksz);
        data.extend_from_slice(&payload);

        assert!(decode_stream(&data).is_none());
    }

    #[test]
    fn decode_stream_round_trips_a_varying_two_page_stream() {
        // Page 1: deltas +2, +2, -1 (all literal). Page 2: deltas +1,
        // wide +100. Total 5 samples, npts must match.
        let mut payload = Vec::new();
        write_page(&mut payload, &[2, 2, 31]); // +2, +2, -1
        let wide = 100i32;
        let b0 = 0x20 | ((wide >> 8) as u8 & 0x1F);
        let b1 = (wide & 0xFF) as u8;
        write_page(&mut payload, &[1, b0, b1]); // +1, +100

        let mut data = Vec::new();
        let blocksz = (SEGMENT_HEADER_SIZE + payload.len()) as u32;
        write_header(&mut data, 5, blocksz);
        data.extend_from_slice(&payload);

        let chrom = decode_stream(&data).expect("decode");
        assert_eq!(chrom.intensity.len(), 5);
        assert_eq!(chrom.time_sec.len(), 5);
        // cumulative sum: 2, 4, 3, 4, 104
        assert_eq!(chrom.intensity, vec![2.0, 4.0, 3.0, 4.0, 104.0]);
        assert_eq!(chrom.time_sec[1], SAMPLE_INTERVAL_SEC as f32);
        assert_eq!(chrom.time_sec[4], 4.0 * SAMPLE_INTERVAL_SEC as f32);
    }

    #[test]
    fn decode_stream_rejects_point_count_mismatch() {
        let mut payload = Vec::new();
        write_page(&mut payload, &[2, 2, 31]);

        let mut data = Vec::new();
        let blocksz = (SEGMENT_HEADER_SIZE + payload.len()) as u32;
        // Declare 5 points but only 3 are present.
        write_header(&mut data, 5, blocksz);
        data.extend_from_slice(&payload);

        assert!(decode_stream(&data).is_none());
    }
}
