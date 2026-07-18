# OpenSZRaw

Rust and Python reader for Shimadzu LabSolutions mass spectrometry raw
data (`.lcd` LC-MS, `.qgd` GCMS, `.gcd` GC), clean-room
reverse-engineered with no Shimadzu SDK or software dependency.

> Sibling readers in the same stack:
> [OpenTFRaw](https://github.com/Sigilweaver/OpenTFRaw) (Thermo),
> [OpenWRaw](https://github.com/Sigilweaver/OpenWRaw) (Waters),
> [OpenTimsTDF](https://github.com/Sigilweaver/OpenTimsTDF) (Bruker),
> [OpenARaw](https://github.com/Sigilweaver/OpenARaw) (Agilent),
> [OpenSXRaw](https://github.com/Sigilweaver/OpenSXRaw) (SCIEX).

## Status

Published on crates.io (`openszraw`) and PyPI (`openszraw`), v0.1.0. A
Rust reader (`crates/openszraw`) implements all three confirmed raw
data variants: `.qgd` GC-MS (full-scan profile and MRM/targeted), `.lcd`
IT-TOF (run-length-encoded profile spectra, calibrated to physical m/z
via the file's own embedded TOF tuning data), and `.lcd` QTOF (centroid).
See `docs/format/` for the byte-level format specs and
`docs/format/06-known-limitations.md` for what is deliberately not yet
resolved (per-channel polarity, some MS2 precursor m/z values). Python
bindings (`crates/openszraw-py`) mirror the Rust API. Wired into
[openmassspec-io](https://github.com/Sigilweaver/OpenMassSpec) 1.5.0+
as a `shimadzu` feature. See the sourcing strategy in the ops repo's
[SCOPING_PLAN.md](https://github.com/Sigilweaver/ops/blob/main/SCOPING_PLAN.md)
and this repo's `re/ROADMAP.md` (local-only, gitignored) for the current
phase.

## Install

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

## License

Apache-2.0. See [LICENSE](LICENSE).

The format specification was developed by binary analysis of public
mass-spectrometry datasets (PRIDE, MassIVE, and MetaboLights accessions).
See [CORPUS.md](CORPUS.md) and [ATTRIBUTION.md](ATTRIBUTION.md).
