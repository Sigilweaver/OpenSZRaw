//! Low-level parsing modules for `.qgd` and `.lcd` files.

pub mod lc_chrom;
pub mod qgd;
pub mod qtfl;
pub mod timestamp;
pub mod ttfl;

use std::io::Read;

use cfb::CompoundFile;

/// The three on-disk variants this crate can decode, detected at `open()`
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
}

/// Root CFBF storage name for each variant.
const GCMS_ROOT: &str = "GCMS Raw Data";
const TTFL_ROOT: &str = "TTFL Raw Data";
const QTFL_ROOT: &str = "QTFL RawData";

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
            } else if comp.exists(QTFL_ROOT) {
                Ok(Variant::Qtfl)
            } else {
                Err(crate::Error::Parse(format!(
                    "neither '{TTFL_ROOT}' nor '{QTFL_ROOT}' storage found in .lcd file"
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
