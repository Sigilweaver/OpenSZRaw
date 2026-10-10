//! Parsing of `.qgd` GCMSsolution GC-MS data (`GCMS Raw Data` storage).
//!
//! See `docs/format/02-gcms-qgd-scans.md`.

use byteorder::{ByteOrder, LittleEndian};

use super::read_uint_le;

pub const SCAN_HEADER_SIZE: usize = 32;

/// One decoded scan from `MS Raw Data`, tagged by acquisition mode.
#[derive(Debug, Clone)]
pub enum QgdScan {
    /// Full-scan profile/centroid mode (Variant A): a plain (m/z,
    /// intensity) spectrum.
    Profile {
        retention_time_ms: u32,
        mz: Vec<f64>,
        intensity: Vec<f32>,
    },
    /// MRM/targeted mode (Variant B): a set of transitions monitored
    /// within this scan, each with its own precursor/product m/z and
    /// intensity.
    Mrm {
        retention_time_ms: u32,
        event_id: u16,
        transitions: Vec<MrmTransition>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct MrmTransition {
    pub precursor_mz: f64,
    pub product_mz: f64,
    pub intensity: f32,
}

impl QgdScan {
    pub fn retention_time_ms(&self) -> u32 {
        match self {
            QgdScan::Profile {
                retention_time_ms, ..
            } => *retention_time_ms,
            QgdScan::Mrm {
                retention_time_ms, ..
            } => *retention_time_ms,
        }
    }
}

/// Parse the `Spectrum Index` stream, returning the absolute byte offsets
/// of each scan into `MS Raw Data`. Detects Variant A (plain `u32[N]`, no
/// header) vs Variant B (2-byte header + `u64[N]`) from the stream size,
/// per `docs/format/02-gcms-qgd-scans.md`. The two size classes cannot
/// collide: `size % 4 == 0` implies `size % 8 != 2`, so checking Variant A
/// first is unambiguous.
pub fn parse_spectrum_index(data: &[u8]) -> crate::Result<Vec<u64>> {
    let size = data.len();
    if size.is_multiple_of(4) {
        Ok(super::parse_u32_array(data)
            .into_iter()
            .map(u64::from)
            .collect())
    } else if size >= 2 && size % 8 == 2 {
        let n = (size - 2) / 8;
        let mut offsets = Vec::with_capacity(n);
        for i in 0..n {
            let off = 2 + i * 8;
            offsets.push(LittleEndian::read_u64(&data[off..off + 8]));
        }
        Ok(offsets)
    } else {
        Err(crate::Error::Parse(format!(
            "Spectrum Index stream size {size} matches neither Variant A (u32[N]) nor Variant B (2-byte header + u64[N])"
        )))
    }
}

/// Map a Variant A (profile) header `Format` code (offset 0x14) to its
/// per-peak intensity byte width. Each record is `[mz: u16 * 10]`
/// followed by a little-endian unsigned intensity of this width.
///
/// Verified against the local corpus by summing decoded intensities and
/// comparing with the file's own `GCMS Raw Data/TIC Data` (`u64` per scan):
///
/// - `2` -> 2 bytes (PXD019638).
/// - `3` -> 3 bytes: 1,320 scans in `PXD019638/L-B2-Br0-1.qgd` and 4,602
///   in `MTBLS11411/240403_10_Control 2.qgd`, all matching `TIC Data`
///   exactly.
/// - `1` -> 4 bytes: 130 scans across 4 `MTBLS11411` files, all on
///   high-abundance ions. These match `TIC Data` exactly only once bit 31
///   is cleared (see [`PROFILE_U32_VALUE_MASK`]).
///
/// `0` is the Variant B (MRM) layout, which keeps these header slots
/// zero; any other code is unknown and returns `None`.
fn profile_intensity_width(format: u16) -> Option<usize> {
    match format {
        2 => Some(2),
        3 => Some(3),
        1 => Some(4),
        _ => None,
    }
}

/// Bit 31 of a 4-byte profile intensity is a flag, not magnitude: the
/// instrument's own `TIC Data` equals the sum of the low 31 bits for every
/// 4-byte scan in the corpus (130 of 130). The flagged peaks sit on the
/// most intense ions of each scan, consistent with a saturation marker,
/// but its meaning is not confirmed, so it is dropped rather than
/// surfaced.
const PROFILE_U32_VALUE_MASK: u32 = 0x7FFF_FFFF;

/// How many spectrum records a scan that failed [`parse_scan`] would have
/// produced, judged from its header alone: the declared transition count
/// (offset 0x1A) for an MRM-shaped header (`Format` slot at 0x14 zero),
/// otherwise 1. Used only to keep record numbering source-stable across
/// skipped scans.
pub fn declared_record_count(scan_bytes: &[u8]) -> usize {
    if scan_bytes.len() < SCAN_HEADER_SIZE {
        return 1;
    }
    let format_a = LittleEndian::read_u16(&scan_bytes[0x14..0x16]);
    let n_transitions_b = LittleEndian::read_u16(&scan_bytes[0x1A..0x1C]);
    if format_a == 0 && n_transitions_b > 0 {
        n_transitions_b as usize
    } else {
        1
    }
}

/// Parse one scan from `MS Raw Data` given its full byte range (32-byte
/// header + payload, bounded by consecutive `Spectrum Index` offsets).
///
/// Returns `Err` if the scan does not cleanly match either the Profile or
/// MRM layout; callers should skip such scans rather than fail the whole
/// run, per `SpectrumSource::iter_spectra`'s documented convention.
pub fn parse_scan(scan_bytes: &[u8]) -> crate::Result<QgdScan> {
    if scan_bytes.len() < SCAN_HEADER_SIZE {
        return Err(crate::Error::Parse(format!(
            "GC-MS scan too short for {SCAN_HEADER_SIZE}-byte header: {} bytes",
            scan_bytes.len()
        )));
    }
    let retention_time_ms = LittleEndian::read_u32(&scan_bytes[0x04..0x08]);
    let format_a = LittleEndian::read_u16(&scan_bytes[0x14..0x16]);
    let n_peaks_a = LittleEndian::read_u16(&scan_bytes[0x16..0x18]);
    let event_id = LittleEndian::read_u16(&scan_bytes[0x18..0x1A]);
    let n_transitions_b = LittleEndian::read_u16(&scan_bytes[0x1A..0x1C]);

    let payload = &scan_bytes[SCAN_HEADER_SIZE..];
    let data_bytes = payload.len();

    // Variant A: the `Format` code at 0x14 selects the per-peak
    // intensity width, and the declared peak count must exactly account
    // for the payload (32-byte header + N_Peaks * (2 + width) bytes).
    if let Some(width) = profile_intensity_width(format_a) {
        let n = n_peaks_a as usize;
        let record_size = 2 + width;
        if data_bytes == n * record_size {
            let mut mz = Vec::with_capacity(n);
            let mut intensity = Vec::with_capacity(n);
            for i in 0..n {
                let base = i * record_size;
                let raw_mz = LittleEndian::read_u16(&payload[base..base + 2]);
                let mut raw_intensity = read_uint_le(&payload[base + 2..base + record_size]);
                if width == 4 {
                    raw_intensity &= PROFILE_U32_VALUE_MASK;
                }
                mz.push(raw_mz as f64 / 10.0);
                intensity.push(raw_intensity as f32);
            }
            return Ok(QgdScan::Profile {
                retention_time_ms,
                mz,
                intensity,
            });
        }
    }

    // Variant B: MRM/targeted mode. The header's own transition count
    // (`n_transitions_b`, offset 0x1A) gives an exact, unambiguous
    // per-transition record size (`data_bytes / n_transitions`). In
    // PXD034978 this yields event 101 -> 4-byte intensity, 102 -> 2-byte,
    // 103 -> 3-byte, matching the per-event width table in the format
    // doc. A transition record is
    // `[precursor_mz: u16 *10][product_mz: u16 *10][intensity: LE uint,
    // 2/3/4 bytes]`.
    if n_transitions_b > 0 && data_bytes.is_multiple_of(n_transitions_b as usize) {
        let record_size = data_bytes / n_transitions_b as usize;
        if record_size > 4 {
            let width = record_size - 4;
            if (2..=4).contains(&width) {
                let n = n_transitions_b as usize;
                let mut transitions = Vec::with_capacity(n);
                for i in 0..n {
                    let base = i * record_size;
                    let raw_precursor = LittleEndian::read_u16(&payload[base..base + 2]);
                    let raw_product = LittleEndian::read_u16(&payload[base + 2..base + 4]);
                    let raw_intensity = read_uint_le(&payload[base + 4..base + 4 + width]);
                    transitions.push(MrmTransition {
                        precursor_mz: raw_precursor as f64 / 10.0,
                        product_mz: raw_product as f64 / 10.0,
                        intensity: raw_intensity as f32,
                    });
                }
                return Ok(QgdScan::Mrm {
                    retention_time_ms,
                    event_id,
                    transitions,
                });
            }
        }
    }

    Err(crate::Error::Parse(format!(
        "GC-MS scan did not match Profile or MRM layout: format={format_a} n_peaks={n_peaks_a} event_id={event_id} n_transitions={n_transitions_b} data_bytes={data_bytes}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_scan_roundtrip() {
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x04..0x08], 336000);
        LittleEndian::write_u16(&mut buf[0x14..0x16], 2); // format
        LittleEndian::write_u16(&mut buf[0x16..0x18], 2); // n_peaks
        buf.extend_from_slice(&1000u16.to_le_bytes());
        buf.extend_from_slice(&484u16.to_le_bytes());
        buf.extend_from_slice(&1020u16.to_le_bytes());
        buf.extend_from_slice(&494u16.to_le_bytes());

        let scan = parse_scan(&buf).expect("parse");
        match scan {
            QgdScan::Profile {
                retention_time_ms,
                mz,
                intensity,
            } => {
                assert_eq!(retention_time_ms, 336000);
                assert_eq!(mz, vec![100.0, 102.0]);
                assert_eq!(intensity, vec![484.0, 494.0]);
            }
            QgdScan::Mrm { .. } => panic!("expected Profile"),
        }
    }

    /// Builds a Variant A profile scan with the given `Format` code and
    /// pre-encoded `(raw_mz, intensity_bytes)` peak records.
    fn profile_scan(format: u16, peaks: &[(u16, &[u8])]) -> Vec<u8> {
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x04..0x08], 1234);
        LittleEndian::write_u16(&mut buf[0x14..0x16], format);
        LittleEndian::write_u16(&mut buf[0x16..0x18], peaks.len() as u16);
        for (mz, inten) in peaks {
            buf.extend_from_slice(&mz.to_le_bytes());
            buf.extend_from_slice(inten);
        }
        buf
    }

