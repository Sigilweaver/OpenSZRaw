# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-07-20

### Added

- `RunMetadata::start_timestamp` is now populated for all three on-disk
  variants (`.qgd`, `.lcd` IT-TOF, `.lcd` QTOF), sourced from the CFBF
  container's own per-entry directory creation timestamps rather than
  the `\x05SummaryInformation` stream (which `.lcd`/`.qgd` don't carry -
  see `docs/format/06-known-limitations.md` #9). Resolves
  Sigilweaver/OpenSZRaw#9.

### Documentation

- Further PDA/chromatogram payload investigation
  (Sigilweaver/OpenSZRaw#2, contributed by @Nabejo): seven same-day
  sessions of clean-room analysis. Confirmed findings: 2 of the 4
  varying `PDA 3D Raw Data/CheckSum` fields are exact stream byte sizes
  (correcting an earlier "flat vs. real flag" reading), and the
  "split" envelope form's two regions are an exact
  256-wavelength-channel/remainder split (also explaining why "split"
  vs. "symmetric" form correlates with wavelength count). Ruled out, in
  each case run through actual value decoding, a physical-plausibility
  check, and randomized controls rather than reported on walk-rate
  alone: a 19-polynomial CRC-16 sweep and several count/derived-size
  candidates for the remaining `CheckSum` fields; a fixed-width fp16
  array; a block-floating-point/adaptive-scale hypothesis family; a
  region-tail marker-bit signal that passed two randomized controls but
  traced to a compensating-error artifact; two more dramatic-looking
  signals surfaced by re-running sweeps with corrected per-region
  target counts; an anti-mode-collapse cost term added to the joint
  temporal+magnitude decoder; from a deliberately manual (not
  automated-sweep) byte-reading pass, a "leading byte of a 3-byte
  token" hypothesis that traced to the same low-value-diversity metric
  artifact on three independent files; and, cross-referencing the
  PSI-MS/mzML open spec directly, two MS-Numpress-inspired
  nibble-granular varint encodings and literal zlib/DEFLATE framing of
  the payload - the one nonzero hit rate found (a nibble scheme on one
  file) was disqualified by a shuffled-byte control and cross-file
  testing, and DEFLATE's small hit rate proved statistically
  indistinguishable from random-byte and shuffled-byte controls. Along
  the way, quantified a ~48% false-positive base rate for this
  document's zero-leftover acceptance test and fixed a real gap in the
  physical-plausibility check itself (mode-dominated/low-diversity
  decodes can look deceptively "smooth" under mean relative step
  alone). The per-point payload grammar itself is still undecoded - see
  `docs/format/04-lcd-chromatogram-pda.md`'s 2026-07-20 sessions 1-7 for
  full detail.

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
