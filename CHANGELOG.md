# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-10-11

### Breaking

This release makes the public API extensible so later fixes and new
fields can ship as minor releases. Every API change:

- Requires `openmassspec-core` 3.0.0 (was 2.0.0). See core's
  `MIGRATING-3.0.md` for the shared-type changes.
- `SpectrumSource::iter_spectra` on `Reader` yields
  `openmassspec_core::Result<SpectrumRecord>`. A source scan that cannot
  be decoded is an `Err` item at its position (it was left out
  silently); iteration continues with the next scan.
  `iter_chromatograms` yields `Result<ChromatogramRecord>` (always
  `Ok`). Write `let spectrum = spectrum?;` in loops, or collect into
  `openmassspec_core::Result<Vec<_>>`.
- As a result `openmassspec_core::write_mzml` stops at the first
  undecodable scan. Files that previously exported with a few QTOF
  scans missing now return an error unless written with
  `RecordErrorPolicy::Skip`.
- New `Error::ScanDecode { source_index, message }` (itself
  `#[non_exhaustive]`). It reaches callers wrapped in
  `openmassspec_core::Error::Vendor`; `From<openszraw::Error> for
  openmassspec_core::Error` is implemented (`Io` maps to core `Io`).
- `openszraw::Error` is `#[non_exhaustive]`. Exhaustive `match`es on it
  need a wildcard arm.
- The `raw` module is now private. Removed from the public API:
  `raw::detect_variant`, `raw::read_stream`, `raw::read_stream_opt`,
  `raw::parse_u32_array`, `raw::timestamp::earliest_created_timestamp`
  (these exposed `cfb::CompoundFile`), and every item of `raw::qgd`,
  `raw::qtfl`, `raw::ttfl`, `raw::mass_raw` and `raw::lc_chrom`
  (parsers, scan/record structs, header-size constants). Use `Reader`.
- `raw::Variant` moves to `openszraw::Variant` and
  `raw::ttfl::Calibration` to `openszraw::Calibration`. Both are
  `#[non_exhaustive]` (`Calibration` fields stay public for reading).
  `Variant` gains `as_str()` and `Hash`.
- `reader::ScanIndexInfo` is `#[non_exhaustive]` (fields stay public for
  reading).
- `Reader::stem` is no longer a public field; use `Reader::stem()`.
- `Reader::skipped_scans()` counts the `Err` items of the last
  `iter_spectra` call.
- Python: `RawReader(path)` and `iter_spectra(path)` raise
  `RuntimeError` for a scan that cannot be decoded (it was left out
  silently). Both take `skip_errors=False`; pass `skip_errors=True` to
  leave such scans out, and read the count from
  `RawReader.skipped_scans`. `read_chromatograms` raises on an error
  item. Enum labels for core variants added after this release fall
  back to core's `as_str()`.
- Python `RawReader.read_spectrum` raises `IndexError` (was
  `RuntimeError`) for an out-of-range scan index.

### Fixed

- `.qgd` run metadata now pairs `MS:1000560` with its exact PSI-MS name,
  "mass spectrometer file format", instead of the made-up name
  "Shimadzu GCMSsolution QGD format". mzML validators reject an accession
  with a mismatched name. psi-ms.obo has no `.qgd`-specific term. A unit
  test now checks every emitted CV term name against the OBO.
- GC-MS full-scan profile scans with 3- or 4-byte intensities are now
  decoded instead of skipped. The header `Format` code selects the width
  (`2` -> 2 bytes, `3` -> 3 bytes, `1` -> 4 bytes with bit 31 masked).
  Decoded sums match the file's own TIC for all 391,625 profile scans in
  the local corpus. Previously 59,403 scans across PXD019638 and
  MTBLS11411 were dropped.
- IT-TOF RLE prefix search is now linear time. A crafted 64 KB scan
  payload took about 6 s to reject (quadratic search); it now takes
  under 1 ms. Results are unchanged (checked against the old search by a
  randomized test).
- A scan that cannot be decoded no longer renumbers every scan after it.
  `scan_number` and `native_id` now come from the scan's position in the
  source index (GC-MS MRM: its transition slots, reserved from the
  header even when the payload is bad). `index` stays the dense output
  position. Output is unchanged for files with no skipped scans.
- The reader no longer hides stream and metadata problems: an existing
  CFBF stream that cannot be opened or read, a missing TTFL calibration
  (raw index used as m/z), and spectra with no `Retention Time` record
  (reported as 0) are logged at `warn`; an LC chromatogram stream that
  does not decode is logged at `debug`. `corpus_scan` reports skipped
  scans and malformed `index.csv` rows.

### Added

- `Reader::skipped_scans()` reports how many source scans the last
  `iter_spectra` call could not decode. Stream and metadata problems are
  logged through the `log` crate. Adds a `log` dependency.
- Python `RawReader.skipped_scans` reports the scans left out under
  `skip_errors=True`.
- Corpus-backed tests find fixtures through `OPENSZRAW_CORPUS` (was a
  hardcoded path) and fail instead of skipping when `REQUIRE_CORPUS=1`.
  CI sets both for the GC-MS fixtures it downloads, verifies them by
  sha256, and retries downloads (`curl -fsSL --retry 3`).
