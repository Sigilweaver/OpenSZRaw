//! High-level reader for Shimadzu `.qgd` / `.lcd` files.
//!
//! [`Reader::open`] detects which of the three on-disk variants a file is
//! (`.qgd` GC-MS, `.lcd` IT-TOF, `.lcd` QTOF - see the crate root doc
//! comment) and presents a single [`SpectrumSource`] implementation
//! regardless of which is underneath.

use std::path::Path;

use byteorder::{ByteOrder, LittleEndian};
use cfb::CompoundFile;
use openmassspec_core::{
    Analyzer, CvTerm, PrecursorInfo, RunMetadata, ScanMode, SpectrumRecord, SpectrumSource,
};

use crate::raw::{self, qgd, qtfl, ttfl, Variant};

const GCMS_SPECTRUM_INDEX: &str = "GCMS Raw Data/Spectrum Index";
const GCMS_MS_RAW_DATA: &str = "GCMS Raw Data/MS Raw Data";

const QTFL_CENTROID_INDEX: &str = "QTFL RawData/Centroid Index";
const QTFL_CENTROID_DATA: &str = "QTFL RawData/Centroid Data";
const QTFL_RETENTION_TIME: &str = "QTFL RawData/Retention Time";

const TTFL_DATA_INDEX: &str = "TTFL Raw Data/Data Index";
const TTFL_MS_RAW_DATA: &str = "TTFL Raw Data/MS Raw Data";
const TTFL_RETENTION_TIME: &str = "TTFL Raw Data/Retention Time";

/// The 3 on-disk copies of the tuning result stream that carry the
/// index-to-m/z calibration data (see `raw::ttfl::Calibration`) - tried
/// in order, since a file only needs one readable copy.
const TTFL_TUNING_RESULT: [&str; 3] = [
    "TTFL Tuning/Tuning Result 00",
    "TTFL Tuning/Tuning Result 01",
    "TTFL Tuning/Tuning Result 02",
];

/// Decoded state for whichever of the 3 on-disk variants this file is,
/// plus everything needed to decode individual scans on demand.
enum Decoded {
    Qgd {
        ms_raw: Vec<u8>,
        offsets: Vec<u64>,
    },
    Qtfl {
        centroid_data: Vec<u8>,
        records: Vec<qtfl::CentroidIndexRecord>,
        retention_time_ms: Vec<u32>,
    },
    Ttfl {
        ms_raw: Vec<u8>,
        subsets: Vec<ttfl::DataIndexSubset>,
        bounds: Vec<(u32, u32)>,
        retention_time_ms: Vec<u32>,
        /// `None` when no readable `TTFL Tuning/Tuning Result NN` stream
        /// with a usable calibration table was found - in that case
        /// spectra fall back to the raw, uncalibrated index axis rather
        /// than fabricate a calibration (see `docs/format/06-known-limitations.md`).
        calibration: Option<ttfl::Calibration>,
    },
}

/// A `.qgd` or `.lcd` reader. See the module doc comment.
pub struct Reader {
    /// Stem name of the file (e.g. "20190607_NM16") used in run metadata.
    pub stem: String,
    variant: Variant,
    decoded: Decoded,
    /// Earliest non-zero CFBF directory-entry creation timestamp in the
    /// container, RFC 3339 UTC - see `raw::timestamp` for how and why this
    /// is a reliable acquisition-start proxy. `None` only if every entry in
    /// the container (implausible for any real file) has an unset creation
    /// time.
    start_timestamp: Option<String>,
}

fn parse_u32_array(data: &[u8]) -> Vec<u32> {
    let n = data.len() / 4;
    (0..n)
        .map(|i| LittleEndian::read_u32(&data[i * 4..i * 4 + 4]))
        .collect()
}

