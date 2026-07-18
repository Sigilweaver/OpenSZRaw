//! Rust reader for Shimadzu LabSolutions mass spectrometry raw data.
//!
//! Supports three on-disk variants, all OLE2/CFBF containers (see
//! `docs/format/01-ole2-container.md`):
//!
//! - `.qgd` GCMSsolution GC-MS data (`GCMS Raw Data` storage) - full-scan
//!   profile spectra and MRM/targeted transitions, see
//!   `docs/format/02-gcms-qgd-scans.md`.
//! - `.lcd` LabSolutions LC-MS, IT-TOF family (`TTFL Raw Data` storage) -
//!   run-length-encoded sparse profile spectra over a raw, uncalibrated
//!   time-bin axis, see `docs/format/03-lcd-ttfl-msdata.md` and
//!   `docs/format/06-known-limitations.md`.
//! - `.lcd` LabSolutions LC-MS, QTOF family (`QTFL RawData` storage) -
//!   calibrated centroid spectra, see `docs/format/05-qtfl-centroid.md`.
//!
//! `.lcd` files are dispatched between IT-TOF and QTOF by checking which
//! top-level CFBF storage is present, never by filename. `PDA 3D Raw Data`
//! (secondary UV detector) is out of scope for this crate, see
//! `docs/format/04-lcd-chromatogram-pda.md`.

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