- Committed `Cargo.lock`, plus `.cargo/config.toml` so lockfile updates
  stay within the MSRV.

### Changed

- MSRV raised from 1.85 to 1.88, matching `openmassspec-core` 2.0.0,
  which already required 1.88.

## [0.2.0] - 2026-09-28

### Added

- Python spectra and run metadata now expose every shared record field.
  Added bounded iteration, chromatograms, variant, calibration, and scan
  index access. QTOF event IDs and cycle IDs and IT-TOF channel IDs are
  preserved in shared records.

### Changed

- Adopts `openmassspec-core` 2.0.0 (arrow 60). Rust users building against
  core 1.x must upgrade core too.
- `cfb` 0.14 -> 0.15.

### Fixed

- GC-MS MRM spectra now carry the decoded `event_id` in
  `acquisition_event_id`, instead of it being decoded, unit-tested, and
  then discarded before reaching the per-transition `SpectrumRecord`s.
  Resolves Sigilweaver/OpenSZRaw#27.

## [0.1.4] - 2026-08-12

### Changed

- Adopt `openmassspec-core` 1.5.0. The new `SpectrumRecord`/`RunMetadata`
  `extra` and `acquisition_event_id` fields are defaulted (empty / `None`);
  no field is populated yet, so output is unchanged.

### Added

- CI now downloads the two `.qgd` GC-MS corpus fixtures (PXD019638
  full-scan profile, PXD034978 MRM) ahead of `cargo test`, so
  `crates/openszraw/tests/conformance.rs`'s `qgd_profile_*`/`qgd_mrm_*`
  tests exercise real decode paths in CI instead of always skipping.
  Resolves Sigilweaver/OpenSZRaw#32.

### Fixed

- QTOF MS2 spectra now link to the MS1 survey scan with the same decoded
  acquisition `cycle_index`, instead of relying on the most recently emitted
  MS1 scan.

## [0.1.3] - 2026-07-29

### Changed

- Bumped `openmassspec-core` to 1.4.0, which added
  `acquisition_software_name`/`acquisition_software_version` to
  `RunMetadata`. Neither is decoded here across any of the four
  variants (Qgd, Qtfl, Ttfl, SingleQuad), so both default to `None` to
  keep building against the new core version. Resolves
  Sigilweaver/OpenSZRaw#30.

### Fixed

- `raw::detect_variant` no longer misdetects QQQ (triple-quadrupole,
  e.g. LCMS-8060) `.lcd` files as QTOF. Every `.lcd` file - QQQ ones
  included - carries an always-present but empty `QTFL RawData` storage
  as boilerplate; detection now requires the QTOF-only
  `QTFL RawData/Centroid Index` substream before classifying a file as
  `Variant::Qtfl`, the same substream-presence check already used to
  protect the `SingleQuad` classification from the identical trap. A
  QQQ file now fails with a clear message naming `TLM Raw Data` and
  stating that the variant is not decoded yet, instead of a confusing
  `stream 'QTFL RawData/Centroid Index' not found` error. Full MRM/QQQ
  payload decoding is still not implemented; see
  `docs/format/06-known-limitations.md` #7 and
  Sigilweaver/OpenSZRaw#5. Addresses Sigilweaver/OpenSZRaw#5;
  resolves Sigilweaver/OpenSZRaw#28.

  Contributed by @Nabejo.

## [0.1.2] - 2026-07-25

### Fixed

- Adapted `RunMetadata` construction (all four format variants) to
  `openmassspec-core` 1.3.0's new `analyzers`/`instrument_serial_number`
  fields (defaulted, as neither is decoded here).
- Declared `openmassspec-core` minimum was still `"1.0.0"`; now that the
  code needs 1.3.0's new fields to compile, bumped the declared minimum
  to `"1.3.0"` to match.

## [0.1.1] - 2026-07-20

### Added

- `RunMetadata::start_timestamp` is now populated for all three on-disk
  variants (`.qgd`, `.lcd` IT-TOF, `.lcd` QTOF), sourced from the CFBF
  container's own per-entry directory creation timestamps rather than
  the `\x05SummaryInformation` stream (which `.lcd`/`.qgd` don't carry -
  see `docs/format/06-known-limitations.md` #9). Resolves
  Sigilweaver/OpenSZRaw#9.
- `LC Raw Data/Chromatogram Ch6` (a conventional UV/RID-style LC detector
  channel, distinct from the still-undecoded `PDA 3D Raw Data`) is now
  decoded and exposed via `SpectrumSource::iter_chromatograms`, tagged
  with PSI-MS `MS:1000811` "electromagnetic radiation chromatogram" - see
  `docs/format/04-lcd-chromatogram-pda.md` and
  `docs/format/06-known-limitations.md` #11. `Ch5` shares the same
  verified page/tokenization framing but is deliberately left undecoded:
  every sample in every locally available file is the same single
  repeated value, which makes its numeric grammar (delta vs. absolute
  convention) untestable from this corpus rather than resolved. Partially
  resolves Sigilweaver/OpenSZRaw#21.

  Contributed by @Nabejo.

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