impl Reader {
    /// Open a `.qgd` or `.lcd` file. The `.lcd` IT-TOF vs QTOF distinction
    /// is made by checking which top-level CFBF storage is present
    /// (`TTFL Raw Data` vs `QTFL RawData`), never by filename - see
    /// `raw::detect_variant`.
    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref();
        let ext_lower = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let file = std::fs::File::open(path)?;
        let mut comp = CompoundFile::open(file)
            .map_err(|e| crate::Error::Parse(format!("not a valid OLE2/CFBF container: {e}")))?;

        let variant = raw::detect_variant(&ext_lower, &mut comp)?;

        let start_timestamp = raw::timestamp::earliest_created_timestamp(&mut comp);

        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());

        let decoded = match variant {
            Variant::Qgd => {
                let si = raw::read_stream(&mut comp, GCMS_SPECTRUM_INDEX)?;
                let ms_raw = raw::read_stream(&mut comp, GCMS_MS_RAW_DATA)?;
                let offsets = qgd::parse_spectrum_index(&si)?;
                Decoded::Qgd { ms_raw, offsets }
            }
            Variant::Qtfl => {
                let ci = raw::read_stream(&mut comp, QTFL_CENTROID_INDEX)?;
                let centroid_data = raw::read_stream(&mut comp, QTFL_CENTROID_DATA)?;
                let rt = raw::read_stream(&mut comp, QTFL_RETENTION_TIME)?;
                let records = qtfl::parse_centroid_index(&ci)?;
                let retention_time_ms = qtfl::parse_retention_time(&rt)?;
                Decoded::Qtfl {
                    centroid_data,
                    records,
                    retention_time_ms,
                }
            }
            Variant::Ttfl => {
                let di = raw::read_stream(&mut comp, TTFL_DATA_INDEX)?;
                let ms_raw = raw::read_stream(&mut comp, TTFL_MS_RAW_DATA)?;
                let rt = raw::read_stream(&mut comp, TTFL_RETENTION_TIME)?;
                let subsets = ttfl::parse_data_index(&di)?;
                let bounds = ttfl::scan_bounds(&subsets, ms_raw.len());
                let retention_time_ms = parse_u32_array(&rt);
                // Any of the 3 on-disk copies carries the same
                // calibration data (see `TTFL_TUNING_RESULT`'s doc
                // comment); try each in turn and fall back to `None`
                // (raw, uncalibrated index axis) if none parse.
                let calibration = TTFL_TUNING_RESULT
                    .iter()
                    .find_map(|path| raw::read_stream_opt(&mut comp, path))
                    .and_then(|data| ttfl::parse_calibration(&data));
                Decoded::Ttfl {
                    ms_raw,
                    subsets,
                    bounds,
                    retention_time_ms,
                    calibration,
                }
            }
        };

        Ok(Reader {
            stem,
            variant,
            decoded,
            start_timestamp,
        })
    }
}

