# 04. LC-MS (.lcd) Chromatogram and PDA Structures

**Status**: PARTIAL

The `.lcd` files use a different encoding scheme for chromatograms (`LSS Raw Data`) and Photo-Diode Array (PDA) UV data (`PDA 3D Raw Data`) than the primary MS raw data streams.

## Segment Header

Chromatogram and PDA streams are divided into chunks or **segments**. Every segment begins with a **24-byte header**.

**Header Layout:**
Assuming a 6-element array of 32-bit (u32 LE) integers:
- **u32[0]**: Magic Number (`17234` which corresponds to the ASCII string `RC\x00\x00`).
- **u32[1]**: Unknown / Version (e.g., 1).
- **u32[2]**: Likely the number of data points or a related parameter (e.g., 321).
- **u32[3]**: Segment Block Size in bytes, inclusive of this 24-byte header (e.g., 353, 562).
- **u32[4]**: Zero padding.
- **u32[5]**: Zero padding.

By reading `u32[3]`, a parser can successfully jump to the next segment header (the next `RC\x00\x00` signature).

## Delta Encoding Payload

Following the 24-byte header, the segment payload contains the actual intensity or absorbance data.
- Because the data payload size is frequently an odd number of bytes (e.g., a 353-byte segment has 329 data bytes), it cannot be a simple array of 32-bit or 16-bit integers.
- This strongly indicates a **bit-packed or variable-length delta encoding** scheme.
- *Failed Hypotheses*: 
  - Standard unsigned LEB128 (7-bit continuation) was tested against a segment but yielded 426 decoded values rather than the 321 points explicitly declared in the segment header (`u32[2]`).
  - Interpreting the raw bits revealed frequent patterns of `0x3F`, suggesting possible 32-bit floats. Interpreting the bytes as **PDP-endian floats** (Middle Endian `3412`, e.g., swapping 16-bit words) actually yields perfectly valid float values (e.g., `0.914`, `1.782`). However, the mathematically odd byte sizes of the segments mean the payload cannot exclusively be an array of PDP-endian floats.
- The exact bit-masking or delta-compression logic remains an open reverse-engineering problem.

## Cross-check against the TTFL MS Raw Data RLE scheme (negative result)

`TTFL Raw Data/MS Raw Data` (the IT-TOF MS payload, see `docs/format/03`)
turned out this session to use a run-length scheme: a `u16` marker word
`0x8000 | run_length` (terminator `0x8000` when `run_length==0`),
followed by a `u16` skip word and `run_length` raw `u16` values,
confirmed byte-exact (zero leftover) across 109,336 real MS scans.

Tried applying the *exact same* decoder directly to PDA segment payloads
(`re/src/analysis/pda_rle_test.py`, tested against
`MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`'s `PDA 3D Raw
Data/3D Raw Data` stream, segments after the 24-byte `RC\x00\x00`
header): **it does not decode cleanly**. The first several segments in
that stream are ~72-byte payloads that are almost entirely zero bytes
(just a leading `u16` matching `u32[2]`'s point count, e.g. `44 00` =
68, then zero padding - plausibly a genuinely blank/baseline region
early in the run), and the later, clearly-populated segments (dense,
high-entropy byte patterns, no long zero runs at all) don't parse as
TTFL-style marker words at all (`ok=False`, full leftover every time).
So the PDA payload is evidently a **different** encoding from the MS RLE
scheme, not a shared cross-stream-type compression as speculated -
whatever it is, it looks denser/less sparse than the MS profile data,
consistent with the earlier PDP-endian-float observation (UV absorbance
across many wavelengths per timepoint is unlikely to have the long
sparse zero-runs that MS profile spectra have, so a sparse RLE scheme
would be a poor fit for PDA data anyway, in hindsight). Not pursued
further this session; still open.
