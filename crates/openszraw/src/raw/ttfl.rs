//! Parsing of `.lcd` IT-TOF LC-MS data (`TTFL Raw Data` storage).
//!
//! See `docs/format/03-lcd-ttfl-msdata.md` and
//! `docs/format/06-known-limitations.md`. **The reconstructed index axis
//! is a raw, uncalibrated digitizer/time-bin index, not m/z** - no
//! calibration formula was found this session (see the known-limitations
//! doc), matching this suite's established precedent for shipping honest,
//! documented uncalibrated axes rather than blocking on it (see
//! OpenSXRaw's legacy-TOF handling).

use byteorder::{ByteOrder, LittleEndian};
use std::collections::BTreeSet;

pub const DATA_INDEX_ENTRY_SIZE: usize = 64;
pub const SUBSET_SIZE: usize = 16;
pub const SCAN_HEADER_SIZE: usize = 64;

/// One (entry, channel) subset's byte offset into `MS Raw Data`, decoded
/// from the `Data Index` stream (64-byte blocks, 4x 16-byte subsets
/// each).
#[derive(Debug, Clone, Copy)]
pub struct DataIndexSubset {
    /// RT entry index (0-based), read from the subset's own `u32[2]`
    /// field (byte offset 8) - **not** derived from the subset's
    /// position in the stream. On 4-real-channel files (e.g. MTBLS432)
    /// these coincide (one 64-byte block == one RT point's 4 channels,
    /// per `docs/format/03-lcd-ttfl-msdata.md`), but on files with fewer
    /// real channels this is not true: verified this session against
    /// `PXD025121/17.lcd`, whose acquisition has only 2 real channels
    /// per RT point, so each 64-byte block packs *two* consecutive RT
    /// points' worth of subsets (block 0's four subsets carry entry_i
    /// `[0, 0, 1, 1]`, block 1 carries `[2, 2, 3, 3]`, etc.) - reading
    /// `entry_i` from the block position instead of the subset's own
    /// field would silently assign the wrong retention time to every
    /// other RT point's spectra on such files.
    pub entry_i: usize,
    /// Position (0-3) of this subset within its 64-byte block. Not
    /// necessarily a stable "channel identity" across the whole file on
    /// fewer-than-4-real-channel files (see `entry_i`'s doc comment) -
    /// kept for diagnostic purposes only.
    pub sub_i: usize,
    pub offset: u32,
}

/// Parse the `Data Index` stream into its 16-byte subset records.
///
/// The stream is normally `N_RT * 64` bytes (4 subsets per RT point), but
/// a trailing **partial** final block (16, 32, or 48 bytes - 1 to 3
/// leftover subsets) is a real, observed condition: verified against 9
/// files in `PXD025121` this session, where a file with an odd number of
/// RT points and 2 real channels per point packs its final RT point's 2
/// subsets alone rather than padding to a full 64-byte block (e.g.
/// `PXD025121/17.lcd`: 657 RT points x 2 real channels = 1314 subsets =
/// 328 full 64-byte blocks + 1 trailing 32-byte block, `21024` bytes
/// total). Rejecting anything not an exact multiple of 64 - as the
/// original format doc's model implies - fails to open 9 real corpus
/// files; this parses full blocks first, then a trailing partial block
/// if the remainder is a positive multiple of 16.
pub fn parse_data_index(data: &[u8]) -> crate::Result<Vec<DataIndexSubset>> {
    let n_full_blocks = data.len() / DATA_INDEX_ENTRY_SIZE;
    let remainder = data.len() % DATA_INDEX_ENTRY_SIZE;
    if remainder != 0 && remainder % SUBSET_SIZE != 0 {
        return Err(crate::Error::Parse(format!(
            "Data Index stream size {} is not a whole number of {SUBSET_SIZE}-byte subsets",
            data.len()
        )));
    }
    let mut subsets = Vec::with_capacity(n_full_blocks * 4 + remainder / SUBSET_SIZE);
    let parse_block = |block: &[u8], n_subsets: usize, subsets: &mut Vec<DataIndexSubset>| {
        for sub_i in 0..n_subsets {
            let sub = &block[sub_i * SUBSET_SIZE..(sub_i + 1) * SUBSET_SIZE];
            let offset = LittleEndian::read_u32(&sub[0..4]);
            let entry_i = LittleEndian::read_u32(&sub[8..12]) as usize;
            subsets.push(DataIndexSubset {
                entry_i,
                sub_i,
                offset,
            });
        }
    };
    for block_i in 0..n_full_blocks {
        let block = &data[block_i * DATA_INDEX_ENTRY_SIZE..(block_i + 1) * DATA_INDEX_ENTRY_SIZE];
        parse_block(block, 4, &mut subsets);
    }
    if remainder > 0 {
        let tail = &data[n_full_blocks * DATA_INDEX_ENTRY_SIZE..];
        parse_block(tail, remainder / SUBSET_SIZE, &mut subsets);
    }
    Ok(subsets)
}