/// Build spectra for the `.qgd` GC-MS variant. Full-scan profile mode
/// yields one MS1 spectrum per physical scan; MRM/targeted mode yields
/// one MS2 "spectrum" per monitored transition within a scan (a single
/// `(product_mz, intensity)` point, with the precursor m/z carried in
/// `PrecursorInfo`) - there is no natural single-spectrum representation
/// for "N transitions measured simultaneously," so each transition is
/// surfaced as its own record, matching how other vendor crates in this
/// suite represent SRM/MRM data (e.g. opentfraw's flat-peak TSQ format).
fn qgd_spectra(stem: &str, ms_raw: &[u8], offsets: &[u64]) -> Vec<SpectrumRecord> {
    let n = offsets.len();
    let mut out = Vec::new();
    for i in 0..n {
        let start = offsets[i] as usize;
        let end = if i + 1 < n {
            offsets[i + 1] as usize
        } else {
            ms_raw.len()
        };
        if start > end || end > ms_raw.len() {
            continue; // malformed offset pair: skip silently
        }
        let scan = match qgd::parse_scan(&ms_raw[start..end]) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rt_sec = scan.retention_time_ms() as f64 / 1000.0;
        match scan {
            qgd::QgdScan::Profile { mz, intensity, .. } => {
                let idx = out.len();
                out.push(SpectrumRecord {
                    index: idx,
                    scan_number: (idx + 1) as u32,
                    native_id: format!("source={stem} start={} end={}", idx + 1, idx + 1),
                    ms_level: 1,
                    // No polarity bit was found anywhere in the 32-byte
                    // scan header or Spectrum Index this session; left
                    // unpopulated rather than assumed.
                    polarity: None,
                    scan_mode: Some(ScanMode::Profile),
                    analyzer: Some(Analyzer::SQMS),
                    filter: None,
                    retention_time_sec: rt_sec,
                    total_ion_current: None,
                    base_peak_mz: None,
                    base_peak_intensity: None,
                    low_mz: None,
                    high_mz: None,
                    ion_injection_time_ms: None,
                    inv_mobility: None,
                    faims_cv: None, // GC-MS instruments have no FAIMS interface.
                    precursor: None,
                    mz,
                    intensity,
                    inv_mobility_per_peak: None,
                });
            }
            qgd::QgdScan::Mrm { transitions, .. } => {
                for t in transitions {
                    let idx = out.len();
                    out.push(SpectrumRecord {
                        index: idx,
                        scan_number: (idx + 1) as u32,
                        native_id: format!("source={stem} start={} end={}", idx + 1, idx + 1),
                        ms_level: 2,
                        polarity: None,
                        scan_mode: Some(ScanMode::Centroid),
                        analyzer: Some(Analyzer::TQMS),
                        filter: None,
                        retention_time_sec: rt_sec,
                        total_ion_current: None,
                        base_peak_mz: None,
                        base_peak_intensity: None,
                        low_mz: None,
                        high_mz: None,
                        ion_injection_time_ms: None,
                        inv_mobility: None,
                        faims_cv: None,
                        precursor: Some(PrecursorInfo {
                            target_mz: Some(t.precursor_mz),
                            selected_mz: Some(t.precursor_mz),
                            ..Default::default()
                        }),
                        mz: vec![t.product_mz],
                        intensity: vec![t.intensity],
                        inv_mobility_per_peak: None,
                    });
                }
            }
        }
    }
    out
}

/// Build spectra for the `.lcd` QTOF variant. `event_id == 1` is treated
/// as the MS1 survey scan and `event_id > 1` as an MS2/DDA product scan,
/// per the cycle structure verified this session (see
/// `raw::qtfl::CentroidIndexRecord::event_id`'s doc comment) - this goes
/// beyond what `docs/format/05-qtfl-centroid.md` originally covered
/// (payload decode only). The real precursor m/z for MS2 scans lives in
/// the separate, undecoded `QTFL RawData/DDA` stream, so MS2 records here
/// only carry a `precursor_native_id` referencing the most recent MS1
/// scan - see `docs/format/06-known-limitations.md`.
fn qtfl_spectra(
    centroid_data: &[u8],
    records: &[qtfl::CentroidIndexRecord],
    retention_time_ms: &[u32],
) -> Vec<SpectrumRecord> {
    let n = records.len();
    let mut out = Vec::with_capacity(n);
    let mut last_ms1_native_id: Option<String> = None;
    for i in 0..n {
        let start = records[i].offset as usize;
        let end = if i + 1 < n {
            records[i + 1].offset as usize
        } else {
            centroid_data.len()
        };
        if start > end || end > centroid_data.len() {
            continue;
        }
        let spec = match qtfl::decode_scan(&centroid_data[start..end]) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rt_ms = retention_time_ms.get(i).copied().unwrap_or(0);
        let is_ms1 = records[i].event_id <= 1;
        let idx = out.len();
        let native_id = format!("scan={}", idx + 1);
        if is_ms1 {
            last_ms1_native_id = Some(native_id.clone());
        }
        out.push(SpectrumRecord {
            index: idx,
            scan_number: (idx + 1) as u32,
            native_id,
            ms_level: if is_ms1 { 1 } else { 2 },
            polarity: None,
            scan_mode: Some(ScanMode::Centroid),
            analyzer: Some(Analyzer::TOFMS),
            filter: None,
            retention_time_sec: rt_ms as f64 / 1000.0,
            total_ion_current: None,
            base_peak_mz: None,
            base_peak_intensity: spec.base_peak_intensity,
            low_mz: None,
            high_mz: None,
            ion_injection_time_ms: None,
            inv_mobility: None,
            faims_cv: None, // Shimadzu QTOF instruments have no FAIMS interface.
            precursor: if is_ms1 {
                None
            } else {
                Some(PrecursorInfo {
                    precursor_native_id: last_ms1_native_id.clone(),
                    ..Default::default()
                })
            },
            mz: spec.mz,
            intensity: spec.intensity,
            inv_mobility_per_peak: None,
        });
    }
    out
}

