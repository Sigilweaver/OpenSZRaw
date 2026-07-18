---
sidebar_position: 3
---

# Quickstart

## Rust

```rust
use openszraw::reader::Reader;
use openmassspec_core::SpectrumSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = Reader::open("sample.lcd")?;
    for spectrum in reader.iter_spectra() {
        println!("{}: {} peaks", spectrum.native_id, spectrum.mz.len());
    }
    Ok(())
}
```

## Python

```python
import openszraw

reader = openszraw.RawReader("sample.lcd")
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum))
```

`Reader::open` (and `RawReader`) auto-detects `.qgd` GC-MS vs `.lcd`
IT-TOF vs `.lcd` QTOF from the file's internal CFBF stream layout -
never from the filename or extension alone. See
[Format variants](./guide/format-variants) for what distinguishes the
three.

## Next

- [Reader API](./guide/reader)
- [Format variants](./guide/format-variants)
- [Format specification](./format/overview)
