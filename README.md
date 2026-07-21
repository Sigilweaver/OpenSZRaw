# OpenSZRaw

[![CI](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenSZRaw/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/openszraw.svg)](https://crates.io/crates/openszraw)
[![PyPI](https://img.shields.io/pypi/v/openszraw.svg)](https://pypi.org/project/openszraw/)
[![docs.rs](https://img.shields.io/docsrs/openszraw)](https://docs.rs/openszraw)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

> Part of the [OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec)
> stack for mass spectrometry raw-file access.

Rust and Python reader for Shimadzu LabSolutions mass spectrometry raw
data (`.lcd` LC-MS, `.qgd` GCMS, `.gcd` GC), with no Shimadzu SDK or
software dependency. Covers `.qgd` GC-MS (full-scan profile and
MRM/targeted) and `.lcd` LC-MS across IT-TOF (profile) and QTOF
(centroid) acquisitions.

Documentation: [sigilweaver.app/openszraw/docs](https://sigilweaver.app/openszraw/docs)

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
