//! Parsing of `.lcd` single-quadrupole LC-MS data (`Mass Raw Data`
//! storage, e.g. Shimadzu LCMS-2020).
//!
//! See `docs/format/07-mass-raw-data-single-quad.md`.

use byteorder::{ByteOrder, LittleEndian};

use super::read_uint_le;

/// Fixed size of each `MS Raw Data` scan's header, before the variable-
/// length peak payload.
pub const SCAN_HEADER_SIZE: usize = 64;

/// Offset within the 64-byte header of the `u16` peak count.
const N_PEAKS_OFFSET: usize = 0x36;

/// One decoded full-scan profile spectrum from `Mass Raw Data/MS Raw
/// Data`.
#[derive(Debug, Clone)]
pub struct SingleQuadScan {
    pub retention_time_ms: u32,
    pub mz: Vec<f64>,
    pub intensity: Vec<f32>,
}

/// Parse one scan from `MS Raw Data` given its full byte range (64-byte
/// header + peak payload, bounded by consecutive `Spectrum Index`
/// offsets).
///
/// The header's own `u16` peak count at `N_PEAKS_OFFSET` (0x36), divided
/// into the payload size, gives the per-peak record width directly -
/// observed as 4 bytes ([mz: u16 * 10][intensity: u16]) or 5 bytes
/// ([mz: u16 * 10][intensity: LE uint, 3 bytes]) per peak in the local
/// corpus (`MTBLS1960`, 8 files) - mirroring both `raw::qgd`'s u16
/// `mz * 10` scaling and `raw::qtfl`'s discovery that intensity byte
/// width is a per-scan (not global) property. Verified byte-exact: the
/// sum of decoded intensities equals the file's own `TIC Data` value for
/// that scan, with 0 mismatches across all 19,200 scans (2,400 scans x 8
/// files) in the local corpus - see
/// `docs/format/07-mass-raw-data-single-quad.md`.
pub fn parse_scan(scan_bytes: &[u8]) -> crate::Result<SingleQuadScan> {
    if scan_bytes.len() < SCAN_HEADER_SIZE {
        return Err(crate::Error::Parse(format!(
            "single-quad scan too short for {SCAN_HEADER_SIZE}-byte header: {} bytes",
            scan_bytes.len()
        )));
    }
    let retention_time_ms = LittleEndian::read_u32(&scan_bytes[0x04..0x08]);
    let n_peaks = LittleEndian::read_u16(&scan_bytes[N_PEAKS_OFFSET..N_PEAKS_OFFSET + 2]) as usize;

    let payload = &scan_bytes[SCAN_HEADER_SIZE..];
    let payload_size = payload.len();

    if n_peaks == 0 || !payload_size.is_multiple_of(n_peaks) {
        return Err(crate::Error::Parse(format!(
            "single-quad scan payload size {payload_size} not evenly divisible by peak count {n_peaks}"
        )));
    }
    let record_width = payload_size / n_peaks;
    if record_width <= 2 || record_width > 6 {
        return Err(crate::Error::Parse(format!(
            "single-quad scan implies implausible peak record width {record_width} bytes (n_peaks={n_peaks}, payload={payload_size})"
        )));
    }
    let intensity_width = record_width - 2;

    let mut mz = Vec::with_capacity(n_peaks);
    let mut intensity = Vec::with_capacity(n_peaks);
    for i in 0..n_peaks {
        let base = i * record_width;
        let raw_mz = LittleEndian::read_u16(&payload[base..base + 2]);
        let raw_intensity = read_uint_le(&payload[base + 2..base + 2 + intensity_width]);
        mz.push(raw_mz as f64 / 10.0);
        intensity.push(raw_intensity as f32);
    }

    Ok(SingleQuadScan {
        retention_time_ms,
        mz,
        intensity,
    })
}

