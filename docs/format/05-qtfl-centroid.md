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
