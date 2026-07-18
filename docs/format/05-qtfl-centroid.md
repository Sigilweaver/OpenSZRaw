# 05. LC-MS QTOF (.lcd) QTFL Centroid Data

**Status**: CONFIRMED

The `QTFL RawData` directory in 9030 QTOF `.lcd` files (e.g., MSV000084197) contains centroided mass spectrometry data. Unlike IT-TOF, the QTOF format uses a clean, uncompressed block layout for its centroid data.

## 1. Centroid Index

The `Centroid Index` stream maps sequential scan events to their data payload in the `Centroid Data` stream.
- **Record Size**: 24 bytes per record.
- **Layout (6x 32-bit LE integers)**:
  - `u32[0]`: Absolute byte offset into the `Centroid Data` stream where the scan header begins.
  - `u32[1]`: Zero padding.
  - `u32[2]`: Subset/Interleave Index (e.g., 0, 1, 2, 3... repeating, likely for alternating acquisition modes like pos/neg or MS1/MS2).
  - `u32[3]`: Global Event/Scan Counter.
  - `u32[4]`: Local Scan Index within the subset.
  - `u32[5]`: Segment/Event ID (e.g., 1, 2...).

The byte length of a scan in `Centroid Data` can be calculated by subtracting its offset (`u32[0]`) from the offset of the next chronological scan in the index.

## 2. Centroid Data Payload

The `Centroid Data` stream contains the scan records.

### Scan Header
Every scan begins with a **64-byte header**. The most critical fields (as 32-bit LE integers) are:
- **`u32[4]`**: Base Peak Intensity (BPI) for the scan.
- **`u32[6]`**: Data Payload Size (`S`) in bytes.

The number of peaks (`N`) in the centroid scan is exactly `S / 10`.

### Data Payload Layout
Immediately following the 64-byte header is the data payload, which is exactly `S` bytes long. It uses a strictly uncompressed block format:
1. **m/z Array**: `N` consecutive 64-bit LE unsigned integers (`u64`).
   - The m/z values are scaled by $10^{12}$ (one trillion).
   - *Decoding*: `mz_actual = u64_value / 1,000,000,000,000.0`
2. **Intensity Array**: `N` consecutive 16-bit LE unsigned integers (`u16`).

*Example validation:* A scan with payload size `S = 160` has exactly 16 peaks. The payload consists of 16x `u64` values (128 bytes) followed by 16x `u16` values (32 bytes), perfectly accounting for the 160 bytes with zero slack.

## Addendum (Phase 4 implementation session): intensity width is variable, and `event_id` encodes MS1/MS2

Two corrections/additions found while implementing the Rust reader,
verified against `MSV000084197/20190607_NM16.lcd` - see
`docs/format/06-known-limitations.md` sections 3 and 4 for full detail:

1. **Intensity is not always 16-bit.** The scan header's `u32[9]` field
   (byte offset `0x24`) gives the intensity byte width per scan (1, 2,
   or 4 bytes observed); `N = S / (8 + width)`, not always `S / 10`.
   Assuming a fixed 16-bit width produces a confirmed corrupt decode on
   higher-dynamic-range (mostly MS2) scans.
2. **`Centroid Index`'s `u32[5]` ("Segment/Event ID") is a real
   per-cycle MS1/MS2 counter**, not an opaque tag: `event_id == 1` is
   the MS1 survey scan, `event_id > 1` are MS2 product-ion scans (1-4
   per cycle observed), one per DDA-selected precursor. The real
   precursor m/z lives in the separate, undecoded `QTFL RawData/DDA`
   stream.

The "Status: CONFIRMED" line above refers specifically to the payload
byte-layout claim it was originally verified against (which remains
correct for the common 16-bit-intensity case); it did not cover scan
classification or the variable intensity width, both undiscovered at
the time.

## Addendum (corpus expansion session, 2026-07-18): corroborated on a second, independent QTOF source

Every claim above and in `docs/format/06-known-limitations.md` sections 3
and 4 was, until this session, verified against exactly one file
(`MSV000084197/20190607_NM16.lcd`). A second independent QTOF source
(`MTBLS14820`, 10 files, a different LCMS-9030 instrument at a different
institution, negative-ion DDA metabolomics on wheat leaf extracts) is now
in the corpus, and a standalone check script
(`re/src/analysis/qtfl_corroborate.py`, gitignored/local-only, not part of
the Rust reader) re-derives the same two claims independently:

- **Intensity byte width + BPI consistency** (`docs/format/06` #4):
  across all 9 fetched `MTBLS14820` files (~66,300 scans checked total),
  `max(intensity)` matched the header's declared base-peak intensity
  (`u32[4]`) with the width read from `u32[9]` in **every single case, 0
  mismatches** - the same result as the original single-file check.
  Width distribution was similar in shape (mostly 2-byte, with real 1-
  and 4-byte scans present in every file) though the exact proportions
  differ per file, as expected for different samples/methods.
- **`event_id` MS1/MS2 cycle structure** (`docs/format/06` #3): held on
  every file - `event_id == 1` always starts a cycle (MS1), and
  subsequent events strictly increase within a cycle. The specific range
  differs from the original file: `MTBLS14820`'s cycles are always
  exactly 1 MS1 + 0-1 MS2 (`event_id` never exceeds 2), versus
  `MSV000084197`'s 1 MS1 + up to 3 MS2 events. This is a **narrower**
  range, not a contradiction - consistent with a DDA method configured
  for fewer precursors per cycle (e.g. "top-1" vs "top-4"), which is a
  per-method acquisition parameter, not a fixed format property.

One new observation, not previously documented: about 1% of `Centroid
Index` records per file decode a scan-header intensity-width field that
is not 1, 2, or 4 (garbage values like 260 were observed). These are the
same records `crates/openszraw::reader::qtfl_spectra` already silently
skips via its `Err(_) => continue` on `qtfl::decode_scan`'s width
validation - i.e. this is pre-existing tolerated behavior, not a new
break, and was very likely present at a similar rate in the original file
too (its own check only ever reported success/failure over the scans
that *could* be parsed, not over 100% of index records). Worth a closer
look at what these records actually are (a genuine format oddity around
interleaved-subset boundaries, going by `docs/format/05`'s original
"Subset/Interleave Index" language) in a future session, but out of
scope here.

**Net result: both corrections in `docs/format/06-known-limitations.md`
are corroborated, not contradicted, by this second source.**