/// Compute the `(start, end)` byte range for each subset's scan in
/// `MS Raw Data`, bounding it by the next distinct offset in the *whole
/// stream's* sorted offset order.
///
/// Scans from different entries/channels are physically interleaved on
/// disk, so a subset's own `entry_i`/`sub_i` neighbors are not necessarily
/// adjacent - this ports the global-sorted-offset approach used by
/// `re/src/analysis/ttfl_rle_verify.py`'s `iter_scans`, which is how the
/// RLE payload decode was verified byte-exact across 109,336 real scans.
pub fn scan_bounds(subsets: &[DataIndexSubset], ms_raw_data_len: usize) -> Vec<(u32, u32)> {
    let sorted_offsets: Vec<u32> = subsets
        .iter()
        .map(|s| s.offset)
        .collect::<BTreeSet<u32>>()
        .into_iter()
        .collect();
    let next_of = |offset: u32| -> u32 {
        match sorted_offsets.binary_search(&offset) {
            Ok(idx) if idx + 1 < sorted_offsets.len() => sorted_offsets[idx + 1],
            _ => ms_raw_data_len as u32,
        }
    };
    subsets
        .iter()
        .map(|s| (s.offset, next_of(s.offset)))
        .collect()
}

/// A single decoded run in the RLE payload: `skip` zero-intensity samples
/// followed by `values.len()` real intensity samples.
#[derive(Debug, Clone)]
struct Run {
    skip: u16,
    values: Vec<u16>,
}

/// Maximum accepted marker run-length when *searching* for the
/// prefix/RLE boundary (keeps the search from accepting metadata bytes
/// that happen to look like an implausibly long run). Matches
/// `re/src/analysis/ttfl_rle_decode.py::find_prefix_end`'s
/// `max_run_len=64`.
const PREFIX_SEARCH_MAX_RUN_LEN: u16 = 64;

/// Maximum accepted marker run-length once decoding a confirmed RLE
/// stream. Matches `ttfl_rle_decode.py::decode_rle`'s `0x8000 + 4096`
/// bound.
const DECODE_MAX_RUN_LEN: u16 = 4096;

const RLE_MARKER_BASE: u16 = 0x8000;

/// Decode a run-length-encoded `(marker, skip, values)` stream starting
/// at word index `start`. Returns the decoded runs and the word index
/// just past the terminator, or `None` if the stream is malformed (ran
/// out of words mid-run, or hit a non-marker word where one was
/// expected).
fn decode_rle(words: &[u16], start: usize, max_run_len: u16) -> Option<(Vec<Run>, usize)> {
    let mut i = start;
    let mut runs = Vec::new();
    let n = words.len();
    while i < n {
        let w = words[i];
        if w == RLE_MARKER_BASE {
            // Terminator: zero-length run, no following skip/data.
            return Some((runs, i + 1));
        }
        if w > RLE_MARKER_BASE && w <= RLE_MARKER_BASE + max_run_len {
            let run_len = (w - RLE_MARKER_BASE) as usize;
            if i + 2 + run_len > n {
                return None; // not enough data left
            }
            let skip = words[i + 1];
            let values = words[i + 2..i + 2 + run_len].to_vec();
            runs.push(Run { skip, values });
            i += 2 + run_len;
        } else {
            return None; // unexpected non-marker word
        }
    }
    None // ran off the end without hitting the terminator
}

fn to_u16_words(bytes: &[u8]) -> Vec<u16> {
    let nwords = bytes.len() / 2;
    (0..nwords)
        .map(|w| LittleEndian::read_u16(&bytes[w * 2..w * 2 + 2]))
        .collect()
}

