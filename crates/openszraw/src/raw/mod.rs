//! Low-level parsing modules for `.qgd` and `.lcd` files.

pub mod lc_chrom;
pub mod mass_raw;
pub mod qgd;
pub mod qtfl;
pub mod timestamp;
pub mod ttfl;

use std::io::Read;

use cfb::CompoundFile;

/// The four on-disk variants this crate can decode, detected at `open()`
/// time - see `docs/format/01-ole2-container.md` and the crate root doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// `.qgd` GCMSsolution GC-MS data (`GCMS Raw Data` storage).
    Qgd,
    /// `.lcd` IT-TOF LC-MS data (`TTFL Raw Data` storage).
    Ttfl,
    /// `.lcd` QTOF LC-MS data (`QTFL RawData` storage).
    Qtfl,
    /// `.lcd` single-quadrupole LC-MS data (`Mass Raw Data` storage, e.g.
    /// Shimadzu LCMS-2020) - see
    /// `docs/format/07-mass-raw-data-single-quad.md`.
    SingleQuad,
}

/// Root CFBF storage name for each variant.
const GCMS_ROOT: &str = "GCMS Raw Data";
const TTFL_ROOT: &str = "TTFL Raw Data";
const QTFL_ROOT: &str = "QTFL RawData";
const MASS_RAW_ROOT: &str = "Mass Raw Data";
/// Root storage for the QQQ (triple-quadrupole, e.g. LCMS-8060) on-disk
/// variant - not decoded by this crate yet, see
/// `docs/format/06-known-limitations.md` section 7 and
/// Sigilweaver/OpenSZRaw#5.
const TLM_ROOT: &str = "TLM Raw Data";

/// A substream that only exists when `MASS_RAW_ROOT` is actually
/// populated. Every `.lcd` file - QQQ (`TLM Raw Data`) ones included -
/// carries an always-present `Mass Raw Data` storage as boilerplate,
/// empty of any substreams when it is not the file's real variant (the
/// same trap `docs/format/06-known-limitations.md` section 7 documents
/// for `QTFL RawData` on QQQ files: confirmed present-but-empty on both
/// `MTBLS2376` and `MTBLS7425`, two QQQ accessions). Checking for this
/// substream rather than the bare root storage avoids misdetecting QQQ
/// files as `SingleQuad`.
const MASS_RAW_MS_DATA: &str = "Mass Raw Data/MS Raw Data";

/// A substream that only exists when `QTFL_ROOT` is actually populated.
/// Every `.lcd` file - QQQ (`TLM Raw Data`) ones included - carries an
/// always-present `QTFL RawData` storage as boilerplate, empty of any
/// substreams when it is not the file's real variant (see
/// `docs/format/06-known-limitations.md` section 7 and
/// Sigilweaver/OpenSZRaw#28: confirmed present-but-empty on
/// `MTBLS12691/20210325_024.lcd`, a QQQ accession). Checking for this
/// substream rather than the bare root storage avoids misdetecting QQQ
/// files as `Qtfl`, the same trap `MASS_RAW_MS_DATA` above avoids for
/// `SingleQuad`.
const QTFL_CENTROID_INDEX: &str = "QTFL RawData/Centroid Index";

/// Detect which variant a file is by extension, and (for `.lcd`) by probing
/// which top-level CFBF storage is present. Never trusts the filename alone
/// for the `.lcd` IT-TOF vs QTOF distinction, per the format docs.
pub fn detect_variant<F: Read + std::io::Seek>(
    path_ext_lower: &str,
    comp: &mut CompoundFile<F>,
) -> crate::Result<Variant> {
    match path_ext_lower {
        "qgd" => {
            if comp.exists(GCMS_ROOT) {
                Ok(Variant::Qgd)
            } else {
                Err(crate::Error::Parse(format!(
                    "'{GCMS_ROOT}' storage not found in .qgd file"
                )))
            }
        }
        "lcd" => {
            if comp.exists(TTFL_ROOT) {
                Ok(Variant::Ttfl)
            } else if comp.exists(QTFL_CENTROID_INDEX) {
                Ok(Variant::Qtfl)
            } else if comp.exists(MASS_RAW_MS_DATA) {
                Ok(Variant::SingleQuad)
            } else if comp.exists(TLM_ROOT) {
                Err(crate::Error::Parse(format!(
                    "'{TLM_ROOT}' storage found - this is a QQQ/triple-quadrupole \
                     .lcd file, which openszraw does not decode yet (see \
                     Sigilweaver/OpenSZRaw#5)"
                )))
            } else {
                Err(crate::Error::Parse(format!(
                    "none of '{TTFL_ROOT}', '{QTFL_ROOT}', '{MASS_RAW_ROOT}', or \
                     '{TLM_ROOT}' storage found in .lcd file"
                )))
            }
        }
        other => Err(crate::Error::Parse(format!(
            "unsupported file extension '.{other}' (expected .qgd or .lcd)"
        ))),
    }
}