/// Build spectra for the `.lcd` IT-TOF variant. Each `(entry, channel)`
/// subset yields one spectrum. Emission order is entry-major (RT order),
/// then channel 0..3 within each entry.
///
/// `mz` is calibrated physical m/z when `calibration` is `Some` (the
/// common case - see `raw::ttfl::Calibration`'s doc comment for the
/// derivation and evidence), computed from the RLE payload's raw
/// time-bin index via `Calibration::mz`. When no calibration could be
/// parsed from the file (`None`), this falls back to the raw,
/// uncalibrated index axis rather than fabricate one - see
/// `docs/format/06-known-limitations.md`.
fn ttfl_spectra(
    stem: &str,
    ms_raw: &[u8],
    subsets: &[ttfl::DataIndexSubset],
    bounds: &[(u32, u32)],
    retention_time_ms: &[u32],
    calibration: Option<&ttfl::Calibration>,
) -> Vec<SpectrumRecord> {
    let mut out = Vec::with_capacity(subsets.len());
    for (subset, &(start, end)) in subsets.iter().zip(bounds.iter()) {
        let start = start as usize;
        let end = end as usize;
        if start > end || end > ms_raw.len() {
            continue;
        }
        let Some(spec) = ttfl::decode_scan(&ms_raw[start..end]) else {
            continue;
        };
        let rt_ms = retention_time_ms.get(subset.entry_i).copied().unwrap_or(0);
        let idx = out.len();
        let mz = match calibration {
            Some(cal) => spec.index_axis.iter().map(|&i| cal.mz(i)).collect(),
            None => spec.index_axis,
        };
        out.push(SpectrumRecord {
            index: idx,
            scan_number: (idx + 1) as u32,
            native_id: format!("source={stem} start={} end={}", idx + 1, idx + 1),
            ms_level: 1,
            // Neither polarity nor MS-level is recoverable per channel:
            // the header's `u32[5]` mode flag was investigated and is not
            // confirmed to map to a specific polarity/level assignment -
            // see docs/format/06-known-limitations.md.
            polarity: None,
            scan_mode: Some(ScanMode::Profile),
            analyzer: Some(Analyzer::TOFMS),
            filter: None,
            retention_time_sec: rt_ms as f64 / 1000.0,
            total_ion_current: None,
            base_peak_mz: None,
            base_peak_intensity: None,
            low_mz: None,
            high_mz: None,
            ion_injection_time_ms: None,
            inv_mobility: None,
            faims_cv: None, // Shimadzu IT-TOF instruments have no FAIMS interface.
            precursor: None,
            mz,
            intensity: spec.intensity,
            inv_mobility_per_peak: None,
        });
    }
    out
}

