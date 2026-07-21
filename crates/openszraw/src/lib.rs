//! Rust reader for Shimadzu LabSolutions mass spectrometry raw data.
//!
//! Supports four on-disk variants, all OLE2/CFBF containers (see
//! `docs/format/01-ole2-container.md`):
//!
//! - `.qgd` GCMSsolution GC-MS data (`GCMS Raw Data` storage) - full-scan
//!   profile spectra and MRM/targeted transitions, see
//!   `docs/format/02-gcms-qgd-scans.md`.
//! - `.lcd` LabSolutions LC-MS, IT-TOF family (`TTFL Raw Data` storage) -
//!   run-length-encoded sparse profile spectra, calibrated to physical
//!   m/z from the file's own embedded TOF tuning data, see
//!   `docs/format/03-lcd-ttfl-msdata.md` and
//!   `docs/format/06-known-limitations.md`.
//! - `.lcd` LabSolutions LC-MS, QTOF family (`QTFL RawData` storage) -
//!   calibrated centroid spectra, see `docs/format/05-qtfl-centroid.md`.
//! - `.lcd` LabSolutions LC-MS, single-quadrupole family (`Mass Raw Data`
//!   storage, e.g. Shimadzu LCMS-2020) - full-scan profile spectra, see
//!   `docs/format/07-mass-raw-data-single-quad.md`.
//!
//! `.lcd` files are dispatched between these families by checking which
//! top-level CFBF storage is present, never by filename. `PDA 3D Raw Data`
//! (secondary UV detector) is out of scope for this crate, see
//! `docs/format/04-lcd-chromatogram-pda.md`.
//!
//! A separate, unrelated stream - `LC Raw Data/Chromatogram ChN`
//! (conventional UV/RID-style LC detector channels) - *is* decoded and
//! exposed via [`openmassspec_core::SpectrumSource::iter_chromatograms`],
//! see `raw::lc_chrom` and `docs/format/04-lcd-chromatogram-pda.md`'s
//! "LC Raw Data Chromatogram Ch5/Ch6 decode" section.

#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]

pub mod raw;
pub mod reader;

pub use reader::Reader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, Error>;
