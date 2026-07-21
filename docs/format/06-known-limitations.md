# 06. Known Limitations

This document lists the known limitations of the Phase 4 Rust reader
(`crates/openszraw`) that are deliberate, documented gaps rather than
bugs - things investigated and found unrecoverable (or not yet decoded)
from the current understanding of the format, not things silently
guessed or fabricated. See the sibling projects' equivalent docs (e.g.
OpenSXRaw's legacy-TOF calibration gap, OpenARaw's
`docs/format/06-known-limitations.md`) for the precedent this follows.

## 1. IT-TOF (`.lcd` TTFL): index-to-m/z calibration (RESOLVED - see docs/format/03 section 3c)

**Resolves Sigilweaver/OpenSZRaw#1.** A prior session left the RLE
payload's index axis uncalibrated ("very likely raw digitizer/TOF
channel number", per `docs/format/03-lcd-ttfl-msdata.md`). This session
located and verified the file's own embedded calibration:
`TTFL Tuning/Tuning Result NN` stores a reference calibrant mass ladder
(identified as sodium formate cluster ions, a standard public ESI
calibration solution) and its measured flight times, which fit the
standard TOF law `time = a*sqrt(mz) + b` to ~1e-6 relative residual in
every one of 81 locally available IT-TOF files. See
`docs/format/03-lcd-ttfl-msdata.md` section 3c for the full derivation,
per-file `(a, b)` values, and the evidence that this also calibrates the
RLE payload's own index axis (order-of-magnitude match, plausible real-ion
m/z, and - most convincingly - a known calibrant background ion
recurring at a stable, tightly-clustered predicted index position across
dozens of scans, concentrated in the channel pair independently expected
to be positive polarity).

`crates/openszraw::reader::ttfl_spectra` now populates
`SpectrumRecord::mz` with `raw::ttfl::Calibration::mz(index)` when a
calibration was found (the common case), falling back to the raw,
uncalibrated index only if no `TTFL Tuning/Tuning Result NN` stream with
a usable calibration table is present in a given file - this has not
been observed in the local corpus, but the fallback is kept rather than
assuming every future file will carry one.

**What remains open**: the calibration's functional form and per-file
constants are confirmed to very high confidence (residual at
floating-point-noise level against the file's own reference masses,
correctly and stably localizing a known calibrant ion across an entire
run), but a small constant index-origin offset between the RLE payload's
own index convention and the tuning stream's time convention has not
been independently ruled out - see the "noise-tail caveat" in
`docs/format/03-lcd-ttfl-msdata.md` section 3c. Applying the calibration
to the rare, very large noise-tail index values documented below (up to
576,297) yields implausibly large m/z; this reflects those index
positions being electronic noise rather than real ions, not a flaw in
the calibration, and this crate does not filter or clamp them (that is
downstream peak-picking's job).

**Addendum to docs/format/03's magnitude estimate**: that doc
characterizes `total_span` (the highest reconstructed position) as
running "in the few-thousand to tens-of-thousands range." Corpus-wide
verification found real scans in `PXD020792/UY02-01-01p400.LCD` reaching
a time-bin index of **576,297** - almost an order of magnitude past
"tens of thousands." This reader does not assume any upper bound on the
index value.

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

## 9. Acquisition `start_timestamp` (RESOLVED - see docs/format/01-ole2-container.md addendum below)

**Resolves Sigilweaver/OpenSZRaw#9.** `RunMetadata::start_timestamp` was
hardcoded `None` for all three on-disk variants. The
`\x05SummaryInformation` OLE2 property set that carries this for `.wiff`
(OpenSXRaw) does not exist in `.lcd`/`.qgd` - confirmed directly with
`olefile.OleFileIO(...).get_metadata()` against one file from each of
MTBLS432 (IT-TOF), MSV000084197 (QTOF), and PXD034978 (`.qgd`).

Instead, every CFBF directory entry (storage or stream) carries its own
`created`/`modified` `FILETIME` fields per `[MS-CFB]` 2.6.4 - independent
of and not previously checked alongside `\x05SummaryInformation`. Every
real corpus file populates these, and LabSolutions writes nearly all of a
run's top-level storages within a fraction of a second of each other at
run start, so the earliest non-zero `created` value across the whole
container is a reliable acquisition-start proxy.

Verified via internal consistency only (no vendor software or output),
per `CONTRIBUTING.md`: across all 9 accessions (~150 files, `.qgd` and all
three `.lcd` families), the earliest per-file timestamp tracks sequential
injection order with regular, plausible batch cycle times -
`PXD025121`'s 29 sequentially numbered files land ~66m43s apart (only two
gaps, both an exact double-length, consistent with an operator break),
and `PXD019638`'s 22 files reconstruct a non-alphabetical, 4-way
interleaved injection order (`Br0`, `Br1`, `Br2`, `Br3` cycled in turn,
~68 minutes apart within each branch) purely from timestamps - a pattern
that could not fall out of a naive filename-based heuristic.

