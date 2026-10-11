# OpenSZRaw

[![CI](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/openszraw.svg)](https://crates.io/crates/openszraw)
[![PyPI](https://img.shields.io/pypi/v/openszraw.svg)](https://pypi.org/project/openszraw/)
[![docs.rs](https://img.shields.io/docsrs/openszraw)](https://docs.rs/openszraw)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
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

These apply to the published releases (openszraw 0.2.0 and earlier) and to
current `main`. Figures come from checks run on the public validation
corpus (see [CORPUS.md](CORPUS.md)).

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
  found. 21 of 29 sampled files have spectra that are not sorted by m/z,
  and noise points map to m/z values up to about 780,000. This affects all 2,230 IT-TOF
  files in the corpus. Treat IT-TOF m/z as unverified.
- **IT-TOF MS level and polarity are not decoded.** Every spectrum is
  labelled MS1 with no polarity, although files carry up to 4 interleaved
  acquisition channels that likely alternate polarity or MS level. Treat
  `ms_level` as unreliable on IT-TOF files. The channel is exposed as
  `extra["openszraw.channel_index"]`, but which channel is which is not
  known.
- **LC-MS/MS triple quadrupole `.lcd` (LCMS-8040/8050/8060) is not
  supported.** `Reader::open` refuses these files (275 corpus files).
- **Polarity is not reported for any file type.**
- **QTOF MS2 spectra have no precursor m/z.** They link to their MS1 scan
  only.
- **Single quadrupole `.lcd` (LCMS-2020): some spectra are not sorted by
  m/z** (all 5 sampled files, about 18 percent of spectra).
- **No TIC or base-peak chromatograms are produced.** LC detector
  chromatograms are decoded only for the few files whose format is
  understood.
- **About 0.2 percent of QTOF scans fail to decode.** Each is logged and
  counted by `Reader::skipped_scans()`.

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

Apache-2.0. See [LICENSE](LICENSE).

The format specification was developed by binary analysis of public
mass-spectrometry datasets (PRIDE, MassIVE, and MetaboLights accessions).
See [CORPUS.md](CORPUS.md) and [ATTRIBUTION.md](ATTRIBUTION.md).
