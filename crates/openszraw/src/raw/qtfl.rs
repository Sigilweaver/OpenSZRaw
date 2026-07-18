//! Parsing of `.lcd` QTOF LC-MS data (`QTFL RawData` storage).
//!
//! See `docs/format/05-qtfl-centroid.md`.

use byteorder::{ByteOrder, LittleEndian};

pub const INDEX_RECORD_SIZE: usize = 24;
pub const SCAN_HEADER_SIZE: usize = 64;

/// Byte size of each `Retention Time` stream record:
/// `[rt_ms: u32, scan_number: u32 (1-based), zero: u32]`.
///
/// Not covered by `docs/format/05-qtfl-centroid.md` (which documents only
/// `Centroid Index`/`Centroid Data`) - verified against
/// `MSV000084197/20190607_NM16.lcd` this session: the record count
/// (16018) exactly matches the `Centroid Index` record count, and the
/// `scan_number` field runs `1..=N` matching stream position; the first
/// field matches header `u32[1]` for every scan after the first (scan 0's
/// header carries an unrelated sentinel value in that slot, so the
/// `Retention Time` stream - not the per-scan header - is used as the
/// authoritative RT source here).
pub const RT_RECORD_SIZE: usize = 12;

/// One `Centroid Index` record. The byte length of the scan is derived
/// from the next record's offset, or end-of-stream for the last record -
/// see `docs/format/05-qtfl-centroid.md`.
#[derive(Debug, Clone, Copy)]
pub struct CentroidIndexRecord {
    pub offset: u32,
    /// `u32[2]`: doc 05 calls this "Subset/Interleave Index"; verified
    /// this session (not in the original doc) to be a per-acquisition-
    /// cycle counter - see `event_id`.
    pub cycle_index: u32,
    /// `u32[5]`: doc 05 calls this "Segment/Event ID". Verified this
    /// session against `MSV000084197/20190607_NM16.lcd`: within one
    /// `cycle_index` group, `event_id` always starts at 1 and increments
    /// (1, 2, ..up to 4 seen in this file), a pattern consistent with a
    /// DDA MS1 survey scan (`event_id == 1`) followed by a
    /// variable number of MS2 product-ion scans
    /// (`event_id > 1`, one per precursor selected that cycle) - see the
    /// addendum in `docs/format/05-qtfl-centroid.md`. This is new
    /// information beyond what the original (CONFIRMED-for-payload-decode
    /// only) doc covered; the real per-scan precursor m/z lives in the
    /// separate `QTFL RawData/DDA` stream, which was not decoded this
    /// session - see `docs/format/06-known-limitations.md`.
    pub event_id: u32,
}

pub fn parse_centroid_index(data: &[u8]) -> crate::Result<Vec<CentroidIndexRecord>> {
    if data.len() % INDEX_RECORD_SIZE != 0 {
        return Err(crate::Error::Parse(format!(
            "Centroid Index stream size {} is not a multiple of {INDEX_RECORD_SIZE}",
            data.len()
        )));
    }
    let n = data.len() / INDEX_RECORD_SIZE;
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &data[i * INDEX_RECORD_SIZE..(i + 1) * INDEX_RECORD_SIZE];
        let offset = LittleEndian::read_u32(&rec[0..4]);
        let cycle_index = LittleEndian::read_u32(&rec[8..12]);
        let event_id = LittleEndian::read_u32(&rec[20..24]);
        records.push(CentroidIndexRecord {
            offset,
            cycle_index,
            event_id,
        });
    }
    Ok(records)
}

/// Parse the `Retention Time` stream into `rt_ms` values, one per scan, in
/// stream order (position `i` corresponds to `Centroid Index` record `i`).
pub fn parse_retention_time(data: &[u8]) -> crate::Result<Vec<u32>> {
    if data.len() % RT_RECORD_SIZE != 0 {
        return Err(crate::Error::Parse(format!(
            "Retention Time stream size {} is not a multiple of {RT_RECORD_SIZE}",
            data.len()
        )));
    }
    let n = data.len() / RT_RECORD_SIZE;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &data[i * RT_RECORD_SIZE..(i + 1) * RT_RECORD_SIZE];
        out.push(LittleEndian::read_u32(&rec[0..4]));
    }
    Ok(out)
}

/// One decoded centroid spectrum.
#[derive(Debug, Clone, Default)]
pub struct QtflSpectrum {
    pub mz: Vec<f64>,
    pub intensity: Vec<f32>,
    /// Base peak intensity read directly from the scan header (`u32[4]`).
    /// Verified to equal `max(intensity)` exactly on real corpus scans
    /// this session, so it is safe to surface as `SpectrumRecord`'s
    /// `base_peak_intensity` (the conformance suite cross-checks it
    /// against the decoded array).
    pub base_peak_intensity: Option<f64>,
}

