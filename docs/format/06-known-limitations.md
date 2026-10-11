# 06. Known Limitations

This document lists the known limitations of the Rust reader
(`crates/openszraw`). These are documented gaps rather than bugs: things
found unrecoverable (or not yet decoded) from the current understanding
of the format, left unpopulated rather than guessed. See the sibling
projects' equivalent docs (e.g. OpenSXRaw's legacy-TOF calibration gap,
OpenARaw's `docs/format/06-known-limitations.md`) for the precedent this
follows.

## 1. IT-TOF (`.lcd` TTFL): index-to-m/z calibration

The RLE payload's index axis is calibrated from the file's own embedded
calibration (Sigilweaver/OpenSZRaw#1): `TTFL Tuning/Tuning Result NN`
stores a reference calibrant mass ladder (sodium formate cluster ions, a
standard public ESI calibration solution) and its measured flight times,
which fit the standard TOF law `time = a*sqrt(mz) + b` to ~1e-6 relative
residual in every one of 81 locally available IT-TOF files. See
`docs/format/03-lcd-ttfl-msdata.md` section 3c for the derivation,
per-file `(a, b)` values, and the evidence that this also calibrates the
RLE payload's own index axis (order-of-magnitude match, plausible real-ion
m/z, and a known calibrant background ion recurring at a stable predicted
index position across dozens of scans, concentrated in the channel pair
expected to be positive polarity).

`crates/openszraw::reader::ttfl_spectra` populates
`SpectrumRecord::mz` with `raw::ttfl::Calibration::mz(index)` when a
calibration is found, and falls back to the raw, uncalibrated index if no
`TTFL Tuning/Tuning Result NN` stream with a usable calibration table is
present. No file in the local corpus lacks one.

**Open: m/z not validated.** A physics check on sample spectra found no
isotope structure and no cluster of known background ions, and 21 of 29
sampled files contain unsorted m/z arrays. Treat IT-TOF m/z as
unverified until this is resolved.

**Open**: a small constant index-origin offset between the RLE payload's
index convention and the tuning stream's time convention has not been
independently ruled out - see the "noise-tail caveat" in
`docs/format/03-lcd-ttfl-msdata.md` section 3c.