`crates/openszraw::raw::timestamp::earliest_created_timestamp` now
computes this at `Reader::open()` time via the `cfb` crate's own
`Entry::created()` (no raw `FILETIME` parsing needed - `cfb` already
converts to `SystemTime`), and `run_metadata()` populates
`start_timestamp` with it for all three variants. Confirmed against the
full local corpus: 139/139 files that open successfully populate a
timestamp (the remaining 12 are the pre-existing, unrelated MTBLS12691
QQQ/TLM open failure from section 7 above, not a gap in this feature).

**What remains open**: the earliest CFBF entry timestamp is a proxy for
"file/run creation," not a hardware-triggered "first scan acquired"
event - these are expected to differ by, at most, low single-digit
seconds (the time LabSolutions takes to initialize the container before
triggering the run), which is consistent with every other vendor crate in
this suite treating `start_timestamp` as a run/method-start proxy rather
than a to-the-microsecond experiment start.

## 10. PDA 3D Raw Data / LSS Raw Data chromatogram payload: not decoded, not wired into the reader

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

Seven same-day (2026-07-20) sessions of further clean-room analysis
narrowed the problem considerably without decoding it. Confirmed: 2 of
the 4 varying fields in the 112-byte `PDA 3D Raw Data/CheckSum` stream
are exact `u32` byte sizes of the `3D Raw Data` and `Max Plot` streams
(not a flat/real flag as previously read), and the "split" envelope
form's two declared-length regions are an exact
256-wavelength-channel/remainder split, which also explains why
"split" vs. "symmetric" envelope form correlates with wavelength
count. Ruled out, each time via actual value decoding plus a
physical-plausibility check and randomized controls rather than on
walk-rate alone: 19 CRC-16 polynomials and several count/derived-size
candidates for the remaining `CheckSum` fields; a fixed-width fp16
array; a block-floating-point/adaptive-scale hypothesis family
(including a candidate that passed two randomized-control checks but
proved to be a compensating-error artifact); two more signals surfaced
by re-running the decode sweeps with corrected per-region target
counts; an anti-mode-collapse cost term added to the joint
temporal+magnitude decoder (the underlying temporal-correlation signal
replicates in a more rigorous multi-pair test but is markedly weaker
than the original single-pair anecdote suggested); and, from a
deliberately manual byte-reading pass (rather than another automated
sweep), a "leading byte of a 3-byte token" hypothesis that traced to a
low-value-diversity metric artifact confirmed on three independent
files. Two genuine process improvements came out of this: a directly
quantified ~48% false-positive base rate for this document's
zero-leftover acceptance test, and a fix to the physical-plausibility
check itself (mode-dominated or low-diversity decodes can look
deceptively "smooth" under mean relative step alone). A seventh session
cross-referenced the PSI-MS/mzML open spec directly (per a specific ask
to look there rather than sweep more parameters of already-tried
byte-granular schemes): two MS-Numpress-inspired nibble-granular varint
encodings and literal zlib/DEFLATE framing of the payload, all ruled out
- one nibble scheme's only nonzero hit rate was disqualified by a
shuffled-byte control (80% of it survived byte-order scrambling) and
cross-file testing (collapsed on two sibling files), and the DEFLATE
framing's small hit rate proved statistically indistinguishable from
both a random-byte and a shuffled-byte control. See
`docs/format/04-lcd-chromatogram-pda.md`'s 2026-07-20 sessions 1-7 for
full detail. None of this decodes the per-value payload; that grammar
is still open.

## 11. LC Raw Data/Chromatogram Ch5/Ch6: `Ch6` decoded and wired in; `Ch5`'s numeric grammar is confirmed untestable from this corpus (PARTIALLY RESOLVES Sigilweaver/OpenSZRaw#21)

A separate, unrelated stream from section 10 above, despite sharing the
same outer 24-byte `RC\x00\x00` segment header - see
`docs/format/04-lcd-chromatogram-pda.md`'s "2026-07-21 session (LC Raw
Data Chromatogram Ch5/Ch6 decode)" for the full derivation. Two things
worth stating precisely, since they carry different confidence levels:

