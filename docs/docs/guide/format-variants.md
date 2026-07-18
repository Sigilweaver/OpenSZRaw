---
sidebar_position: 3
---

# Format variants

OpenSZRaw's corpus covers three distinct on-disk raw-data variants,
each nested inside a different top-level storage of the same OLE2/CFBF
container format (see [OLE2 container](../format/ole2-container)).
`Reader::open` distinguishes them by checking which storage is present,
never by file extension.

## `.qgd` GC-MS

Root storage: `GCMS Raw Data/`. Produced by GCMSsolution on Shimadzu
GC-MS instruments (e.g. the QP2010 series). Two acquisition modes are
covered:

- **Full-scan profile mode** - conventional scanning-quadrupole GC-MS
  spectra, one MS1 spectrum per scan.
- **MRM/targeted mode** - scheduled Multiple Reaction Monitoring on a
  triple-quadrupole GC-MS/MS instrument, with per-transition
  `(precursor, product, intensity)` records interleaved across
  acquisition "events" within a run.

No polarity bit has been found in either mode; see
[Known limitations](../format/known-limitations).

## `.lcd` IT-TOF

Root storage: `TTFL Raw Data/`. Produced by LabSolutions on Shimadzu's
IT-TOF (ion trap - time-of-flight) LC-MS instruments. Spectra are
run-length-encoded sparse profile data over 4 interleaved acquisition
channels per retention-time point (commonly alternating polarity or
MS level in the source method, e.g. datasets named `..._pos-neg_NN.lcd`
- though OpenSZRaw does not yet decode which channel is which). The
reconstructed index axis is a raw digitizer time-bin index, converted to
physical m/z via a per-file calibration parsed from the file's own TOF
tuning data - see
[Known limitations](../format/known-limitations).

## `.lcd` QTOF

Root storage: `QTFL RawData/`. Produced by LabSolutions on Shimadzu's
9030-series quadrupole time-of-flight LC-MS instruments. Spectra are
centroided at acquisition time (no profile data), stored in a flat,
uncompressed block layout with calibrated 64-bit-fixed-point m/z values
and a per-scan variable-width intensity array. A per-cycle `event_id`
distinguishes MS1 survey scans from MS2 product-ion scans, but the real
MS2 precursor m/z is not yet decoded (it lives in the separate
`QTFL RawData/DDA` stream).

## Telling them apart from the outside

Both `.lcd` variants share the same file extension, so the extension
alone can't tell you which is which - only the internal CFBF storage
does. `Reader::open` (and Python's `RawReader`) always checks this
directly:

```rust
use openszraw::reader::Reader;

// Works identically whether "sample.lcd" is IT-TOF or QTOF underneath.
let reader = Reader::open("sample.lcd")?;
```

If you need to know which variant you got *without* reading spectra,
inspect `reader.run_metadata().instrument`, which carries a
variant-specific PSI-MS CV term (`MS:1000604` "LCMS-IT-TOF" vs
`MS:1002998` "LCMS-9030").

## PDA / chromatogram data

A fourth storage, `PDA 3D Raw Data/` (photodiode-array UV detector
data), appears in some `.lcd` files alongside the MS storages above.
It uses a different, still-undecoded segment-based delta encoding and
is out of scope for this reader - see
[Known limitations](../format/known-limitations).
