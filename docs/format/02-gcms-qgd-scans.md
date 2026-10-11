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

Seen in PXD019638 (u32 index, every scan) and MTBLS11411 (u64 index, where full-scan scans are mixed with MRM scans in the same file). The scan layout is chosen per scan from the header, not from the index width. This mode encodes data as interleaved (m/z, intensity) records.
- **Header placement**: The `Format` code is at offset 0x14 (20). The `N_Peaks` is at offset 0x16 (22). Offsets 0x18-0x1B are zero.
- **Format code**: `Format` is a per-scan code for the intensity byte width. A single file mixes codes from scan to scan (higher widths appear on the more intense scans):

  | `Format` | Intensity width | Record size | Seen in |
  |---|---|---|---|
  | `2` | 2 bytes | 4 | PXD019638 |
  | `3` | 3 bytes | 5 | PXD019638, MTBLS11411 |
  | `1` | 4 bytes, bit 31 is a flag | 6 | MTBLS11411 |

- **Data layout**: Following the 32-byte header, there are exactly `N_Peaks` records: a u16 LE raw m/z followed by an unsigned LE intensity of the width above.
- **m/z Decoding**: The raw m/z is encoded as `m/z * 10` (e.g., raw 1000 = 100.0 Da).
- **Intensity Decoding**: The intensity is an unsigned little-endian integer. For `Format` `1`, bit 31 is cleared before use. The flagged values sit on the most intense ions, consistent with a saturation marker (compare the `0x807FB314`-style MRM values below), but the meaning is not confirmed, so the reader drops the bit.
- **Size validation**: The scan size exactly equals `32 + N_Peaks * record_size` bytes.
- **Evidence**: summing decoded intensities reproduces the file's own `GCMS Raw Data/TIC Data` stream (one `u64` per scan) for all 391,625 profile scans in the local corpus (47 `.qgd` files). For code `1` this holds only once bit 31 is cleared.
- **Unknown codes**: a scan with any other `Format` value is reported as an `Err` item by `iter_spectra` and counted (see `Reader::skipped_scans`).

### MRM / Targeted Mode (Variant B)

Detected in files like PXD034978 (u64 index). This mode represents targeted acquisition (Multiple Reaction Monitoring, MRM) with a fixed number of monitored transitions per event. Scans of different events (segments) can be interleaved, causing the scan sizes to vary accordingly.
- **Header placement**: The **Event ID** is at offset 0x18 (24). In the reference file, these take values like 101, 102, 103. Offset 0x1A is the number of transitions in the scan (`n_transitions`).
- **Data layout**: Following the 32-byte header, the payload is `n_transitions` records, each encoded as:
  `[Precursor m/z (u16, *10)]` `[Product m/z (u16, *10)]` `[Intensity (variable LE int)]`
- **Intensity width**: the width depends on the Event ID and is derived per scan from the payload: `width = data_bytes / n_transitions - 4`. For example, in `PXD034978/49_27a__8122021_11.qgd` (`n_transitions` = 2 in every scan):
  * **Event 101**: 32-bit intensity (4 bytes). Total transition size = 8 bytes.
  * **Event 103**: 24-bit intensity (3 bytes). Total transition size = 7 bytes.
  * **Event 102**: 16-bit intensity (2 bytes). Total transition size = 6 bytes.

  The Event ID to width mapping is probably also defined in the `QP5K Instrument Parameters/MS Parameter` stream (which contains the raw transition `m/z` values, e.g., `320.2` and `118.0` encoded as `u16`s), but the reader does not need it.
- **m/z Decoding**: Both precursor and product m/z values are encoded as `u16` (`m/z * 10`). For example, `3202` corresponds to `320.2 Da`.
- **Observation**: Certain high-precision intensity values (like those in Event 101) can appear constant (e.g., `0x807FB314`), potentially indicating detector saturation, an error code, or an inactive channel for that specific transition.
- **Time multiplexing**: within one nominal Event ID, the monitored (precursor, product) pairs can differ from scan to scan (seen alternating between two pairs sharing a common product ion across consecutive scans in the reference file). This is MRM time-multiplexing across a segment with more logical transitions than fit in one scan's dwell cycle, not a decode error. Each scan's transitions are decoded independently; no assumption is made that a given transition slot maps to the same (precursor, product) across scans.