- **Byte-exact, zero-exception-verified (all 5 `PXD020792` corpus
  files):** the segment body is internally divided into `u16`
  length-prefixed/suffixed "pages" (the sub-segment structure
  `docs/format/04` previously flagged as not yet characterized), and a
  `(threshold=0x20, wide_width=2)` literal/wide-token tokenization rule
  decodes every page of every file's `Ch5` and `Ch6` stream to exactly
  its declared point count with zero leftover bytes. This part is not in
  question.
- **Strongly evidenced but not byte-exact-certain:** the specific
  numeric interpretation of a wide token (which byte holds the high
  bits, how many bits, sign convention) is corroborated by a
  physical-plausibility argument - cumulative-summing the decoded deltas
  produces a smooth, single-ramp-then-plateau chromatogram in all 5
  files, at roughly 12x lower mean per-sample delta magnitude than any
  alternative bit layout tried - rather than a second independent
  byte-exact proof the way the tokenization split itself has.

`crates/openszraw::raw::lc_chrom` implements the confirmed framing plus
the best-evidenced numeric interpretation, wired into `Reader`'s
`SpectrumSource::iter_chromatograms`. It deliberately does **not** emit a
chromatogram for `Ch5`: every one of `Ch5`'s 7200 samples, in every one
of the 5 locally available files, decodes to the exact same wide token
(raw value 512), which makes the delta-vs-absolute-value question
genuinely unanswerable from this corpus (a constant delta would integrate
into an unbounded ramp, contradicting `Ch5`'s documented flat/quiet
character - so either the wide-token formula is specifically wrong for
`Ch5`'s value, or `Ch5`'s repeated byte pair is a sentinel rather than a
generic magnitude token). `decode_stream` skips any channel whose decoded
tokens show fewer than 2 distinct values rather than guess, which also
means a future file whose `Ch5` genuinely varies would decode normally
without any code change.

This is intentionally left as a documented open question rather than
resolved by assumption, per `CONTRIBUTING.md`'s clean-room policy
("if you can only explain a field by having watched what LabSolutions
shows for it, don't write that down - keep digging in the bytes instead,
or flag it as unresolved"). Fetching a real, varying `Ch5`-equivalent
channel from a different accession (see
`docs/format/04`'s "Further avenues" section) is the concrete next step
that could resolve it.

## 12. Single-quadrupole (`.lcd` `Mass Raw Data`): a fourth on-disk variant (RESOLVED - see docs/format/07 "2026-07-21 session: full decode")

**Resolves Sigilweaver/OpenSZRaw#24.** Found while exploring metadata/
processing streams after #20's corpus widening added the first
single-quadrupole sample (`MTBLS1960`, Shimadzu LCMS-2020).
`detect_variant` now recognizes this variant's root storage (`Mass Raw
Data`) as `Variant::SingleQuad`, and `crates/openszraw::raw::mass_raw`
decodes `MS Raw Data`'s per-scan payload: a fixed 64-byte header
(scan index, retention time, an alternating - plausibly polarity -
flag, and a peak count) followed by `n_peaks` fixed-width `[mz: u16 *
10][intensity: LE uint]` records, with the intensity byte width derived
per-scan from `payload_size / n_peaks` rather than assumed. Verified
byte-exact against the file's own `TIC Data` stream (0 mismatches
across 19,200 scans, 8 corpus files).

**What remains open**: only one accession (`MTBLS1960`) represents this
variant - a same-session search for a second single-quadrupole LC-MS
source found none, so this decode is corroborated only by its internal
`TIC Data` cross-check, not (yet) by an independent file. The 0x10
header flag's alternating pattern is plausibly a polarity-switching
indicator (matching this dataset's own study-method description) but
has no confirmed mapping to a specific `Polarity` value, so
`SpectrumRecord::polarity` is left `None` for this variant, the same
treatment as the TTFL channel-mode flag in section 2 above.

## 13. Metadata and post-processing streams: an entire undocumented storage tree, not read anywhere

Every `.lcd` file carries dozens of top-level storages beyond raw
spectra and chromatogram/PDA data - instrument method/config
(`GUMM_Information`), post-acquisition compound-ID/quantitation results
(`Mass Data Processing`, confirmed to hold real per-run results via
embedded compound-name strings matching a study's actual biology), LC
detector processing parameters (`LSS Data Processing` and siblings),
report templates, and audit/system-check metadata. None of it is read
by `crates/openszraw` today. See
`docs/format/08-metadata-and-processing-streams.md` for the full
corpus-wide map and a prioritized list of what's worth decoding first -
`File Property`/`Method File Property` (plain XML, already decodable)
is the one piece of this list that's a small, contained win rather than
open-ended reverse engineering.
