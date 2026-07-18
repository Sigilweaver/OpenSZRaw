---
sidebar_position: 4
---

# LC-MS IT-TOF (.lcd)

## Status: Partial

Payload run-length encoding: confirmed. Index-to-m/z calibration:
confirmed. Scan metadata prefix: still open.

The `TTFL Raw Data` storage in IT-TOF `.lcd` files holds the primary
mass-spectrometry data. It uses an indexing scheme built around 4
interleaved acquisition channels per retention-time point (e.g. rapidly
alternating ionization polarity or MS level - dataset naming like
`..._pos-neg_NN.lcd` implies this, though the reader does not yet decode
which channel maps to which mode).

## Retention Time and Data Index

`Retention Time` is a plain `u32[N_RT]` array in milliseconds. `Data
Index` is normally `N_RT * 64` bytes - one 64-byte entry per
retention-time point, split into four 16-byte subsets (one per
interleaved channel). Each subset carries an absolute byte offset into
`MS Raw Data`, the retention-time entry index it belongs to, and a
global event counter.

Two corpus-derived corrections to keep in mind when reading this stream:

- A subset's own `entry_i` field must be read directly, not assumed to
  equal its physical block position - files with only 2 real
  interleaved channels per retention-time point pack two RT points'
  subsets into one 64-byte block.
- `Data Index` is not always an exact multiple of 64 bytes; a trailing
  partial block (any positive multiple of 16 bytes) is valid when the
  real-subset count isn't a multiple of 4.

## MS Raw Data scan header (64 bytes)

Every scan starts with a 64-byte header of 16 `u32` fields. The
confirmed fields include: a global scan sequence number, the RT entry
index, retention time in milliseconds (an exact copy of the `Retention
Time` stream value), the channel/subset index, and several file
constants. A field once hypothesized to be a per-scan peak counter was
disproved by direct measurement - it's better explained as a sub-cycle
acquisition timestamp, though that reading is not fully confirmed
either. The true per-scan peak count is only recoverable by fully
decoding the payload.

## Payload: run-length-encoded sparse profile (confirmed)

Every scan payload splits into an undecoded, variable-length metadata
prefix followed by a run-length-encoded tail: repeating
`[marker: u16 = 0x8000 | run_length] [skip: u16] [run_length raw u16 intensity values]`,
terminated by the marker word `0x8000` alone. Reconstructing
`(position, intensity)` pairs is a matter of walking the runs and
accumulating `skip` into a running position.

This decoder was verified byte-exact (zero leftover bytes, no parse
errors) across every scan in every locally available IT-TOF `.lcd` file
from two accessions - 109,336 scans total, 100% clean. The reconstructed
position axis can reach values in the hundreds of thousands for some
scans, so the reader does not clamp or assume any upper bound on it.

## Index-to-m/z calibration (confirmed)

The RLE payload's reconstructed position is a raw digitizer/TOF time-bin
index. The reader converts it to physical m/z using a per-file
calibration parsed from `TTFL Tuning/Tuning Result NN`: this stream
stores a reference calibrant mass ladder (identified as sodium formate
cluster ions, `[Na(HCOONa)n]+` - a standard, publicly documented ESI
calibration solution, identified by its exact spacing matching integer
multiples of the public monoisotopic mass of `HCOONa`) alongside its
measured flight times. Fitting the standard TOF flight-time law
`time = a*sqrt(mz) + b` by least squares gives a residual at the level
of floating-point round-trip noise (about 1 part in a million) across
every IT-TOF file in the local corpus, with only 4 distinct `(a, b)`
pairs across 81 files - consistent with a handful of real per-instrument
tuning sessions, not coincidence.

Since vendor software cannot be used to check this (per this project's
clean-room policy), the evidence that this also calibrates the RLE
payload's own index axis is independent and internal: applying it to
real scan data recovers plausible small-molecule m/z for the bulk of
real signal, and - most convincingly - the theoretical sodium formate
cluster masses recur at a tightly clustered predicted index position
(within a few index units, well under 0.1 Da) across dozens of
independent scans spanning an entire run, concentrated in the channel
pair independently expected to be positive polarity. See
[`docs/format/03-lcd-ttfl-msdata.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/03-lcd-ttfl-msdata.md)
section 3c for the full derivation.

Applying the calibration to the rare, very large noise-tail index values
(into the hundreds of thousands) yields implausibly large m/z - this
reflects those positions being electronic noise, not a flaw in the
calibration; the reader does not filter or clamp them.

## Not yet resolved

- **Scan metadata prefix**: the bytes before the RLE tail (~194 bytes in
  most scans, more when extra MS/MS precursor metadata is present) are
  not decoded.
- **Per-channel polarity/MS-level**: not resolved - see
  [Known limitations](./known-limitations).

See
[`docs/format/03-lcd-ttfl-msdata.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/03-lcd-ttfl-msdata.md)
in the repository for the full byte-level record and verification
methodology.
