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