    fn profile_intensities(buf: &[u8]) -> Vec<f32> {
        match parse_scan(buf).expect("parse") {
            QgdScan::Profile { intensity, .. } => intensity,
            QgdScan::Mrm { .. } => panic!("expected Profile"),
        }
    }

    #[test]
    fn profile_scan_accepts_2_3_and_4_byte_intensities() {
        let buf = profile_scan(2, &[(1000, &[0x34, 0x12]), (1020, &[1, 0])]);
        assert_eq!(profile_intensities(&buf), vec![4660.0, 1.0]);

        // Format 3: 3-byte intensity (seen in PXD019638 and MTBLS11411).
        let buf = profile_scan(3, &[(1000, &[0x34, 0x12, 0x03]), (1020, &[0, 0, 1])]);
        assert_eq!(profile_intensities(&buf), vec![201268.0, 65536.0]);

        // Format 1: 4-byte intensity with bit 31 cleared (MTBLS11411).
        let buf = profile_scan(
            1,
            &[
                (2002, &0x8062_C812u32.to_le_bytes()),
                (3484, &70_000u32.to_le_bytes()),
            ],
        );
        assert_eq!(
            profile_intensities(&buf),
            vec![0x0062_C812 as f32, 70_000.0]
        );
    }

    #[test]
    fn profile_scan_rejects_width_that_disagrees_with_format() {
        // Format 3 declares 3-byte intensities, but the payload is sized
        // for 2-byte records: must not decode as a profile scan.
        let buf = profile_scan(3, &[(1000, &[1, 0]), (1020, &[2, 0])]);
        assert!(parse_scan(&buf).is_err());
        // Unknown format code.
        let buf = profile_scan(7, &[(1000, &[1, 0, 0, 0, 0, 0, 0])]);
        assert!(parse_scan(&buf).is_err());
    }

