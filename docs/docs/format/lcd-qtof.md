---
sidebar_position: 5
---

# LC-MS QTOF (.lcd)

## Status: Confirmed for the payload shape, with corrections

The `QTFL RawData` storage in 9030-series QTOF `.lcd` files holds
centroided mass-spectrometry data. Unlike IT-TOF, QTOF uses a clean,
uncompressed block layout.

## Centroid Index

24 bytes per record, mapping sequential scan events to their payload in
`Centroid Data`: an absolute byte offset, a subset/interleave index, a
global event counter, a local scan index, and a segment/event ID. A
scan's byte length is derived by subtracting its offset from the next
chronological scan's offset.

The segment/event ID field is a real per-acquisition-cycle counter, not
an opaque tag: `event_id == 1` is the MS1 survey scan, and `event_id >
1` are MS2 product-ion scans (1-4 observed per cycle in the corpus),
consistent with DDA acquisition selecting a variable number of
precursors per cycle.

## Centroid Data payload

Every scan begins with a 64-byte header, including the base-peak
intensity and the data payload size in bytes. The payload itself is:

1. An `N`-element array of `u64` (LE) m/z values, scaled by 10^12
   (`mz = u64_value / 1e12`).
2. An `N`-element intensity array.

`N` is `payload_size / (8 + width)`, where `width` (1, 2, or 4 bytes) is
read from the scan header rather than assumed fixed. Treating intensity
as always 16-bit produces a confirmed corrupt decode on
higher-dynamic-range (mostly MS2) scans - verified by cross-checking
`max(intensity)` against the header's declared base-peak intensity
across every non-empty scan in the reference corpus file: zero
mismatches once width is read per-scan, versus spurious values and a
base-peak mismatch when assumed fixed at 2 bytes.

## Not yet resolved

- **MS2 precursor m/z**: the real per-scan precursor m/z for MS2 events
  lives in the separate `QTFL RawData/DDA` stream, which has not been
  decoded. The reader classifies `ms_level` from `event_id` and
  populates MS2 `PrecursorInfo` with only a reference to the parent MS1
  scan, not a decoded m/z - see
  [Known limitations](./known-limitations).

See
[`docs/format/05-qtfl-centroid.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/05-qtfl-centroid.md)
in the repository for the full byte-level record.
