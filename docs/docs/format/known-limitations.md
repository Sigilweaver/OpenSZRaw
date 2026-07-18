---
sidebar_position: 6
---

# Known limitations

This page lists the deliberate, documented gaps in the current reader
(`crates/openszraw`) - things investigated and found unrecoverable (or
not yet decoded) from the current understanding of the format, not
things silently guessed or fabricated. See the sibling readers'
equivalent pages (e.g. OpenSXRaw's legacy-TOF calibration gap) for the
precedent this follows.

## IT-TOF (`.lcd` TTFL): m/z calibration

The reconstructed index axis from the run-length-encoded payload (see
[LC-MS IT-TOF](./lcd-ittof)) is a raw digitizer/TOF time-bin index. The
reader converts it to physical m/z using a per-file calibration parsed
from the file's own `TTFL Tuning/Tuning Result NN` stream: a reference
calibrant mass ladder (identified as sodium formate cluster ions, a
standard public ESI calibration solution) paired with measured flight
times, fit to the standard TOF law `time = a*sqrt(mz) + b` by least
squares. This fits to a residual at the level of floating-point
round-trip noise across every IT-TOF file in the local corpus - see the
repository's `docs/format/03-lcd-ttfl-msdata.md` section 3c for the full
derivation and evidence.

If a file has no readable calibration table, the reader falls back to
the raw, uncalibrated index rather than fabricate one - this has not
been observed in the local corpus.

The reconstructed index can reach values well past what might seem like
a reasonable upper bound for a time-bin axis (into the hundreds of
thousands in some real corpus scans); under calibration this maps to
implausibly large m/z, reflecting those positions being electronic
noise rather than real ions. The reader does not clamp, filter, or
validate the index against any assumed range - that is downstream
peak-picking's job.

## IT-TOF (`.lcd` TTFL): per-channel polarity/MS-level is not resolved

Each retention-time entry's `Data Index` carries 4 interleaved channel
subsets. One 64-byte scan-header field is a constant that differs by
channel in a way that looks like a coarse mode/polarity flag, but there
is no confirmed mapping from that field (or from the channel index
itself) to a specific polarity or MS level, despite dataset naming
(`..._pos-neg_NN.lcd`) implying the run alternates ionization polarity
across channels.

The reader leaves `polarity` as `None` and `ms_level` as `1` for every
IT-TOF spectrum, rather than assign a channel-to-polarity/level mapping
it cannot back with decoded evidence.

## QTOF (`.lcd` QTFL): MS2 precursor m/z is not decoded

`Centroid Index`'s segment/event-ID field is a real per-acquisition-
cycle counter (MS1 survey scan at `event_id == 1`, MS2 product-ion scans
at `event_id > 1`), consistent with DDA acquisition selecting a variable
number of precursors per cycle. The real per-scan precursor m/z for
those MS2 events lives in the separate `QTFL RawData/DDA` stream, which
has not been decoded.

The reader classifies `ms_level` from `event_id` since that pattern is
well-supported by corpus evidence, but populates MS2 spectra's
`PrecursorInfo` with only a `precursor_native_id` reference to the most
recent MS1 scan, leaving `target_mz`/`selected_mz` as `None` rather than
fabricate a value.

## GC-MS (`.qgd`): polarity and exact instrument model are not resolved

No polarity bit has been found in the 32-byte `MS Raw Data` scan header
or the `Spectrum Index` stream, so `.qgd` `polarity` is always `None`.
GC-EI-MS is conventionally positive-ion, but that's a domain convention,
not a decoded field, so it's left unpopulated rather than assumed.

No PSI-MS CV term dedicated to the `.qgd` file format itself exists
(unlike `.lcd`, which has `MS:1003009` "Shimadzu Biotech LCD format"),
so `source_file_format` falls back to the generic `MS:1000560` "mass
spectrometer file format" node. `instrument` likewise stays at the
generic `MS:1000124` "Shimadzu instrument model" for all `.qgd` files,
since no per-file instrument-model string has been found decoded
anywhere in the corpus.

## PDA / chromatogram streams: encoding not decoded, out of scope

`.lcd` files' chromatogram (`LSS Raw Data`) and photodiode-array UV
(`PDA 3D Raw Data`) streams use a segment-based encoding, unrelated to
the MS run-length scheme above (confirmed by attempting to decode PDA
segments with the same RLE decoder used for IT-TOF MS data - it does not
decode cleanly). The segment payload appears to be a bit-packed or
variable-length delta encoding; plain LEB128 and a fixed-width float
interpretation were both tried and ruled out. This encoding is not
decoded and is out of scope for the current reader.

See
[`docs/format/06-known-limitations.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/06-known-limitations.md)
and
[`docs/format/04-lcd-chromatogram-pda.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/docs/format/04-lcd-chromatogram-pda.md)
in the repository for the full byte-level record behind each of these.
