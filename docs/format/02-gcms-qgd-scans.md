# 02. GC-MS (.qgd) Scan Structures

**Status**: PARTIAL

The `GCMS Raw Data` directory within `.qgd` files stores mass spectrometry data. We have identified two distinct data encoding formats, typically correlating with different instrument generations or acquisition modes (e.g., Profile/Centroid vs. MRM/SIM).

## 1. Spectrum Index Stream

The `Spectrum Index` stream maps sequential scan numbers to byte offsets within the `MS Raw Data` stream. There are two variants of this index:

**Variant A (e.g., PXD019638, ~42MB files):**
- No header.
- Array of 32-bit (u32 LE) unsigned integers.
- Each integer is an absolute byte offset into `MS Raw Data`.
- The number of scans is `stream_size / 4`.

**Variant B (e.g., PXD034978, ~12.5MB files):**
- A 2-byte header (u16 LE, typically value = 1).
- Array of 64-bit (u64 LE) unsigned integers.
- Each integer is an absolute byte offset into `MS Raw Data`.
- The number of scans is `(stream_size - 2) / 8`.

## 2. MS Raw Data Scan Header

Every scan in the `MS Raw Data` stream begins with a **32-byte header**. The fields are primarily 32-bit (u32 LE) and 16-bit (u16 LE) integers.

| Offset | Type | Description |
|--------|------|-------------|
| 0x00   | u32  | Scan Index / Flags (0-based in Variant A, matches sequential index in Variant B) |
| 0x04   | u32  | Retention Time (matches the value in the `Retention Time` stream) |
| 0x08   | u32  | Constant (e.g., 0x01D60000 = 30801920) |
| 0x0C   | u32  | Scan Number (0-based in Variant A, 1-based in Variant B) |
| 0x10   | u32  | Zero padding |
| 0x14   | u16  | Format / Event ID (See below) |
| 0x16   | u16  | N_Peaks / Event ID (See below) |
| 0x18   | u16  | Format / Event ID (See below) |
| 0x1A   | u16  | N_Peaks / Event ID (See below) |
| 0x1C   | u32  | Zero padding |

The placement of the Format and Peak Count / Event ID fields differs between the two data variants.

## 3. Data Encoding Variants

### Profile / Centroid Mode (Variant A)

Detected in files like PXD019638 (u32 index). This mode encodes data as interleaved (m/z, intensity) pairs.
- **Header placement**: The `Format` (value 2) is at offset 0x14 (20). The `N_Peaks` is at offset 0x16 (22). Offsets 0x18-0x1B are zero.
- **Data layout**: Following the 32-byte header, there are exactly `N_Peaks` pairs of 16-bit (u16 LE) values.
- **m/z Decoding**: The first u16 is the raw m/z. In our reference file, it is encoded as `m/z * 10` (e.g., raw 1000 = 100.0 Da).
- **Intensity Decoding**: The second u16 is the raw intensity.
- **Size validation**: The scan size exactly equals `32 + N_Peaks * 4` bytes.

### MRM / Targeted Mode (Variant B)

Detected in files like PXD034978 (u64 index). This mode represents targeted acquisition (Multiple Reaction Monitoring, MRM) with a fixed number of monitored transitions per event. Scans of different events (segments) can be interleaved, causing the scan sizes to vary accordingly.
- **Header placement**: The **Event ID** is found at offset 0x18 (24). In our reference file, these take values like 101, 102, 103.
- **Data layout**: Following the 32-byte header, the data is stored as:
    * *Resolution*: A GC-MS MRM data payload consists of multiple MRM transitions. Each transition is encoded as:
    `[Precursor m/z (u16, *10)]` `[Product m/z (u16, *10)]` `[Intensity (variable LE int)]`
  * The intensity bit-width is completely dependent on the **Event ID** (the sixth `u16` in the header).
  * For example, in `PXD034978`:
    * **Event 101**: 32-bit intensity (4 bytes). Total transition size = 8 bytes.
    * **Event 103**: 24-bit intensity (3 bytes). Total transition size = 7 bytes.
    * **Event 102**: 16-bit intensity (2 bytes). Total transition size = 6 bytes.
  * **Event ID Mapping**: The specific bit-width mapping for a given Event ID is likely defined in the `QP5K Instrument Parameters/MS Parameter` stream (which contains the raw transition `m/z` values, e.g., `320.2` and `118.0` found encoded as `u16`s). However, due to the massive size and complexity of the parameter stream, the most practical parse-time strategy is to dynamically infer the width.
    * *Fallback Approach*: By observing the byte size of the payload (e.g., `Data Bytes`) and dividing by the number of transitions known for that event (or by testing divisibility by 6, 7, and 8), a parser can securely auto-detect the intensity width per Event ID on the fly.
- **m/z Decoding**: Both precursor and product m/z values are encoded as `u16` (`m/z * 10`). For example, `3202` corresponds to `320.2 Da`.
- **Observation**: Certain high-precision intensity values (like those in Event 101) can appear constant (e.g., `0x807FB314`), potentially indicating detector saturation, an error code, or an inactive channel for that specific transition.

## Addendum (Phase 4 implementation session): the header already gives the transition count directly

The "Fallback Approach" above (dividing by 6/7/8 and hoping for a unique
fit) turned out to be unnecessary: offset `0x1A` (the table's
"N_Peaks / Event ID" slot) is literally `n_transitions` for Variant B,
not just an alternate Event ID reading. Verified against
`PXD034978/49_27a__8122021_11.qgd`: for events 101/102/103, `0x1A` reads
back `2` in every scan, and `data_bytes / 2 - 4` reproduces the doc's
own per-event widths exactly (event 101 -> 4 bytes, 102 -> 2 bytes,
103 -> 3 bytes). This is used directly in
`crates/openszraw::raw::qgd::parse_scan` instead of the blind-divisibility
search, which is both simpler and free of the ambiguity the divisibility
approach could hit (e.g. a `data_bytes` divisible by more than one of
6/7/8).

Also worth recording: within one nominal Event ID, the *actual*
monitored (precursor, product) pairs can differ from scan to scan (seen
alternating between two pairs sharing a common product ion across
consecutive scans in the reference file) - this is real MRM
time-multiplexing across a segment with more logical transitions than
fit in one scan's dwell cycle, not a decode error. Each scan's
transitions are decoded independently per the layout above; no
assumption is made that a given transition slot maps to the same
(precursor, product) across scans.
