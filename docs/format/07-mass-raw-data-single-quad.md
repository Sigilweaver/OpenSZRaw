# 07. Single-Quadrupole (`.lcd` `Mass Raw Data`): a fourth, now-decoded on-disk variant

**Status**: DECODED (resolves Sigilweaver/OpenSZRaw#24). `detect_variant`
now recognizes `Mass Raw Data` as `Variant::SingleQuad`, and
`crates/openszraw::raw::mass_raw` decodes `MS Raw Data`'s per-scan
payload - full-scan profile spectra, wired into `Reader::open()` through
the same `SpectrumSource` path as every other variant. See "2026-07-21
session: full decode" below for the derivation and evidence; the
original DISCOVERY-phase reconnaissance is kept below it unedited, since
its byte-shape observations (and its unresolved questions) turned out to
be correct and are still the more approachable read for what each stream
looks like before the decode.

## Summary

`crates/openszraw::raw::detect_variant` currently recognizes three
`.lcd` root storages: `TTFL Raw Data` (IT-TOF), `QTFL RawData` (QTOF),
and (unsupported but at least named, see `docs/format/06` section 7)
`TLM Raw Data` (QQQ). `MTBLS1960` (Shimadzu LCMS-2020, a single
quadrupole - the first such instrument in the corpus, added during the
2026-07-21 corpus-widening pass for Sigilweaver/OpenSZRaw#20) uses none
of these. Its real per-spectrum data lives under a fourth root storage
name never previously seen or documented: **`Mass Raw Data`**.

This is not a misdetection like the QQQ case - `detect_variant` finds
none of its three known storages and correctly returns an honest parse
error (`neither 'TTFL Raw Data' nor 'QTFL RawData' storage found in
.lcd file`) rather than silently guessing. But the error doesn't name
what the file actually *is*, because nothing in this crate has ever
looked for it. Single-quadrupole `.lcd` files are simply invisible to
this reader today, not merely unsupported.

## Storage tree (confirmed via `olefile`, both `MTBLS1960/E1.lcd` and `P1.lcd`)

```
Mass Raw Data/Error Log        0 bytes (empty)
Mass Raw Data/MS Raw Data      8,859,753 bytes (E1.lcd) / 8,834,290 bytes (P1.lcd)
Mass Raw Data/MS Status Data   0 bytes (empty)
Mass Raw Data/Operation Log    0 bytes (empty)
Mass Raw Data/Retention Time   9,600 bytes
Mass Raw Data/Scan Group Index 8 bytes
Mass Raw Data/Spectrum Index   9,600 bytes
Mass Raw Data/Status           67,392 bytes
Mass Raw Data/TIC Data         19,200 bytes
```

The `Error Log`/`Operation Log`/`MS Status Data` naming pattern echoes
`GCMS Raw Data`'s equivalent empty-in-every-corpus-file streams
(`docs/format/02`) - plausibly the same underlying acquisition-software
lineage, consistent with Shimadzu's single-quad LC-MS and GC-MS product
lines sharing control electronics/firmware heritage. This is a
plausibility note, not a confirmed claim.

## What's already decodable from byte-shape alone

**`Retention Time`** (9,600 bytes): an array of 1,200 `u32` values
(9600/4), monotonically non-decreasing, starting at 0. Values increase
in an alternating step pattern (e.g. `0, 496, 1500, 1996, 3000, 3496,
4500, 4996, ...` - alternating deltas of roughly 496 and 1004) across
the whole file - consistent with two interleaved acquisition types
(e.g. alternating positive/negative polarity, which `CORPUS.md` already
notes as a plausible per-study description for this dataset from its
own method text, not yet independently confirmed from bytes). Units not
yet confirmed but the value range (up to ~600,000 over 1,200 points) is
consistent with milliseconds over a several-minute LC run.

**`Spectrum Index`** (9,600 bytes, same length as `Retention Time`): a
parallel array of 1,200 `u32` values, monotonically increasing, starting
at 0 (e.g. `0, 3352, 6508, 9736, 12884, 16172, 19328, ...`). Step sizes
(~3,200-6,500) are consistent with per-scan byte offsets into `MS Raw
Data` - the same structural role as `.qgd`'s `Spectrum Index` relative
to `GCMS Raw Data/MS Raw Data` (`docs/format/02`). `MS Raw Data`'s total
size (8,859,753 bytes) divided by 1,200 scans averages ~7,383
bytes/scan, in the right ballpark for the observed offset deltas.

**`TIC Data`** (19,200 bytes = exactly 2x `Retention Time`'s length): an
array of 2,400 `u32` values (RT-point count x 2), also showing the same
alternating-magnitude pattern as `Retention Time`'s deltas (values
clustering around ~350,000 and ~424,000 alternately in the one sample
checked) - another data point consistent with two interleaved scan
types, though the "2 entries per RT point" structure itself is not yet
explained (could be TIC-per-polarity, a (value, reserved) pair, or
something else).

**`Scan Group Index`** (8 bytes): far too small to be per-scan; likely a
single small header/descriptor rather than an array. Not examined.

**`Status`** (67,392 bytes) and `MS Raw Data` itself: not examined
beyond confirming `MS Raw Data` is real, substantial, non-zero binary
content (first 64 bytes: `00000000 00000000 0000c509 00009600
40069411 4e62b203 00000000 00000000 02000000 00000000 00000000 4100...`
- not obviously ASCII, not obviously a recognized envelope from any
other decoded stream in this project).

## Why this matters

Every other multi-file investigation in this project (TTFL RLE, QTOF
centroid, GC-MS scan variants) started from exactly this kind of
byte-shape reconnaissance and reached a full decode. The parallel
`Retention Time`/`Spectrum Index` array structure and plausible
byte-offset relationship to `MS Raw Data` make this look like a
promising, contained target - probably closer in spirit to `.qgd`'s
already-solved format (shared naming: `MS Raw Data`, `Spectrum Index`,
`Retention Time`, `TIC Data` - `.qgd` has all four under `GCMS Raw
Data`) than to TTFL's RLE or QTFL's centroid schemes, though this is a
hypothesis, not yet tested against real per-scan header bytes.

**Corpus caveat**: only one accession (`MTBLS1960`, 8 files) currently
represents this variant, all from the same study. Per this project's
own established lesson (see `docs/format/03`'s TTFL correction on a
second accession, `docs/format/05`'s QTOF correction on a second
source), single-accession claims should be treated as provisional until
corroborated - a second single-quadrupole source would meaningfully
de-risk whatever gets decoded here. `LCMS-2020` is a widely-used, older
Shimadzu model, so a second source is plausibly easy to find via the
same MetaboLights lead-finding approach documented in `CORPUS.md`'s
2026-07-21 pass.

## Constraints

Clean-room only, per `CONTRIBUTING.md`: no LabSolutions software or
vendor SDK/output as ground truth - only independent byte-level
verification against the corpus, same as every other format in this
project.

## 2026-07-21 session: full decode

**Correction to the section above**: `Retention Time`/`Spectrum Index`
are 2,400-element `u32` arrays, not 1,200 - the DISCOVERY-phase note
divided 9,600 by 8 instead of 4. Every ratio/plausibility conclusion
above (alternating step pattern, byte-offset role, `TIC Data` being 2x
that length) is unaffected; only the absolute scan count was wrong.
`TIC Data` is a plain 2,400-element **`u64`** array, not "2,400 `u32`
values, 2 per RT point" - every odd-indexed `u32` half is 0 in the local
corpus (values comfortably fit in 32 bits), which is what produced the
appearance of paired entries. This is now directly confirmed (see
below), not just a cleaner reinterpretation.

### Header and payload layout

`MS Raw Data`'s per-scan record is a **fixed 64-byte header** followed by
a variable-length peak payload, structurally the same shape as `.qgd`'s
32-byte header (`docs/format/02`) but roomier:

| Offset | Type | Field |
|--------|------|-------|
| 0x00 | u32 | Scan index (0-based, matches sequential position) |
| 0x04 | u32 | Retention time, ms (matches the `Retention Time` stream exactly) |
| 0x10 | u32 | Alternating flag, exactly 2 distinct values, strict scan-parity regularity (1,200/1,200 over 2,400 scans in every corpus file) - plausibly a polarity-switching flag (see "What remains open" below) |
| 0x36 | u16 | Peak count (`n_peaks`) |

(Offsets not listed are either constant, near-constant with small
unexplained jitter, or zero padding across the local corpus; none were
needed for the decode below, so they were not investigated further.)

The payload following the 64-byte header is `n_peaks` fixed-width
records, where **the record width itself is derived, not assumed**:
`payload_size / n_peaks` gives an exact per-peak byte count for every
scan in the corpus (0 non-integer results across 19,200 scans). Each
record is `[mz: u16, raw = mz * 10][intensity: little-endian unsigned
int, record_width - 2 bytes]` - mirroring `raw::qgd`'s `u16 mz * 10`
scaling and `raw::qtfl`'s discovery that intensity byte width is a
per-scan, not global, property (`docs/format/06` section 4). Two record
widths are observed in the local corpus: 4 bytes (2-byte intensity,
1,625 of 2,400 scans in `E1.lcd`) and 5 bytes (3-byte intensity, 775 of
2,400) - the split is not a clean 50/50 by scan parity or by the 0x10
flag, and no header field was found that predicts it directly; it was
recovered purely from the per-scan arithmetic above, the same way
`docs/format/06` section 4 found QTOF's intensity width from
`data_bytes`/count rather than a dedicated flag. `MS Raw Data`'s per-scan
byte range comes from consecutive `Spectrum Index` offsets, the same
`.qgd`-style Variant A pattern (absolute offsets, no separate header
stream).

### Verification

**Byte-exact, zero-exception, all 8 corpus files (19,200 scans
total):** decoding every scan's peaks with the layout above and summing
the resulting intensities reproduces the file's own `Mass Raw Data/TIC
Data` value for that scan exactly - 0 mismatches. This is the same class
of proof `docs/format/06` section 4 used for QTOF's intensity width
(cross-checking against a separately stored aggregate field the file
already carries), and it is about as strong as clean-room verification
gets: `TIC Data` is Shimadzu's own precomputed per-scan total, not
something this decode could produce by construction from `MS Raw Data`
alone.

**Physical plausibility**: decoded m/z falls in a `[400, 2000]` Da range
in every scan across all 8 files, well inside the m/z range small-
molecule/metabolite LC-MS methods typically use (this corpus's study is
soybean/insect metabolomics, `CORPUS.md`), and the intensity values are
non-negative integer counts of a magnitude consistent with a
single-quadrupole detector. `Retention Time` spans up to ~30 minutes per
file, a plausible LC run duration. Peak m/z within a scan is
overwhelmingly (though not perfectly - see below) increasing, consistent
with a full mass-range profile sweep.

**What remains open**: the 0x10 alternating flag is a real, decoded
field (not fabricated) with a suspiciously clean 1,200/1,200 parity
split, and matches this dataset's own per-study method description of
alternating polarity acquisition (`CORPUS.md`) - but there is no way to
independently confirm from the bytes alone which of its two values is
positive vs. negative polarity, so (mirroring the exact same gap for
TTFL's channel-mode flag, `docs/format/06` section 2) `SpectrumRecord::polarity`
is left `None` rather than guessed. One scan in `E1.lcd` (index 1, of
2,400) had its first two decoded m/z values out of the otherwise-
monotonic order (402.6 immediately preceded by 403.8) - a minor,
localized non-monotonicity that does not affect the TIC cross-check
(which is order-independent) and was not investigated further, since it
does not block a correct per-peak decode.

**Corpus caveat, still open**: only `MTBLS1960` (8 files, one study)
represents this variant. A same-session search for a second
single-quadrupole LC-MS source (EBI's MetaboLights-scoped search API,
per `CORPUS.md`'s documented 2026-07-21 lead-finding approach, plus a
cross-check against OmicsDI) turned up no second `.lcd` single-quad LC-MS
lead - every other "single quadrupole Shimadzu" hit in MetaboLights
either names a GC-MS instrument (`.qgd`, out of scope here, e.g.
MTBLS12316's GC-2010 Plus/QP2010 Plus single quad) or turned out to be
an already-known QTOF/QQQ/IT-TOF accession. Unlike the QTOF and TTFL
corrections this project has made on a second source in the past, this
decode is corroborated only by its own internal `TIC Data` cross-check,
not by a second independent file.

### Implementation

`Variant::SingleQuad` (`crates/openszraw::raw::mod.rs`) is detected from
the `Mass Raw Data` root storage; `crates/openszraw::raw::mass_raw`
implements the header/payload decode above, wired into `Reader::open()`
and `SpectrumSource::iter_spectra` (`crates/openszraw::reader.rs`)
alongside the other three variants. `RunMetadata::instrument` falls back
to the generic `MS:1000124` "Shimadzu instrument model" CV term (no
dedicated PSI-MS term for a specific single-quad Shimadzu LC-MS model
was found in `psi-ms.obo`, and the storage name alone does not confirm
an exact instrument model).

**A near-repeat of the `docs/format/06` section 7 QQQ/QTFL trap, caught
before merge**: a first version of `detect_variant`'s `SingleQuad` arm
checked only for the bare `Mass Raw Data` root storage's presence, the
same way the pre-existing (and still unfixed, see section 7) QTOF check
does for `QTFL RawData`. A full local-corpus scan
(`cargo run --release --example corpus_scan`) caught this immediately:
QQQ files (confirmed on both `MTBLS2376` and `MTBLS7425`, whose real
data lives under `TLM Raw Data`) also carry an always-present but
completely empty `Mass Raw Data` storage as boilerplate - exactly the
same pattern as their empty `QTFL RawData` storage. Checking for
`Mass Raw Data/MS Raw Data` (a substream only present when this is the
file's real variant) instead of the bare root fixes this for
`SingleQuad` specifically; the pre-existing `QTFL RawData` version of
the same trap is unresolved and tracked separately (section 7, #5).

## Done means (original DISCOVERY-phase criteria, now met)

A documented, evidence-backed decode of `Mass Raw Data/MS Raw Data`'s
per-scan payload (mirroring `docs/format/02`/`03`/`05`'s bar:
byte-exact verification, physical-plausibility checks) - done, see
above; a second corroborating accession was searched for but not
found, so this remains single-source. `Variant::SingleQuad` added to
`crates/openszraw::raw::detect_variant`, and `SpectrumSource`/
`RunMetadata` wiring so single-quadrupole files open and decode through
the same `Reader::open()` entry point as every other variant - done,
see "Implementation" above.