**Index range**: real scans reach a time-bin index of 576,297
(`PXD020792/UY02-01-01p400.LCD`), well past the "few-thousand to
tens-of-thousands" range described in `docs/format/03`. The reader
assumes no upper bound. Calibrating these rare, very large noise-tail
indexes yields implausibly large m/z; they are electronic noise rather
than real ions, and this crate does not filter or clamp them (that is
downstream peak-picking's job).

## 1b. IT-TOF (`.lcd` TTFL): `Data Index` layout varies by channel count

The `Data Index` stream is not always an exact multiple of 64 bytes, and
a subset's own `entry_i` field can differ from its physical block
position. `PXD025121/17.lcd` has 2 real channels per RT point instead of
4 (so each 64-byte block packs two RT points) and ends in a trailing
32-byte partial block. `crates/openszraw::raw::ttfl::parse_data_index`
handles both; see `docs/format/03-lcd-ttfl-msdata.md` for the byte-level
evidence.

## 2. IT-TOF (`.lcd` TTFL): per-channel polarity/MS-level is not resolved

Each RT entry's `Data Index` carries up to 4 interleaved "channel" subsets
(`sub_i` 0-3), and the 64-byte scan header's `u32[5]` field is a constant
`0x10000` or `0x10001` depending on channel. There is no confirmed
mapping from that bit (or from `sub_i` itself) to a specific `Polarity`
or MS level per spectrum, despite dataset naming (`..._pos-neg_NN.lcd`)
implying the run alternates ionization polarity across channels.

`crates/openszraw::reader::ttfl_spectra` therefore leaves
`SpectrumRecord::polarity` as `None` and `ms_level` as `1` for every
TTFL spectrum.

## 3. QTOF (`.lcd` QTFL): MS2 precursor m/z is not decoded

`Centroid Index`'s `u32[5]` field ("Segment/Event ID") is a
per-acquisition-cycle counter that starts at 1 (the MS1 survey scan) and
increments for each MS2 product-ion scan in that cycle (1 to 4 events
per cycle in `MSV000084197/20190607_NM16.lcd`, consistent with a DDA
acquisition selecting a variable number of precursors per cycle). The
real per-scan precursor m/z for those MS2 events lives in the separate
`QTFL RawData/DDA` stream (316 KB in that file), which is not decoded.

`crates/openszraw::reader::qtfl_spectra` classifies `ms_level` from
`event_id` (1 => MS1, >1 => MS2) but populates MS2 spectra's
`PrecursorInfo` with only `precursor_native_id` (a reference to the most
recent MS1 scan), leaving `target_mz`/`selected_mz` as `None`. This
satisfies `openmassspec_core::conformance::assert_source_invariants`'s
requirement that MS2+ spectra carry some precursor information without
claiming a decoded m/z.

## 4. QTOF (`.lcd` QTFL): intensity byte width is variable

`docs/format/05-qtfl-centroid.md` describes centroid intensities as
16-bit (`N = payload_size / 10`). The scan header's `u32[9]` field (byte
offset `0x24`) gives the intensity byte width per scan (1, 2, or 4 bytes
in the corpus), with `N = payload_size / (8 + width)`. Verified by
cross-checking `max(intensity)` against the header's declared base-peak
intensity (`u32[4]`) across every non-empty scan in
`MSV000084197/20190607_NM16.lcd` (0 mismatches out of 13,929 scans;
13,696 at 2 bytes, 218 at 4 bytes, 15 at 1 byte); see section 6 for a
second source. Reading every scan as 2-byte produces a corrupt decode
on the higher-dynamic-range MS2 scans.

`crates/openszraw::raw::qtfl` reads the width from `u32[9]`, covered by
`qtfl_centroid_mz_is_plausible_and_has_ms2` (conformance) and
`decodes_a_wide_dynamic_range_scan_with_u32_intensity` (unit).

## 5. GC-MS (`.qgd`): polarity and exact instrument model are not resolved