impl SpectrumSource for Reader {
    fn run_metadata(&self) -> RunMetadata {
        match self.variant {
            Variant::Qgd => RunMetadata {
                source_file_name: format!("{}.qgd", self.stem),
                // No dedicated PSI-MS CV term for GCMSsolution's .qgd
                // format was found in psi-ms.obo; fall back to the
                // generic "mass spectrometer file format" node with a
                // descriptive name, matching how other vendor crates in
                // this suite handle CV gaps (e.g. openaraw's generic
                // Agilent instrument fallback).
                source_file_format: CvTerm::new("MS:1000560", "Shimadzu GCMSsolution QGD format"),
                native_id_format: CvTerm::new("MS:1000929", "Shimadzu Biotech nativeID format"),
                instrument: CvTerm::new("MS:1000124", "Shimadzu instrument model"),
                software_name: "openszraw".to_string(),
                software_version: env!("CARGO_PKG_VERSION").to_string(),
                start_timestamp: self.start_timestamp.clone(),
                mobility_array_kind: None,
            },
            Variant::Qtfl => RunMetadata {
                source_file_name: format!("{}.lcd", self.stem),
                source_file_format: CvTerm::new("MS:1003009", "Shimadzu Biotech LCD format"),
                native_id_format: CvTerm::new(
                    "MS:1002898",
                    "Shimadzu Biotech QTOF nativeID format",
                ),
                // MS:1002998 "LCMS-9030" is Shimadzu's only Q-TOF product
                // (per psi-ms.obo); justified by the `QTFL RawData`
                // storage itself (this format family) plus the corpus
                // file's own `GUMM_Information/ShimadzuLCMS-Q-TOF.1`
                // substream, not by filename.
                instrument: CvTerm::new("MS:1002998", "LCMS-9030"),
                software_name: "openszraw".to_string(),
                software_version: env!("CARGO_PKG_VERSION").to_string(),
                start_timestamp: self.start_timestamp.clone(),
                mobility_array_kind: None,
            },
            Variant::Ttfl => RunMetadata {
                source_file_name: format!("{}.lcd", self.stem),
                source_file_format: CvTerm::new("MS:1003009", "Shimadzu Biotech LCD format"),
                native_id_format: CvTerm::new("MS:1000929", "Shimadzu Biotech nativeID format"),
                // MS:1000604 "LCMS-IT-TOF" is justified directly by the
                // `TTFL Raw Data` storage (this format family is
                // IT-TOF-specific hardware).
                instrument: CvTerm::new("MS:1000604", "LCMS-IT-TOF"),
                software_name: "openszraw".to_string(),
                software_version: env!("CARGO_PKG_VERSION").to_string(),
                start_timestamp: self.start_timestamp.clone(),
                mobility_array_kind: None,
            },
        }
    }

    fn spectrum_count_hint(&self) -> Option<usize> {
        match &self.decoded {
            // Approximate: MRM scans expand into one record per
            // transition, so the real spectrum count can exceed this.
            Decoded::Qgd { offsets, .. } => Some(offsets.len()),
            Decoded::Qtfl { records, .. } => Some(records.len()),
            Decoded::Ttfl { subsets, .. } => Some(subsets.len()),
        }
    }

    fn iter_spectra<'a>(&'a mut self) -> Box<dyn Iterator<Item = SpectrumRecord> + 'a> {
        let spectra = match &self.decoded {
            Decoded::Qgd { ms_raw, offsets } => qgd_spectra(&self.stem, ms_raw, offsets),
            Decoded::Qtfl {
                centroid_data,
                records,
                retention_time_ms,
            } => qtfl_spectra(centroid_data, records, retention_time_ms),
            Decoded::Ttfl {
                ms_raw,
                subsets,
                bounds,
                retention_time_ms,
                calibration,
            } => ttfl_spectra(
                &self.stem,
                ms_raw,
                subsets,
                bounds,
                retention_time_ms,
                calibration.as_ref(),
            ),
        };
        Box::new(spectra.into_iter())
    }
}
