# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `RunMetadata::start_timestamp` is now populated for all three on-disk
  variants (`.qgd`, `.lcd` IT-TOF, `.lcd` QTOF), sourced from the CFBF
  container's own per-entry directory creation timestamps rather than
  the `\x05SummaryInformation` stream (which `.lcd`/`.qgd` don't carry -
  see `docs/format/06-known-limitations.md` #9). Resolves
  Sigilweaver/OpenSZRaw#9.

### Documentation

- Further PDA/chromatogram payload investigation
  (Sigilweaver/OpenSZRaw#2, contributed by @Nabejo): identified 2 of the
  4 varying fields in `PDA 3D Raw Data/CheckSum` as exact stream byte
  sizes (correcting an earlier "flat vs. real flag" reading), ruled out
  a 19-polynomial CRC-16 sweep (plus Fletcher/Adler/plain-sum) for the
  remaining 2 fields, ruled out a fixed-width fp16 (binary16) array, and
  clarified that the fp16 and spectral-domain-delta ideas can only
  function as validators for a token-boundary rule, not as independent
  framing searches. The per-point payload grammar itself is still
  undecoded - see `docs/format/04-lcd-chromatogram-pda.md`'s 2026-07-20
  session for full detail.

## [0.1.0] - 2026-07-18

### Added

- Corpus expansion (local validation corpus, not shipped): grew from
  94 files/~960 MB/6 accessions to 151 files/~2.9 GB/9 accessions. New:
  a second independent QTOF (LCMS-9030) source (`MTBLS14820`, 10 files),
  the corpus's first QQQ/triple-quad sample (`MTBLS12691`, LCMS-8060
  MRM, 12 files - not yet decodable by the reader, see
  `docs/format/06-known-limitations.md` #7), a third GC-MS(/MS)
  accession on a newer instrument generation (`MTBLS11411`, GC/MS-TQ8050
  NX, 5 files), and a broadened `MTBLS432` (15 -> 45 of 93 available
  files). `CORPUS.md`'s accession table now tracks instrument family and
  acquisition mode per accession, not just container format.

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
  yet solved. The segment envelope around each undecoded body is now
  confirmed (see `docs/format/04-lcd-chromatogram-pda.md`), and several
  additional value-encoding hypotheses were tried and ruled out, but the
  per-point payload itself remains open (Sigilweaver/OpenSZRaw#2).

See [docs/format/06-known-limitations.md](docs/format/06-known-limitations.md)
for the full detail behind each of these.