No polarity bit is known in the 32-byte `MS Raw Data` scan header (see
`docs/format/02-gcms-qgd-scans.md`'s field table) or the `Spectrum Index`
stream, so `SpectrumRecord::polarity` is always `None`. GC-EI-MS is
conventionally positive-ion, but that is a domain convention, not a
decoded field.

No PSI-MS CV term dedicated to the `.qgd` file format exists in
`psi-ms.obo` (unlike `.lcd`, which has `MS:1003009` "Shimadzu Biotech LCD
format"), so `RunMetadata::source_file_format` uses the generic
`MS:1000560` "mass spectrometer file format" node with that node's exact
OBO name. `RunMetadata::instrument` stays at the generic `MS:1000124`
"Shimadzu instrument model" for all `.qgd` files, since no per-file
instrument-model string has been decoded (per-spectrum `Analyzer::SQMS`
vs `Analyzer::TQMS` is inferred from the scan's decoded acquisition mode,
profile vs MRM).

## 6. QTOF (`.lcd` QTFL): sections 3 and 4 hold on a second source

`MTBLS14820` (10 files, a second, independent LCMS-9030 QTOF source)
reproduces the cycle structure in section 3 and the variable intensity
width in section 4 with 0 mismatches across ~66,300 checked scans. See
`docs/format/05-qtfl-centroid.md` for detail.

## 7. QQQ (`.lcd` `TLM Raw Data`): not decoded

LC-MS/MS triple-quadrupole `.lcd` files (e.g. `MTBLS12691`, LCMS-8060,
MRM-targeted) store their per-spectrum data under a `TLM Raw Data`
storage (`MS Raw Data`, `Spectrum Index`, `Retention Time`, plus
status/TIC streams), structurally closer to `.qgd` GC-MS naming than to
`TTFL Raw Data` or `QTFL RawData`. Nothing in `crates/openszraw::raw`
parses it (Sigilweaver/OpenSZRaw#5). The corpus holds 5 QQQ accessions /
275 files across at least 3 instrument generations (see `CORPUS.md`).

These files fail at `Reader::open` with an explicit error naming
`TLM Raw Data` as an unsupported triple-quadrupole variant. Every `.lcd`
file carries an empty boilerplate `QTFL RawData` storage, so
`raw::detect_variant` requires `QTFL RawData/Centroid Index` before
classifying a file as QTOF; a unit test reproduces the
`MTBLS12691/20210325_024.lcd` storage shape.

## 8. GC-MS (`.qgd`): meaning of bit 31 in 4-byte profile intensities

Profile scans with header `Format` code `1` (4-byte intensities, seen in
`MTBLS11411`) set bit 31 on some of the most intense ions. Summed
intensities match the file's `TIC Data` only with that bit cleared, so
the reader clears it. It is consistent with a saturation marker but its
meaning is not confirmed, and it is not surfaced. Scans with any `Format`
code other than `0` (MRM), `1`, `2`, or `3` are reported as `Err` items
by `iter_spectra` and counted in `Reader::skipped_scans`.

## 9. Acquisition `start_timestamp` is a container-creation proxy

`.lcd`/`.qgd` files have no `\x05SummaryInformation` OLE2 property set
(the source of `start_timestamp` for `.wiff` in OpenSXRaw). Instead,
`crates/openszraw::raw::timestamp::earliest_created_timestamp` takes the
earliest non-zero `created` `FILETIME` across all CFBF directory entries
(`[MS-CFB]` 2.6.4, read via the `cfb` crate's `Entry::created()`), and
`run_metadata()` reports it for all variants (Sigilweaver/OpenSZRaw#9).
LabSolutions writes nearly all of a run's top-level storages within a
fraction of a second of each other at run start.

Verified by internal consistency only (no vendor software or output),
per `CONTRIBUTING.md`: across 9 accessions (~150 files, `.qgd` and all
three `.lcd` families), the earliest per-file timestamp tracks sequential
injection order with regular batch cycle times (`PXD025121`'s 29
sequentially numbered files land ~66m43s apart; `PXD019638`'s 22 files
reconstruct a 4-way interleaved injection order from timestamps alone).

**Open**: this is "file/run creation," not a hardware-triggered "first
scan acquired" event. The two are expected to differ by at most a few
seconds, consistent with other vendor crates in this suite treating
`start_timestamp` as a run-start proxy.

## 10. PDA 3D Raw Data / LSS Raw Data chromatogram payload: not decoded

`PDA 3D Raw Data` and `LSS Raw Data` chromatogram streams are not wired
into `crates/openszraw` (Sigilweaver/OpenSZRaw#2). The segment envelope
(24-byte `RC\x00\x00` header plus a 4-byte-or-8-byte length-checked
wrapper around each segment's body) is verified, but the per-point value
encoding inside that body is not decoded. This is UV detector /
chromatogram data, not MS spectra.

Known facts: 2 of the 4 varying fields in the 112-byte
`PDA 3D Raw Data/CheckSum` stream are exact `u32` byte sizes of the
`3D Raw Data` and `Max Plot` streams, and the "split" envelope form's two
declared-length regions are an exact 256-wavelength-channel/remainder
split. Ruled out (by value decoding with physical-plausibility checks and
randomized controls): 19 CRC-16 polynomials for the remaining `CheckSum`
fields, a fixed-width fp16 array, block-floating-point/adaptive-scale
encodings, a "leading byte of a 3-byte token" scheme, two
MS-Numpress-inspired nibble varints, and zlib/DEFLATE framing. The
zero-leftover acceptance test used in that work has a measured ~48%
false-positive base rate. See `docs/format/04-lcd-chromatogram-pda.md`
for detail.

## 11. LC Raw Data/Chromatogram Ch5/Ch6: `Ch5` is not emitted

`crates/openszraw::raw::lc_chrom` decodes `LC Raw Data/Chromatogram`
streams and `Reader` exposes them through
`SpectrumSource::iter_chromatograms` (Sigilweaver/OpenSZRaw#21); see
`docs/format/04-lcd-chromatogram-pda.md` for the derivation. Confidence
differs by layer:

- **Byte-exact (all 5 `PXD020792` files):** the segment body is divided
  into `u16` length-prefixed/suffixed pages, and a
  `(threshold=0x20, wide_width=2)` literal/wide-token rule decodes every
  page of `Ch5` and `Ch6` to exactly its declared point count with zero
  leftover bytes.
- **Strongly evidenced, not byte-exact:** the numeric interpretation of a
  wide token (byte order of the high bits, bit count, sign). Cumulative
  summing gives a smooth single-ramp-then-plateau chromatogram in all 5
  files, at roughly 12x lower mean per-sample delta than any alternative
  bit layout tried.

No chromatogram is emitted for `Ch5`: every one of its 7200 samples, in
every local file, decodes to the same wide token (raw value 512), so the
delta-vs-absolute question cannot be answered from this corpus.
`decode_stream` skips any channel whose decoded tokens show fewer than 2
distinct values, so a file whose `Ch5` varies decodes without a code
change. A varying `Ch5`-equivalent channel from another accession would
resolve it (see `docs/format/04`'s "Further avenues").

## 12. Single-quadrupole (`.lcd` `Mass Raw Data`): one source, polarity flag unmapped

`detect_variant` recognizes the single-quadrupole root storage
(`Mass Raw Data`) as `Variant::SingleQuad`, and
`crates/openszraw::raw::mass_raw` decodes it (Sigilweaver/OpenSZRaw#24):
a fixed 64-byte header (scan index, retention time, an alternating flag,
and a peak count) followed by `n_peaks` `[mz: u16 * 10][intensity: LE
uint]` records, with the intensity width derived per scan from
`payload_size / n_peaks`. Verified byte-exact against the file's own
`TIC Data` stream (0 mismatches across 19,200 scans, 8 files). See
`docs/format/07-mass-raw-data-single-quad.md`.

**Open**: only one accession (`MTBLS1960`, LCMS-2020) represents this
variant, so the decode is corroborated only by its internal `TIC Data`
cross-check. The 0x10 header flag's alternating pattern is plausibly a
polarity-switching indicator but has no confirmed mapping, so
`SpectrumRecord::polarity` is `None` for this variant (same treatment as
the TTFL channel flag in section 2).

## 13. Metadata and post-processing streams: not read

Every `.lcd` file carries dozens of top-level storages beyond raw
spectra and chromatogram/PDA data: instrument method/config
(`GUMM_Information`), post-acquisition compound-ID/quantitation results
(`Mass Data Processing`), LC detector processing parameters
(`LSS Data Processing` and siblings), report templates, and
audit/system-check metadata. None of it is read by `crates/openszraw`.
See `docs/format/08-metadata-and-processing-streams.md` for the
corpus-wide map and a prioritized list; `File Property`/`Method File
Property` (plain XML) is the smallest contained piece.

## 14. QTOF (`.lcd` QTFL): m/z transform is wrong

`crates/openszraw::reader::qtfl_spectra` decodes each stored `u64` value
as `mz = value / 1e12`. This reproduces the header's base-peak intensity
on every non-empty scan, but not physical m/z: in two independent
accessions (MSV000084197, MTBLS13204), isotope partners of intense peaks
are spaced about 1100/m Da apart (1.86 Da at m/z 592, 1.51 at 729, 1.04
at 1062) instead of 1.0034 Da. The stored value is not linear in true
m/z, and every QTOF file is affected. The correct transform is not yet
known. See `docs/format/05-qtfl-centroid.md`.
