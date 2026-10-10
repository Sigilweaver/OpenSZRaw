---
sidebar_position: 3
---

# GC-MS (.qgd)

## Status: Partial

The `GCMS Raw Data` storage in `.qgd` files holds all mass-spectrometry
data. Two distinct data encodings exist, corresponding to different
acquisition modes.

## Spectrum Index

`GCMS Raw Data/Spectrum Index` maps sequential scan numbers to byte
offsets in `MS Raw Data`. Two on-disk variants have been observed:

- **No header**: an array of `u32` (LE) absolute byte offsets. Scan
  count is `stream_size / 4`.
- **2-byte header** (`u16` LE, typically `1`): an array of `u64` (LE)
  absolute byte offsets. Scan count is `(stream_size - 2) / 8`.

## MS Raw Data scan header

Every scan begins with a 32-byte header of mostly `u32`/`u16` fields:
scan index/flags, retention time (matching the `Retention Time`
stream), a scan number, and a pair of format/event-ID/peak-count fields
whose exact byte placement depends on which acquisition mode the scan
is (see below).

## Full-scan profile mode

The payload following the header is `N_Peaks` interleaved
`(m/z, intensity)` records: a `u16` m/z scaled by 10 (raw `1000` =
`100.0` Da) followed by a little-endian unsigned intensity. The header's
`Format` code selects the intensity width per scan, and one file can mix
codes:

| `Format` | Intensity width |
|---|---|
| `2` | 2 bytes |
| `3` | 3 bytes |
| `1` | 4 bytes; bit 31 is a flag and is cleared |

Scan size validates exactly as `32 + N_Peaks * (2 + width)` bytes.
Summed intensities match the file's own `TIC Data` stream for every
profile scan in the local corpus. A scan with an unknown `Format` code
is skipped and counted in `Reader::skipped_scans`.

## MRM / targeted mode

Represents scheduled Multiple Reaction Monitoring on a triple-quad
GC-MS/MS instrument. The scan header's Event ID field distinguishes
acquisition "events" (segments), which can interleave within a run. The
payload is a sequence of transitions, each encoded as
`[precursor m/z (u16, x10)] [product m/z (u16, x10)] [intensity (variable-width LE int)]`.

The intensity byte width is **not** guessed by dividing the payload
size against candidate widths - it's read directly from the scan
header's `N_Peaks`/Event-ID slot, which for this mode holds the literal
transition count, letting the reader compute
`intensity_width = data_bytes / n_transitions - 4` exactly. Within one
nominal Event ID, the actual monitored `(precursor, product)` pairs can
differ scan-to-scan (real MRM time-multiplexing across a segment with
more logical transitions than fit in a single scan's dwell cycle, not a
decode error).

## Not yet resolved

- **Polarity**: no polarity bit has been found in either the 32-byte
  scan header or the `Spectrum Index` stream. GC-EI-MS is conventionally
  positive-ion, but that's a domain convention, not a decoded field, so
  it's left unpopulated - see
  [Known limitations](./known-limitations).
- **Instrument model**: no per-file instrument-model string has been
  found decoded anywhere in the corpus.

See
[`docs/format/02-gcms-qgd-scans.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/02-gcms-qgd-scans.md)
in the repository for the full byte-level record, including the
per-event intensity-width table derived from the reference corpus file.