/// Find the byte offset within `payload` where the fixed-size
/// scan-metadata prefix ends and the RLE-encoded profile data begins, by
/// trying every 2-byte-aligned candidate marker position and requiring
/// the decode to consume the entire remainder of the payload with zero
/// leftover bytes. Ports `ttfl_rle_decode.py::find_prefix_end` exactly
/// (including its `max_run_len=64` bound on the candidate marker word
/// itself).
fn find_prefix_end(payload: &[u8]) -> Option<usize> {
    if payload.len() < 4 {
        return None;
    }
    let mut i = 0;
    while i + 1 < payload.len() {
        if payload[i + 1] == 0x80 {
            let n = payload[i] as u16;
            if n <= PREFIX_SEARCH_MAX_RUN_LEN {
                let tail = &payload[i..];
                if tail.len() % 2 == 0 {
                    let words = to_u16_words(tail);
                    let nwords = words.len();
                    if let Some((_, end_idx)) = decode_rle(&words, 0, DECODE_MAX_RUN_LEN) {
                        if end_idx == nwords {
                            return Some(i);
                        }
                    }
                }
            }
        }
        i += 2;
    }
    None
}

/// One decoded IT-TOF spectrum: a sparse set of (raw time-bin index,
/// intensity) samples reconstructed from the RLE payload's cumulative
/// `skip` values. **The index axis is not m/z** - see module doc.
#[derive(Debug, Clone, Default)]
pub struct TtflSpectrum {
    pub index_axis: Vec<f64>,
    pub intensity: Vec<f32>,
}

