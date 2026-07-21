# 07. Single-Quadrupole (`.lcd` `Mass Raw Data`): a fifth, undocumented on-disk variant

**Status**: DISCOVERY

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

## Done means

A documented, evidence-backed decode of `Mass Raw Data/MS Raw Data`'s
per-scan payload (mirroring `docs/format/02`/`03`/`05`'s bar:
byte-exact verification, physical-plausibility checks, ideally a second
corroborating accession), `Variant::SingleQuad` (or similarly named)
added to `crates/openszraw::raw::detect_variant`, and
`SpectrumSource`/`RunMetadata` wiring so single-quadrupole files open
and decode through the same `Reader::open()` entry point as every other
variant - or, short of a full decode, at minimum a corrected
`detect_variant` error message that names the file as a recognized-but-
unsupported single-quad variant rather than the current generic
"neither TTFL nor QTFL" message, so a user gets an honest diagnosis
even before the payload itself is solved.
