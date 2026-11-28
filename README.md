# OpenSZRaw

[![CI](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.24586025.svg)](https://doi.org/10.5281/zenodo.24586025)
[![crates.io](https://img.shields.io/crates/v/openszraw.svg)](https://crates.io/crates/openszraw)
[![PyPI](https://img.shields.io/pypi/v/openszraw.svg)](https://pypi.org/project/openszraw/)
[![docs.rs](https://img.shields.io/docsrs/openszraw)](https://docs.rs/openszraw)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/Sigilweaver/OpenSZRaw/blob/main/LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

> Part of the [OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec)
> stack for mass spectrometry raw-file access.

Rust and Python reader for Shimadzu LabSolutions mass spectrometry raw
data (`.lcd` LC-MS, `.qgd` GCMS, `.gcd` GC), with no Shimadzu SDK or
software dependency. Covers `.qgd` GC-MS (full-scan profile and
MRM/targeted) and `.lcd` LC-MS across IT-TOF (profile) and QTOF
(centroid) acquisitions.

Documentation: [sigilweaver.app/openszraw/docs](https://sigilweaver.app/openszraw/docs)

## Known issues and gaps

These apply to openszraw 0.4.0. Figures come
from checks run on the public validation corpus (see
[CORPUS.md](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md)).

- **LC-MS QTOF (`.lcd`, LCMS-9030) m/z values are wrong.** Intensities
  decode correctly, but the m/z axis is distorted: isotope partners of
  intense peaks appear about 1100/m Da apart (1.86 Da at m/z 592, 1.04 Da
  at m/z 1062) instead of 1.003 Da. Measured in two independent
  accessions; this affects all 416 QTOF files in the corpus. Do not use
  QTOF m/z values for identification or accurate-mass work. The error
  depends on m/z, so a single offset does not correct it.
- **LC-MS IT-TOF (`.lcd`) m/z values are not validated.** The per-file
  calibration fits the file's own tuning data, but in the files checked
  no isotope structure or cluster of known background ions could be
  found. Noise points map to m/z values up to about 780,000, and peaks
  just above the calibration intercept map to m/z below 1. This affects
  all 2,230 IT-TOF files in the corpus. Treat IT-TOF m/z as unverified.
- **IT-TOF peaks at or below the calibration intercept are left out.** A
  time-bin index at or below the intercept `b` has no m/z: the inverted
  calibration folds it back to a small positive value (0.0003 to 0.025)
  that is not a mass. The reader leaves those peaks out of `mz` and
  `intensity` and records them. The spectrum carries the warning
  `openszraw.uncalibrated_peaks_dropped`, `extra["openszraw.uncalibrated_peaks_dropped"]`
  holds the count and `extra["openszraw.uncalibrated_peaks"]` holds the
  dropped `index:intensity` pairs, separated by `;`. `total_ion_current`
  is the sum of the peaks written. In the three files checked this is
  0.06 to 0.8 percent of peaks (804 of 1.43 M, 2,591 of 1.18 M, 11,776 of
  1.53 M), and 735 of 1,340, 949 of 1,332 and 2,918 of 3,002 spectra
  lose at least one.
- **IT-TOF MS level and polarity are not decoded.** Every spectrum is
  labelled MS1 with no polarity, although files carry up to 4 interleaved
  acquisition channels that likely alternate polarity or MS level. Treat
  `ms_level` as unreliable on IT-TOF files. The channel is exposed as
  `extra["openszraw.channel_index"]`, but which channel is which is not
  known.
- **LC-MS/MS triple quadrupole `.lcd` (LCMS-8040/8050/8060) is not
  supported.** `Reader::open` refuses these files (275 corpus files).
- **Polarity is not reported for any file type.** mzML export writes
  spectra with no polarity term.
- **Collision energy and activation are not decoded.** `.qgd` MRM and
  QTOF MS2 records have `collision_energies` empty and no `activation`.
  mzML export marks the activation as unknown (an `openmassspec: activation`
  userParam with value `unknown`) and writes no collision energy, so the
  `unknown_collision_energy_unit` check never fires for this reader.
- **Empty spectra are written and flagged.** A scan with no peaks (for
  example 1,163 of 7,406 in one `MTBLS14820` file, and many IT-TOF scans)
  carries Core's `empty_spectrum` warning and declares no base peak or m/z
  range. `WriteSummary::has_warnings` is true for such a file; that is not
  a failure.
- **Spectra with no `Retention Time` record have no retention time.**
  Core's writer rejects them (`MissingRetentionTime`). No corpus file
  checked has this.
- **QTOF MS2 spectra have no precursor m/z.** They link to their MS1 scan
  only.
- **Single quadrupole `.lcd` (LCMS-2020): one stored pair per affected
  spectrum is out of m/z order.** About 17 to 18 percent of spectra (402
  to 434 of 2,400 in each of the 8 `MTBLS1960` files) store a pair, usually
  the highest-intensity peak, away from its m/z position. The reader reads
  each `(m/z, intensity)` pair as stored and stable-sorts the pairs by
  m/z; the summed intensity is unchanged (it equals the file's `TIC Data`).
  The spectrum carries the warning `openszraw.mz_reordered` and
  `extra["openszraw.mz_reordered_pairs"]` holds how many pairs were out of
  place. Why the file stores the pair there is not known.
- **No TIC or base-peak chromatograms are produced.** LC detector
  chromatograms are decoded only for the few files whose format is
  understood.
- **About 0.2 percent of QTOF scans fail to decode** (31 to 81 of about
  7,400 per file in `MTBLS14820`, "unsupported intensity width"). Each one is an
  `Err` item from `iter_spectra` (counted by `Reader::skipped_scans()`),
  so mzML export of an affected file stops there unless the writer is
  told to skip bad records (`RecordErrorPolicy::Skip`); there is no
  `--allow` check for it. In Python,
  `RawReader` and `iter_spectra` raise `RuntimeError` for such a scan
  unless called with `skip_errors=True`, which leaves the scan out, counts
  it in `skipped_scans` (on the reader and on the `iter_spectra` iterator)
  and reports the count with a `RuntimeWarning`.

What has been checked and holds:

- **GC-MS `.qgd`** (full-scan profile and MRM): m/z is consistent with
  unit-resolution GC-MS (93 percent of the 5 most intense peaks per
  spectrum lie within 0.1 of an integer mass). Every MRM spectrum carries
  its precursor target m/z; the transitions have not been cross-checked
  against the acquisition method. Two `.qgd` files are checked in CI.
- **Single quadrupole `.lcd`**: summed intensity matches the file's own
  TIC stream, and m/z sits on a 0.1 grid from 400 to 2000, as expected for
  a unit-resolution instrument. No reference ions were available to check
  mass accuracy directly.
- **QTOF intensities**: the maximum intensity of every non-empty spectrum
  equals the base-peak intensity the file records (150,731 of 150,731
  sampled spectra). Retention times never step backwards in a 63-file
  sample covering every supported variant.

## Install

**Prefer [`openmassspec-io`](https://github.com/Sigilweaver/OpenMassSpec)
with the `shimadzu` feature/extra** unless you need this parser standalone
(minimal dependencies, or building your own abstraction) - the umbrella
gives you format auto-detection, mzML conversion, and Arrow streaming
across all wired-in vendors for free:

```sh
cargo add openmassspec-io --features shimadzu
```

```sh
pip install openmassspec[shimadzu]
```

Standalone:

Rust:

```sh
cargo add openszraw
```

Python:

```sh
pip install openszraw
```

## Quickstart

Rust:

```rust
use openszraw::reader::Reader;
use openmassspec_core::SpectrumSource;

let mut reader = Reader::open("sample.lcd")?;
for spectrum in reader.iter_spectra() {
    let spectrum = spectrum?;
    println!("{}: {} peaks", spectrum.native_id, spectrum.mz.len());
}
```

Python:

```python
import openszraw

reader = openszraw.RawReader("sample.lcd")
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum.mz))
```

`Reader::open` (and `RawReader`) auto-detects `.qgd` vs `.lcd` IT-TOF vs
`.lcd` QTOF from the file's internal CFBF stream layout, never from the
filename or extension alone.

See the [docs site](https://sigilweaver.app/openszraw/docs) for the full
guide, format specification, and API reference.

## License

Apache-2.0. See [LICENSE](https://github.com/Sigilweaver/OpenSZRaw/blob/main/LICENSE).

The format specification was developed by binary analysis of public
mass-spectrometry datasets (PRIDE, MassIVE, and MetaboLights accessions).
See [CORPUS.md](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md) and [ATTRIBUTION.md](https://github.com/Sigilweaver/OpenSZRaw/blob/main/ATTRIBUTION.md).