/// Decode one scan's full byte range (64-byte header + payload) into a
/// centroid spectrum. Payload size `S` (bytes) is read from the header at
/// `u32[6]`; `N = S / 10` peaks follow as `N` u64 m/z values (scaled by
/// 1e12) then `N` u16 intensities.
pub fn decode_scan(scan_bytes: &[u8]) -> crate::Result<QtflSpectrum> {
    if scan_bytes.len() < SCAN_HEADER_SIZE {
        return Err(crate::Error::Parse(format!(
            "QTOF centroid scan too short for {SCAN_HEADER_SIZE}-byte header: {} bytes",
            scan_bytes.len()
        )));
    }
    let bpi = LittleEndian::read_u32(&scan_bytes[0x10..0x14]);
    let payload_size = LittleEndian::read_u32(&scan_bytes[0x18..0x1C]) as usize;
    let payload = &scan_bytes[SCAN_HEADER_SIZE..];

    if payload_size == 0 {
        return Ok(QtflSpectrum::default());
    }
    if payload.len() < payload_size {
        return Err(crate::Error::Parse(format!(
            "QTOF centroid scan payload shorter than declared size: have {}, want {payload_size}",
            payload.len()
        )));
    }
    if payload_size % 10 != 0 {
        return Err(crate::Error::Parse(format!(
            "QTOF centroid payload size {payload_size} is not a multiple of 10"
        )));
    }
    let n = payload_size / 10;
    let mz_bytes = &payload[0..n * 8];
    let intensity_bytes = &payload[n * 8..n * 8 + n * 2];

    let mut mz = Vec::with_capacity(n);
    for i in 0..n {
        let raw_mz = LittleEndian::read_u64(&mz_bytes[i * 8..i * 8 + 8]);
        mz.push(raw_mz as f64 / 1_000_000_000_000.0);
    }
    let mut intensity = Vec::with_capacity(n);
    for i in 0..n {
        let raw_intensity = LittleEndian::read_u16(&intensity_bytes[i * 2..i * 2 + 2]);
        intensity.push(raw_intensity as f32);
    }

    Ok(QtflSpectrum {
        mz,
        intensity,
        base_peak_intensity: Some(bpi as f64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_scan(bpi: u32, mzs_scaled: &[u64], intensities: &[u16]) -> Vec<u8> {
        let n = mzs_scaled.len();
        assert_eq!(n, intensities.len());
        let payload_size = n * 10;
        let mut buf = vec![0u8; SCAN_HEADER_SIZE];
        LittleEndian::write_u32(&mut buf[0x10..0x14], bpi);
        LittleEndian::write_u32(&mut buf[0x18..0x1C], payload_size as u32);
        for m in mzs_scaled {
            buf.extend_from_slice(&m.to_le_bytes());
        }
        for i in intensities {
            buf.extend_from_slice(&i.to_le_bytes());
        }
        buf
    }

    #[test]
    fn decodes_the_doc_worked_example() {
        // docs/format/05: S=160 -> 16 peaks. Use a smaller 2-peak version
        // for a compact test, matching real scan0 values from
        // MSV000084197/20190607_NM16.lcd.
        let scan = build_scan(
            6650,
            &[533_541_065_063_308, 537_476_384_016_910],
            &[2886, 1013],
        );
        let spec = decode_scan(&scan).expect("decode");
        assert_eq!(spec.mz.len(), 2);
        assert!((spec.mz[0] - 533.541065063308).abs() < 1e-6);
        assert!((spec.mz[1] - 537.47638401691).abs() < 1e-6);
        assert_eq!(spec.intensity, vec![2886.0, 1013.0]);
        assert_eq!(spec.base_peak_intensity, Some(6650.0));
    }

    #[test]
    fn empty_scan_decodes_to_no_peaks() {
        let scan = build_scan(0, &[], &[]);
        let spec = decode_scan(&scan).expect("decode");
        assert!(spec.mz.is_empty());
        assert!(spec.intensity.is_empty());
    }

    #[test]
    fn centroid_index_parses_offset_and_event_id() {
        let mut data = vec![0u8; INDEX_RECORD_SIZE * 2];
        LittleEndian::write_u32(&mut data[0..4], 0);
        LittleEndian::write_u32(&mut data[20..24], 1); // event_id record 0
        LittleEndian::write_u32(&mut data[24..28], 224);
        LittleEndian::write_u32(&mut data[24 + 20..24 + 24], 2); // event_id record 1
        let recs = parse_centroid_index(&data).expect("parse");
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].offset, 0);
        assert_eq!(recs[0].event_id, 1);
        assert_eq!(recs[1].offset, 224);
        assert_eq!(recs[1].event_id, 2);
    }
}
