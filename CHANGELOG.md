# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust reader (`openszraw`) for Shimadzu LabSolutions raw data,
  covering `.qgd` GC-MS (full-scan profile and MRM/targeted acquisition),
  `.lcd` IT-TOF (run-length-encoded profile spectra, calibrated to
  physical m/z), and `.lcd` QTOF (centroid).
- Full CFBF/OLE2 stream catalog and per-format payload decoding,
  documented in `docs/format/`.
- `examples/corpus_scan.rs` for running the reader across a full local
  corpus and reporting per-file pass/fail and spectrum counts.
- Python bindings via a new `openszraw-py` PyO3 crate, exposing
  `RawReader` and `Spectrum` to mirror the sibling readers' Python API.
  Packaged as `openszraw` on PyPI; wheels (Linux/macOS/Windows) and an
  sdist build and publish from the release workflow.

### Fixed

- IT-TOF (`.lcd` TTFL) `SpectrumRecord::mz` now reports calibrated
  physical m/z instead of the raw digitizer/time-bin index, using a
  per-file calibration parsed from the file's own `TTFL Tuning/Tuning
  Result NN` stream (identified as a sodium formate cluster ion
  reference ladder fit to the standard TOF `time = a*sqrt(mz) + b` law).
  See [docs/format/03-lcd-ttfl-msdata.md](docs/format/03-lcd-ttfl-msdata.md)
  section 3c for the full derivation and evidence. Resolves
  [#1](https://github.com/Sigilweaver/OpenSZRaw/issues/1).

### Known limitations

- IT-TOF per-channel polarity/MS-level is not resolved; every TTFL
  spectrum reports `ms_level = 1` and `polarity = None`.
- QTOF MS2 spectra carry a precursor reference but not a decoded
  precursor m/z (`QTFL RawData/DDA` is not yet decoded).
- `.qgd` polarity is not populated; no polarity bit has been found in
  the scan header or `Spectrum Index` stream.
- PDA/chromatogram stream decoding (bit-packed delta encoding) is not
  yet solved.

See [docs/format/06-known-limitations.md](docs/format/06-known-limitations.md)
for the full detail behind each of these.
