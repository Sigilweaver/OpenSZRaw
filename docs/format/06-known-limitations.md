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