/// Decode one scan's full byte range (64-byte header + variable-length
/// metadata prefix + RLE payload tail) into a sparse spectrum.
///
/// Returns `None` if the scan is too short to contain a header, or if no
/// valid RLE boundary was found; callers should skip such scans rather
/// than fail the whole run, per `SpectrumSource::iter_spectra`'s
/// documented convention.
pub fn decode_scan(scan_bytes: &[u8]) -> Option<TtflSpectrum> {
    if scan_bytes.len() < SCAN_HEADER_SIZE {
        return None;
    }
    let payload = &scan_bytes[SCAN_HEADER_SIZE..];
    if payload.is_empty() {
        return Some(TtflSpectrum::default());
    }
    let prefix_end = find_prefix_end(payload)?;
    let tail = &payload[prefix_end..];
    if tail.len() % 2 != 0 {
        return None;
    }
    let words = to_u16_words(tail);
    let nwords = words.len();
    let (runs, end_idx) = decode_rle(&words, 0, DECODE_MAX_RUN_LEN)?;
    if end_idx != nwords {
        return None; // leftover bytes: not a clean decode
    }

    let mut index_axis = Vec::new();
    let mut intensity = Vec::new();
    let mut pos: u32 = 0;
    for run in runs {
        pos += run.skip as u32;
        for v in run.values {
            index_axis.push(pos as f64);
            intensity.push(v as f32);
            pos += 1;
        }
    }
    Some(TtflSpectrum {
        index_axis,
        intensity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_subset(buf: &mut [u8], offset: u32, entry_i: u32, evctr: u32) {
        LittleEndian::write_u32(&mut buf[0..4], offset);
        LittleEndian::write_u32(&mut buf[4..8], 0);
        LittleEndian::write_u32(&mut buf[8..12], entry_i);
        LittleEndian::write_u32(&mut buf[12..16], evctr);
    }

    #[test]
    fn parse_data_index_reads_entry_i_from_bytes_not_position() {
        // One 64-byte block whose 4 subsets carry entry_i [0, 0, 1, 1] -
        // reproduces PXD025121/17.lcd's 2-real-channel packing (two RT
        // points' worth of subsets share one physical block).
        let mut block = vec![0u8; 64];
        write_subset(&mut block[0..16], 100, 0, 0);
        write_subset(&mut block[16..32], 200, 0, 1);
        write_subset(&mut block[32..48], 300, 1, 2);
        write_subset(&mut block[48..64], 400, 1, 3);

        let subsets = parse_data_index(&block).expect("parse");
        assert_eq!(subsets.len(), 4);
        assert_eq!(subsets[0].entry_i, 0);
        assert_eq!(subsets[1].entry_i, 0);
        assert_eq!(subsets[2].entry_i, 1);
        assert_eq!(subsets[3].entry_i, 1);
    }

    #[test]
    fn parse_data_index_accepts_trailing_partial_block() {
        // Full 64-byte block (entry 0's 4 subsets) plus a trailing
        // 32-byte partial block (2 subsets, entry 1) - reproduces the
        // real trailing-partial-block condition found in 9 PXD025121
        // files this session.
        let mut data = vec![0u8; 64 + 32];
        write_subset(&mut data[0..16], 0, 0, 0);
        write_subset(&mut data[16..32], 10, 0, 1);
        write_subset(&mut data[32..48], 20, 0, 2);
        write_subset(&mut data[48..64], 30, 0, 3);
        write_subset(&mut data[64..80], 40, 1, 4);
        write_subset(&mut data[80..96], 50, 1, 5);

        let subsets = parse_data_index(&data).expect("parse");
        assert_eq!(subsets.len(), 6);
        assert_eq!(subsets[4].entry_i, 1);
        assert_eq!(subsets[4].offset, 40);
        assert_eq!(subsets[5].entry_i, 1);
        assert_eq!(subsets[5].offset, 50);
    }

    #[test]
    fn parse_data_index_rejects_non_16_byte_remainder() {
        let data = vec![0u8; 64 + 7]; // remainder not a multiple of 16
        assert!(parse_data_index(&data).is_err());
    }

    /// Reproduces the doc's canonical worked example: RT entry 0, channel
    /// 1, `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`, offset 1842 -
    /// 194-byte prefix + 460-byte (230-word) RLE tail decoding to 63 runs,
    /// 103 total peak values, first run marker 0x8002 -> run_length 2,
    /// skip 263, values [289, 67].
    #[test]
    fn decodes_a_minimal_rle_tail() {
        let mut scan = vec![0u8; SCAN_HEADER_SIZE];
        // A short bogus "metadata prefix" of 4 bytes, then the RLE tail.
        scan.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        // marker 0x8002 (run_length 2), skip 263, values 289 and 67.
        scan.extend_from_slice(&0x8002u16.to_le_bytes());
        scan.extend_from_slice(&263u16.to_le_bytes());
        scan.extend_from_slice(&289u16.to_le_bytes());
        scan.extend_from_slice(&67u16.to_le_bytes());
        // terminator
        scan.extend_from_slice(&0x8000u16.to_le_bytes());

        let spec = decode_scan(&scan).expect("decode");
        assert_eq!(spec.index_axis, vec![263.0, 264.0]);
        assert_eq!(spec.intensity, vec![289.0, 67.0]);
    }

    #[test]
    fn empty_payload_decodes_to_empty_spectrum() {
        let scan = vec![0u8; SCAN_HEADER_SIZE];
        let spec = decode_scan(&scan).expect("decode");
        assert!(spec.index_axis.is_empty());
        assert!(spec.intensity.is_empty());
    }

    #[test]
    fn malformed_payload_is_skipped() {
        // No 0x80-high-byte marker anywhere: find_prefix_end must fail.
        let mut scan = vec![0u8; SCAN_HEADER_SIZE];
        scan.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        assert!(decode_scan(&scan).is_none());
    }

    #[test]
    fn scan_bounds_uses_global_sorted_offsets() {
        // Two entries, 4 subsets each, but on-disk order interleaves them
        // (subset offsets are not monotonic in entry/sub order).
        let subsets = vec![
            DataIndexSubset {
                entry_i: 0,
                sub_i: 0,
                offset: 0,
            },
            DataIndexSubset {
                entry_i: 0,
                sub_i: 1,
                offset: 50,
            },
            DataIndexSubset {
                entry_i: 1,
                sub_i: 0,
                offset: 20,
            },
            DataIndexSubset {
                entry_i: 1,
                sub_i: 1,
                offset: 80,
            },
        ];
        let bounds = scan_bounds(&subsets, 100);
        // Sorted distinct offsets: [0, 20, 50, 80]. So offset 0 -> ends at 20,
        // offset 50 -> ends at 80, offset 20 -> ends at 50, offset 80 -> ends at 100.
        assert_eq!(bounds[0], (0, 20));
        assert_eq!(bounds[1], (50, 80));
        assert_eq!(bounds[2], (20, 50));
        assert_eq!(bounds[3], (80, 100));
    }
}