    #[test]
    fn mrm_scan_roundtrip() {
        // Mirrors real event-102 bytes from PXD034978:
        // transitions (320.2 -> 116.0, intensity 504), (350.2 -> 116.0, intensity 508).
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x04..0x08], 387000);
        LittleEndian::write_u16(&mut buf[0x18..0x1A], 102); // event_id
        LittleEndian::write_u16(&mut buf[0x1A..0x1C], 2); // n_transitions
        for (p, prod, i) in [(3202u16, 1160u16, 504u16), (3502, 1160, 508)] {
            buf.extend_from_slice(&p.to_le_bytes());
            buf.extend_from_slice(&prod.to_le_bytes());
            buf.extend_from_slice(&i.to_le_bytes());
        }

        let scan = parse_scan(&buf).expect("parse");
        match scan {
            QgdScan::Mrm {
                retention_time_ms,
                event_id,
                transitions,
            } => {
                assert_eq!(retention_time_ms, 387000);
                assert_eq!(event_id, 102);
                assert_eq!(transitions.len(), 2);
                assert!((transitions[0].precursor_mz - 320.2).abs() < 1e-9);
                assert!((transitions[0].product_mz - 116.0).abs() < 1e-9);
                assert_eq!(transitions[0].intensity, 504.0);
                assert!((transitions[1].precursor_mz - 350.2).abs() < 1e-9);
                assert_eq!(transitions[1].intensity, 508.0);
            }
            QgdScan::Profile { .. } => panic!("expected Mrm"),
        }
    }

    #[test]
    fn spectrum_index_variant_a() {
        let mut data = Vec::new();
        for v in [0u32, 100, 250] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let offsets = parse_spectrum_index(&data).expect("parse");
        assert_eq!(offsets, vec![0, 100, 250]);
    }

    #[test]
    fn spectrum_index_variant_b() {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_le_bytes());
        for v in [0u64, 1000, 2500] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let offsets = parse_spectrum_index(&data).expect("parse");
        assert_eq!(offsets, vec![0, 1000, 2500]);
    }
}
