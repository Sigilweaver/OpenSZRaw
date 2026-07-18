# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust reader (`openszraw`) for Shimadzu LabSolutions raw data,
  covering `.qgd` GC-MS (full-scan profile and MRM/targeted acquisition),
  `.lcd` IT-TOF (run-length-encoded profile spectra over a raw,
  uncalibrated time-bin axis), and `.lcd` QTOF (centroid).
- Full CFBF/OLE2 stream catalog and per-format payload decoding,
  documented in `docs/format/`.
- `examples/corpus_scan.rs` for running the reader across a full local
  corpus and reporting per-file pass/fail and spectrum counts.

### Known limitations

- IT-TOF m/z values are a raw, uncalibrated time-bin index, not
  physical m/z - no calibration formula has been located yet.
- IT-TOF per-channel polarity/MS-level is not resolved; every TTFL
  spectrum reports `ms_level = 1` and `polarity = None`.
- QTOF MS2 spectra carry a precursor reference but not a decoded
  precursor m/z (`QTFL RawData/DDA` is not yet decoded).
- `.qgd` polarity is not populated; no polarity bit has been found in
  the scan header or `Spectrum Index` stream.
- PDA/chromatogram stream decoding (bit-packed delta encoding) is not
  yet solved.
- Python bindings are not yet implemented.

See [docs/format/06-known-limitations.md](docs/format/06-known-limitations.md)
for the full detail behind each of these.