/// Read an entire CFBF stream into memory.
pub fn read_stream<F: Read + std::io::Seek>(
    comp: &mut CompoundFile<F>,
    path: &str,
) -> crate::Result<Vec<u8>> {
    let mut stream = comp
        .open_stream(path)
        .map_err(|e| crate::Error::Parse(format!("stream '{path}' not found: {e}")))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Read a CFBF stream into memory if it exists, returning `None` rather
/// than an error when it is absent (many streams are instrument/mode
/// specific and legitimately missing).
pub fn read_stream_opt<F: Read + std::io::Seek>(
    comp: &mut CompoundFile<F>,
    path: &str,
) -> Option<Vec<u8>> {
    let mut stream = comp.open_stream(path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Build an in-memory CFBF file with the given top-level storages
    /// (created empty, i.e. no substreams) plus any explicit streams.
    fn build_cfb(storages: &[&str], streams: &[&str]) -> CompoundFile<Cursor<Vec<u8>>> {
        let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        for storage in storages {
            comp.create_storage(storage).unwrap();
        }
        for stream in streams {
            comp.create_stream(stream).unwrap();
        }
        comp
    }

    #[test]
    fn qqq_tlm_file_with_boilerplate_empty_qtfl_storage_is_not_misdetected_as_qtof() {
        // Reproduces the MTBLS12691/20210325_024.lcd shape from
        // Sigilweaver/OpenSZRaw#28: an empty `QTFL RawData` storage (no
        // `Centroid Index`/`Centroid Data` substreams) alongside a
        // populated `TLM Raw Data` storage.
        let mut comp = build_cfb(&[QTFL_ROOT, TLM_ROOT], &[]);
        let err = detect_variant("lcd", &mut comp).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(TLM_ROOT),
            "error should name '{TLM_ROOT}': {msg}"
        );
        assert!(
            msg.to_lowercase().contains("not") || msg.to_lowercase().contains("unsupported"),
            "error should say the variant is unsupported, not a generic parse failure: {msg}"
        );
    }

    #[test]
    fn qtof_file_with_populated_qtfl_storage_is_detected_as_qtof() {
        let mut comp = build_cfb(&[QTFL_ROOT], &[QTFL_CENTROID_INDEX]);
        assert_eq!(detect_variant("lcd", &mut comp).unwrap(), Variant::Qtfl);
    }

    #[test]
    fn ttfl_takes_priority_even_if_boilerplate_qtfl_storage_is_present() {
        let mut comp = build_cfb(&[TTFL_ROOT, QTFL_ROOT], &[]);
        assert_eq!(detect_variant("lcd", &mut comp).unwrap(), Variant::Ttfl);
    }

    #[test]
    fn single_quad_file_is_detected_even_with_boilerplate_empty_qtfl_storage() {
        let mut comp = build_cfb(&[QTFL_ROOT, MASS_RAW_ROOT], &[MASS_RAW_MS_DATA]);
        assert_eq!(
            detect_variant("lcd", &mut comp).unwrap(),
            Variant::SingleQuad
        );
    }

    #[test]
    fn unrecognized_lcd_file_reports_all_known_roots() {
        let mut comp = build_cfb(&[], &[]);
        let err = detect_variant("lcd", &mut comp).unwrap_err();
        let msg = err.to_string();
        for root in [TTFL_ROOT, QTFL_ROOT, MASS_RAW_ROOT, TLM_ROOT] {
            assert!(msg.contains(root), "error should mention '{root}': {msg}");
        }
    }
}
