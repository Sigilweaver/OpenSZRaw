---
sidebar_position: 4
---

# mzML export

OpenSZRaw doesn't ship a dedicated export binary yet, but since `Reader`
implements `openmassspec_core::SpectrumSource`, it can be written to mzML
using the same writer every reader in the OpenMassSpec stack uses, so
output is consistent across vendors:

```rust
use openszraw::reader::Reader;
use openmassspec_core::write_mzml;

let mut reader = Reader::open("sample.lcd")?;
let mut out = std::fs::File::create("output.mzML")?;
write_mzml(&mut reader, &mut out)?;
```

`write_mzml` iterates the reader's spectra via `SpectrumSource` (the same
stream described in [Reader](./reader)) and emits PSI-MS CV-annotated
mzML, with the source-file-format and instrument CV terms varying by
variant (`MS:1003009` "Shimadzu Biotech LCD format" for both `.lcd`
variants; the generic `MS:1000560` "mass spectrometer file format" node
for `.qgd`, since no PSI-MS CV term dedicated to `.qgd` exists yet).
Given the [current reader limitations](./reader#what-the-reader-does-not-yet-do)
(no IT-TOF m/z calibration, no decoded QTOF MS2 precursor m/z, no
polarity), exported mzML will carry those same gaps until the
underlying fields are decoded.
