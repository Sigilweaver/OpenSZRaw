---
sidebar_position: 2
---

# Scan data

`Reader` decodes each spectrum differently depending on which of the
three on-disk variants the file is. This page is the reader-facing
summary; see [Format specification](../format/overview) for the
byte-level layout each summary below refers to.

## `.qgd` GC-MS

Every scan in `GCMS Raw Data/MS Raw Data` starts with a 32-byte header,
located via the `Spectrum Index` stream. Two acquisition modes exist:

- **Full-scan profile mode**: the payload is `N_Peaks` interleaved
  `(m/z, intensity)` records: a `u16` m/z scaled by 10, then a 2-, 3-,
  or 4-byte intensity whose width the header's `Format` code selects.
  The reader yields one MS1 `SpectrumRecord` per scan.
- **MRM/targeted mode**: the payload is a sequence of monitored
  transitions, each `(precursor m/z, product m/z, intensity)` with a
  variable intensity byte width read directly from the scan header. The
  reader yields one MS2 `SpectrumRecord` per transition (a single
  `(product_mz, intensity)` point, with the precursor's m/z carried in
  `PrecursorInfo`) rather than trying to force multiple simultaneously
  monitored transitions into a single spectrum.

See [GC-MS (.qgd)](../format/qgd-gcms) for the full byte layout.

## `.lcd` IT-TOF

Every scan in `TTFL Raw Data/MS Raw Data` is a variable-length metadata
prefix followed by a run-length-encoded sparse array of 16-bit intensity
samples over an implicit index axis. The reader reconstructs
`(position, intensity)` pairs from the RLE stream and converts
`position` (a raw digitizer time-bin index) to `mz` with a per-file
calibration from the file's own tuning data. These m/z values are not
validated, and spectra are not always sorted by m/z (see
[Known limitations](../format/known-limitations)).
Four interleaved acquisition channels (`sub_i` 0-3) are tracked per
retention-time point via the `Data Index` stream, but the reader does
not currently map any channel to a specific polarity or MS level.

See [LC-MS IT-TOF (.lcd)](../format/lcd-ittof) for the full byte layout.

## `.lcd` QTOF

Every scan in `QTFL RawData/Centroid Data` is a 64-byte header followed
by an `N`-element `u64` m/z array (scaled by 10^12) and an `N`-element
intensity array whose byte width (1, 2, or 4 bytes) is read from the
scan header - it is not always 16-bit. `Centroid Index`'s per-scan
`event_id` field distinguishes MS1 (`event_id == 1`) from MS2
(`event_id > 1`) scans within each acquisition cycle; MS2 records carry
a reference to their parent MS1 scan but not a decoded precursor m/z.
The reported m/z values are wrong: the axis is distorted, so isotope
peaks appear about 1100/m Da apart instead of 1.003 Da.

See [LC-MS QTOF (.lcd)](../format/lcd-qtof) for the full byte layout.

## Retention time

`retention_time_sec` is derived from each format's own `Retention Time`
stream (milliseconds for `.lcd`, matched against the scan header's own
copy of the value for IT-TOF).

## Fields not yet populated

Polarity is `None` for every spectrum in every variant - see
[Known limitations](../format/known-limitations) for the full list of
what each format still leaves open.
