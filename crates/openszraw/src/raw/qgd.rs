//! Parsing of `.qgd` GCMSsolution GC-MS data (`GCMS Raw Data` storage).
//!
//! See `docs/format/02-gcms-qgd-scans.md`.

use byteorder::{ByteOrder, LittleEndian};

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
    if size % 4 == 0 {
        let n = size / 4;
        let mut offsets = Vec::with_capacity(n);
        for i in 0..n {
            offsets.push(LittleEndian::read_u32(&data[i * 4..i * 4 + 4]) as u64);
        }
        Ok(offsets)
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

/// Read a little-endian unsigned integer of `width` bytes (2, 3, or 4)
/// into a `u32`.
fn read_uint_le(bytes: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf[..bytes.len()].copy_from_slice(bytes);
    u32::from_le_bytes(buf)
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

    // Variant A: Format==2 at 0x14 and the declared peak count exactly
    // accounts for the payload size (32-byte header + N_Peaks*4 bytes).
    if format_a == 2 && data_bytes == n_peaks_a as usize * 4 {
        let n = n_peaks_a as usize;
        let mut mz = Vec::with_capacity(n);
        let mut intensity = Vec::with_capacity(n);
        for i in 0..n {
            let raw_mz = LittleEndian::read_u16(&payload[i * 4..i * 4 + 2]);
            let raw_intensity = LittleEndian::read_u16(&payload[i * 4 + 2..i * 4 + 4]);
            mz.push(raw_mz as f64 / 10.0);
            intensity.push(raw_intensity as f32);
        }
        return Ok(QgdScan::Profile {
            retention_time_ms,
            mz,
            intensity,
        });
    }

    // Variant B: MRM/targeted mode. The header's own transition count
    // (`n_transitions_b`, offset 0x1A) gives an exact, unambiguous
    // per-transition record size (`data_bytes / n_transitions`), which we
    // use in preference to the format doc's blind-divisibility fallback
    // (dividing by 6/7/8 and hoping for a unique fit) - this was verified
    // against real corpus bytes this session (PXD034978:
    // event 101 -> 4-byte intensity, event 102 -> 2-byte, event
    // 103 -> 3-byte, all exactly reproducing
    // `data_bytes / n_transitions_b - 4`, and matching the doc's own
    // per-event width table). A transition record is
    // `[precursor_mz: u16 *10][product_mz: u16 *10][intensity: LE uint,
    // 2/3/4 bytes]`.
    if n_transitions_b > 0 && data_bytes % n_transitions_b as usize == 0 {
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

    #[test]
    fn mrm_scan_roundtrip() {
        // Mirrors the real event-102 bytes verified this session:
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