/// Put the stored `(m/z, intensity)` pairs in ascending m/z order with a
/// stable sort, moving each pair as a unit. Returns the number of pairs that
/// were out of place: the fewest pairs whose removal leaves the rest in order
/// (the length minus the longest non-decreasing run), so one displaced
/// record is 1 however far it moved, and 0 when the scan was already sorted
/// and nothing changed.
///
/// Some scans store one pair (usually a high-intensity peak) away from its
/// m/z position. Each pair is read as stored and the intensities sum to the
/// file's `TIC Data` value, so ordering them changes no decoded value.
///
/// `mz` and `intensity` must have the same length and `mz` must be finite,
/// which `parse_scan` guarantees (`u16 / 10`).
pub fn sort_peaks_by_mz(mz: &mut Vec<f64>, intensity: &mut Vec<f32>) -> usize {
    debug_assert_eq!(mz.len(), intensity.len());
    if mz.windows(2).all(|w| w[0] <= w[1]) {
        return 0;
    }
    // Longest non-decreasing subsequence length, by patience sorting.
    let mut tails: Vec<f64> = Vec::new();
    for &x in mz.iter() {
        let pos = tails.partition_point(|&t| t <= x);
        if pos == tails.len() {
            tails.push(x);
        } else {
            tails[pos] = x;
        }
    }
    let displaced = mz.len() - tails.len();
    let mut order: Vec<usize> = (0..mz.len()).collect();
    order.sort_by(|&a, &b| mz[a].total_cmp(&mz[b]));
    *mz = order.iter().map(|&i| mz[i]).collect();
    *intensity = order.iter().map(|&i| intensity[i]).collect();
    displaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_leaves_a_sorted_scan_alone() {
        let mut mz = vec![400.0, 402.3, 402.3];
        let mut intensity = vec![1.0, 2.0, 3.0];
        assert_eq!(sort_peaks_by_mz(&mut mz, &mut intensity), 0);
        assert_eq!(mz, vec![400.0, 402.3, 402.3]);
        assert_eq!(intensity, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn sort_moves_one_displaced_pair_and_counts_it_once() {
        // The base peak is stored first but belongs between 402.6 and 404.6.
        let mut mz = vec![403.8, 400.0, 401.2, 402.6, 404.6, 405.0];
        let mut intensity = vec![9000.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sort_peaks_by_mz(&mut mz, &mut intensity), 1);
        assert_eq!(mz, vec![400.0, 401.2, 402.6, 403.8, 404.6, 405.0]);
        assert_eq!(intensity, vec![1.0, 2.0, 3.0, 9000.0, 4.0, 5.0]);
    }

    #[test]
    fn sort_is_stable_for_equal_mz() {
        let mut mz = vec![401.0, 400.0, 400.0];
        let mut intensity = vec![7.0, 1.0, 2.0];
        assert_eq!(sort_peaks_by_mz(&mut mz, &mut intensity), 1);
        assert_eq!(mz, vec![400.0, 400.0, 401.0]);
        assert_eq!(intensity, vec![1.0, 2.0, 7.0]);
    }

    #[test]
    fn sort_counts_two_displaced_pairs() {
        let mut mz = vec![405.0, 400.0, 401.0, 402.0, 403.0, 410.0, 404.0, 411.0];
        let mut intensity = vec![0.0; 8];
        assert_eq!(sort_peaks_by_mz(&mut mz, &mut intensity), 2);
        assert!(mz.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn width_4_scan_roundtrip() {
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x04..0x08], 12500);
        LittleEndian::write_u16(&mut buf[N_PEAKS_OFFSET..N_PEAKS_OFFSET + 2], 2);
        buf.extend_from_slice(&4000u16.to_le_bytes());
        buf.extend_from_slice(&195u16.to_le_bytes());
        buf.extend_from_slice(&4023u16.to_le_bytes());
        buf.extend_from_slice(&786u16.to_le_bytes());

        let scan = parse_scan(&buf).expect("parse");
        assert_eq!(scan.retention_time_ms, 12500);
        assert_eq!(scan.mz, vec![400.0, 402.3]);
        assert_eq!(scan.intensity, vec![195.0, 786.0]);
    }

    #[test]
    fn width_5_scan_roundtrip() {
        // 3-byte intensity, mirroring the wider dynamic-range scans
        // observed in the corpus (e.g. MTBLS1960/E1.lcd scan 160).
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x04..0x08], 98765);
        LittleEndian::write_u16(&mut buf[N_PEAKS_OFFSET..N_PEAKS_OFFSET + 2], 1);
        buf.extend_from_slice(&4000u16.to_le_bytes());
        buf.extend_from_slice(&[0x34, 0x12, 0x03]); // LE 3-byte uint: 0x031234 = 201268

        let scan = parse_scan(&buf).expect("parse");
        assert_eq!(scan.retention_time_ms, 98765);
        assert_eq!(scan.mz, vec![400.0]);
        assert_eq!(scan.intensity, vec![201268.0]);
    }

    #[test]
    fn rejects_scan_shorter_than_header() {
        let buf = vec![0u8; 10];
        assert!(parse_scan(&buf).is_err());
    }

    #[test]
    fn rejects_indivisible_payload() {
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u16(&mut buf[N_PEAKS_OFFSET..N_PEAKS_OFFSET + 2], 3);
        buf.extend_from_slice(&[0u8; 7]); // not divisible by 3
        assert!(parse_scan(&buf).is_err());
    }
}
