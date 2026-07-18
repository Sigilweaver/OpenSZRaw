---
sidebar_position: 98
---

# Changelog

The canonical changelog lives at
[`CHANGELOG.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CHANGELOG.md)
in the repository root. The notes below mirror the latest state.

## Unreleased

Not yet published to crates.io or PyPI.

- Initial Rust reader (`openszraw`) for Shimadzu LabSolutions raw data,
  covering `.qgd` GC-MS (full-scan profile and MRM/targeted
  acquisition), `.lcd` IT-TOF (run-length-encoded profile spectra over a
  raw, uncalibrated time-bin axis), and `.lcd` QTOF (centroid).
- Full CFBF/OLE2 stream catalog and per-format payload decoding,
  documented in [Format specification](./format/overview).
- `examples/corpus_scan.rs` for running the reader across a full local
  corpus and reporting per-file pass/fail and spectrum counts.
- Python bindings via a new `openszraw-py` PyO3 crate, exposing
  `RawReader` and `Spectrum` to mirror the sibling readers' Python API
  (see [Python API](./guide/python-api)). Not yet packaged for PyPI.

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

See [Known limitations](./format/known-limitations) for the full detail
behind each of these.
