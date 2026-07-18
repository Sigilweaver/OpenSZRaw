# 06. Known Limitations

This document lists the known limitations of the Phase 4 Rust reader
(`crates/openszraw`) that are deliberate, documented gaps rather than
bugs - things investigated and found unrecoverable (or not yet decoded)
from the current understanding of the format, not things silently
guessed or fabricated. See the sibling projects' equivalent docs (e.g.
OpenSXRaw's legacy-TOF calibration gap, OpenARaw's
`docs/format/06-known-limitations.md`) for the precedent this follows.

## 1. IT-TOF (`.lcd` TTFL): the reconstructed axis is a raw, uncalibrated time-bin index, not m/z

`docs/format/03-lcd-ttfl-msdata.md` documents the RLE payload's implicit
index axis as "very likely raw digitizer/TOF channel number," with no
calibration formula located. This session did not find one either -
`TTFL Instrument Param/MS Parameter` and `TTFL Tuning/*` streams exist in
every file and are the natural next place to look, but were not opened
this session (out of scope: the task was to ship a correct, honest
reader, not resolve every open format question).

`crates/openszraw::reader::ttfl_spectra` populates `SpectrumRecord::mz`
with this raw index directly (as `f64`), with a doc comment at the call
site making clear it is not calibrated m/z. This mirrors OpenSXRaw's
documented precedent for its own legacy-TOF uncalibrated-bin case
(`docs/format/04-legacy-wiff-calibration.md` / README: "m/z values are
raw, uncalibrated time-bin integers").

**Addendum to docs/format/03's magnitude estimate**: that doc
characterizes `total_span` (the highest reconstructed position) as
running "in the few-thousand to tens-of-thousands range." Corpus-wide
verification this session found real scans in
`PXD020792/UY02-01-01p400.LCD` reaching a time-bin index of **576,297**
- almost an order of magnitude past "tens of thousands." The doc's
estimate was apparently based on a smaller sample; this reader does not
assume any upper bound on the index value.

## 1b. IT-TOF (`.lcd` TTFL): `Data Index` entry_i and block-count assumptions were both wrong on a second accession

Found while running the full local corpus through `examples/corpus_scan.rs`
(9 of 31 `PXD025121` files initially failed to *open* at all): the
`Data Index` stream is not always an exact multiple of 64 bytes, and the
subset's own `entry_i` field can differ from its physical block
position. See the addendum in `docs/format/03-lcd-ttfl-msdata.md` for
the full byte-level evidence (`PXD025121/17.lcd`: 2 real channels per RT
point instead of 4, plus a trailing 32-byte partial block). Both are now
handled correctly in `crates/openszraw::raw::ttfl::parse_data_index`;
all 94 corpus files (31 of which are `PXD025121`) now open and pass
`assert_source_invariants` after the fix - see the corpus-scan results
in the Phase 4 session summary.

## 2. IT-TOF (`.lcd` TTFL): per-channel polarity/MS-level is not resolved

Each RT entry's `Data Index` carries 4 interleaved "channel" subsets
(`sub_i` 0-3), and `docs/format/03-lcd-ttfl-msdata.md` notes the 64-byte
scan header's `u32[5]` field is a constant `0x10000` or `0x10001`
depending on channel, "a coarse mode/polarity flag, not investigated
further." This session did not investigate it further either - there is
no confirmed mapping from that bit (or from `sub_i` itself) to a
specific `Polarity` or MS level per spectrum, despite dataset naming
(`..._pos-neg_NN.lcd`) implying the run alternates ionization polarity
across channels.

`crates/openszraw::reader::ttfl_spectra` therefore leaves
`SpectrumRecord::polarity` as `None` and `ms_level` as `1` for every
TTFL spectrum, rather than assign a channel-to-polarity/level mapping it
cannot back with decoded evidence.

## 3. QTOF (`.lcd` QTFL): MS2 precursor m/z is not decoded

Section "3. Data Encoding Variants" beyond `docs/format/05-qtfl-centroid.md`'s
original scope: this session found that `Centroid Index`'s `u32[5]`
field ("Segment/Event ID") is not an opaque per-scan tag as the doc's
wording suggests, but a real per-acquisition-cycle counter that starts
at 1 (the MS1 survey scan) and increments for each subsequent MS2
product-ion scan in that cycle (1 to 4 events seen per cycle in
`MSV000084197/20190607_NM16.lcd`, consistent with a DDA acquisition
selecting a variable number of precursors per cycle). The real
per-scan precursor m/z for those MS2 events lives in the separate
`QTFL RawData/DDA` stream (316 KB of real content in that file), which
was **not decoded this session** - out of scope for Phase 4's payload
decode work.

`crates/openszraw::reader::qtfl_spectra` classifies `ms_level` from
`event_id` (1 => MS1, >1 => MS2) since that pattern is well-supported by
the corpus evidence above, but populates MS2 spectra's
`PrecursorInfo` with only `precursor_native_id` (a reference to the most
recent MS1 scan), leaving `target_mz`/`selected_mz` as `None` rather
than fabricate a value. This satisfies
`openmassspec_core::conformance::assert_source_invariants`'s requirement
that MS2+ spectra carry *some* precursor information, without
overclaiming a decoded m/z that was not actually recovered.

## 4. QTOF (`.lcd` QTFL): intensity byte width is variable, corrected from docs/format/05

`docs/format/05-qtfl-centroid.md` states centroid intensities are always
16-bit (`N = payload_size / 10`). This is **incomplete**: this session
found the scan header's `u32[9]` field (byte offset `0x24`) encodes the
intensity byte width per scan (1, 2, or 4 bytes observed in the corpus),
with `N = payload_size / (8 + width)`. Verified by cross-checking
`max(intensity)` against the header's declared base-peak intensity
(`u32[4]`) across every non-empty scan in
`MSV000084197/20190607_NM16.lcd`: 0 mismatches out of 13,929 scans once
width is read from `u32[9]`, versus a **confirmed corrupt decode**
(spurious zero-interleaved intensities, a spurious sub-10-Da "m/z" among
real peaks, and a base-peak-intensity mismatch) on the file's
higher-dynamic-range MS2 scans when treating intensity as always 2
bytes wide. Width distribution in that file: 13,696 scans at 2 bytes,
218 at 4 bytes, 15 at 1 byte.

This is now correctly implemented in `crates/openszraw::raw::qtfl` and
covered by a conformance test
(`qtfl_centroid_mz_is_plausible_and_has_ms2`) and a unit test
(`decodes_a_wide_dynamic_range_scan_with_u32_intensity`). `docs/format/05`
itself has not been edited to keep its original CONFIRMED-for-the-
payload-shape claim intact; this file is the place recording the
correction.

## 5. GC-MS (`.qgd`): polarity and exact instrument model are not resolved

No polarity bit was found in the 32-byte `MS Raw Data` scan header (see
`docs/format/02-gcms-qgd-scans.md`'s field table) or the `Spectrum Index`
stream, so `SpectrumRecord::polarity` is always `None`. GC-EI-MS is
conventionally positive-ion, but that is a domain convention, not a
decoded field, so it is left unpopulated rather than assumed.

Similarly, no PSI-MS CV term dedicated to the `.qgd` file format itself
exists in `psi-ms.obo` (unlike `.lcd`, which has `MS:1003009` "Shimadzu
Biotech LCD format"), so `RunMetadata::source_file_format` falls back to
the generic `MS:1000560` "mass spectrometer file format" node with a
descriptive name. `RunMetadata::instrument` likewise stays at the
generic `MS:1000124` "Shimadzu instrument model" for all `.qgd` files,
since no per-file instrument-model string was found decoded anywhere in
the corpus (per-spectrum `Analyzer::SQMS` vs `Analyzer::TQMS` is
inferred from the scan's own decoded acquisition mode - profile vs MRM -
which is a real, evidenced distinction, not a guess).

## 6. QTOF (`.lcd` QTFL): sections 3 and 4 corroborated on a second independent source

`MTBLS14820` (10 files, a second, independent LCMS-9030 QTOF source
fetched during the 2026-07-18 corpus expansion pass) reproduces both
corrections below with 0 mismatches across ~66,300 checked scans - see
the addendum in `docs/format/05-qtfl-centroid.md` for the full detail.
This was previously verified against only one file
(`MSV000084197/20190607_NM16.lcd`).

## 7. QQQ (`.lcd`): a fourth on-disk variant now has corpus representation, but is not decoded, and is currently misdetected as QTOF

The 2026-07-18 corpus expansion pass added the first confirmed QQQ
(triple quadrupole) sample to the corpus: `MTBLS12691`, an LCMS-8060
system running MRM-targeted acquisition (Shimadzu's "LC/MS/MS Method
Package for Primary Metabolites v2", per the study's own methods
description). These files fail to open in the current reader.

Two distinct things are going on, worth separating clearly:

1. **The QQQ variant is not decoded at all.** Its real per-spectrum data
   lives under a `TLM Raw Data` storage (`TLM Raw Data/MS Raw Data`,
   `TLM Raw Data/Spectrum Index`, `TLM Raw Data/Retention Time`, plus
   status/TIC streams) - structurally closer to `.qgd` GC-MS's
   `MS Raw Data`/`Spectrum Index` naming than to either `TTFL Raw Data`
   or `QTFL RawData`, which makes sense: QQQ and (single/triple-quad)
   GC-MS share a quadrupole-based architecture, unlike the TOF-based
   IT-TOF/QTOF formats. Nothing in `crates/openszraw::raw` parses this
   storage - it is not read anywhere in the crate.
2. **`raw::detect_variant` actively misidentifies these files as QTOF**,
   producing a confusing error rather than a clear "unsupported"
   message. Every `.lcd` file - QQQ ones included - carries an
   always-present `QTFL RawData` storage as boilerplate, even when it
   has none of the substreams (`Centroid Index`, `Centroid Data`) that
   actually make a file QTOF. `detect_variant` checks `TTFL Raw Data`
   first, then treats *any* remaining `QTFL RawData` presence as
   sufficient to call it `Variant::Qtfl` - so a QQQ file with an empty
   `QTFL RawData` storage is misclassified as QTOF, and then fails with
   `stream 'QTFL RawData/Centroid Index' not found` instead of a clear
   "this is a QQQ/TLM file, not yet supported" message. Confirmed by
   listing the full CFBF storage tree of `MTBLS12691/20210325_024.lcd`:
   `QTFL RawData` is present and empty, `TLM Raw Data` is present and
   populated.

Neither is fixed in this pass (out of scope for a corpus/docs-only
session - implementing a new payload decoder deserves its own dedicated
clean-room analysis, not a rushed addition here).
[Sigilweaver/OpenSZRaw#5](https://github.com/Sigilweaver/OpenSZRaw/issues/5)
tracks both: correcting `detect_variant` to name the QQQ/TLM variant
explicitly (even before it's decoded, so the error message is honest
about what wasn't understood rather than misleading), and eventually
decoding `TLM Raw Data` itself using the now-available `MTBLS12691`
corpus sample (12 files fetched; ~296 available in total across the
study's five remote subdirectories, see `CORPUS.md`).

## 8. GC-MS/MS (`.qgd`, MTBLS11411): a third scan-header `format` value seen, not investigated

`docs/format/02-gcms-qgd-scans.md` documents two `Spectrum Index`/scan
layouts ("Variant A" profile, "Variant B" MRM), distinguished by index
width (u32 vs u64) and by where the format/event-ID fields land in the
32-byte scan header. `MTBLS11411` (GC/MS-TQ8050 NX, a GC-MS/MS triple
quad - a different, newer instrument generation than either of the
corpus's existing `.qgd` accessions) uses the Variant B u64 index, but
its scan header's offset-0x14 `format` field reads `3`, a value neither
Variant A (`2`) nor the existing Variant B description assigns any
meaning to. The existing reader decodes these files successfully anyway
(184,740 spectra/file, `assert_source_invariants` passes for all 5
fetched files) since it does not currently branch on this field's exact
value for Variant B - but the field's meaning for this instrument
generation is unresolved, not confirmed to be inert. Not investigated
further this session; flagged here rather than silently ignored.

## 9. PDA 3D Raw Data / LSS Raw Data chromatogram payload: not decoded, not wired into the reader

Unlike sections 1-5 above, this is not a gap in something the reader
implements - `PDA 3D Raw Data` and `LSS Raw Data` chromatogram streams
are not wired into `crates/openszraw` at all. See
`docs/format/04-lcd-chromatogram-pda.md` for the full write-up: the
segment envelope (24-byte `RC\x00\x00` header, plus a newly-confirmed
4-byte-or-8-byte length-checked wrapper around each segment's body) is
solid and exhaustively verified, but the per-point value encoding inside
that body remains undecoded despite a wide sweep of variable-length
integer and escape-byte hypotheses (Sigilweaver/OpenSZRaw#2). This is UV
detector / chromatogram data, not core MS spectra, so it does not block
MS-level format parity.
