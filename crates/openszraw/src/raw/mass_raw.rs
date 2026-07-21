//! Parsing of `.lcd` single-quadrupole LC-MS data (`Mass Raw Data`
//! storage, e.g. Shimadzu LCMS-2020).
//!
//! See `docs/format/07-mass-raw-data-single-quad.md`.

use byteorder::{ByteOrder, LittleEndian};

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

/// Parse the `Retention Time` / `Spectrum Index` streams: both are a
/// plain, headerless `u32[N]` array (`Spectrum Index` holding absolute
/// byte offsets of each scan into `MS Raw Data`, `Retention Time` holding
/// milliseconds) - the same shape as `.qgd`'s Variant A `Spectrum Index`
/// (`raw::qgd::parse_spectrum_index`).
pub fn parse_u32_array(data: &[u8]) -> Vec<u32> {
    let n = data.len() / 4;
    (0..n)
        .map(|i| LittleEndian::read_u32(&data[i * 4..i * 4 + 4]))
        .collect()
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

    if n_peaks == 0 || payload_size % n_peaks != 0 {
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

/// Read a little-endian unsigned integer of `width` bytes (1-4) into a
/// `u32`.
fn read_uint_le(bytes: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf[..bytes.len()].copy_from_slice(bytes);
    u32::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

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
