---
sidebar_position: 5
---

# Python API

The Python bindings (`crates/openszraw-py`, a PyO3 crate) expose a
small, eager reader built on the same Rust core as the
[Rust reader](./reader). They are not yet published to PyPI - build
them locally with maturin, see [Install](../install).

```python
import openszraw

reader = openszraw.RawReader("sample.lcd")
```

`RawReader` auto-detects `.qgd` GC-MS vs `.lcd` IT-TOF vs `.lcd` QTOF
from the file's internal CFBF layout, exactly like the Rust reader (see
[Format variants](./format-variants)). Opening it decodes **every**
spectrum up front into memory, so construction is where the work
happens and subsequent access is cheap. For streaming access over large
acquisitions, use the Rust reader's `iter_spectra` instead.

## `RawReader`

| Member                 | Type       | Description                                                  |
| ---------------------- | ---------- | -------------------------------------------------------------- |
| `RawReader(path)`      | constructor | Open the `.qgd`/`.lcd` file at `path` and decode it             |
| `scan_count`           | `int`      | Number of decoded spectra                                      |
| `read_spectrum(index)` | `Spectrum` | The spectrum at zero-based `index` (raises if out of range)    |

```python
print(reader.scan_count)
for i in range(reader.scan_count):
    spectrum = reader.read_spectrum(i)
    ...
```

## `Spectrum`

| Attribute             | Type          | Description                          |
| --------------------- | ------------- | ------------------------------------ |
| `mz`                  | `list[float]` | m/z values (float64)                 |
| `intensity`           | `list[float]` | Intensities (float32)                |
| `ms_level`            | `int`         | MS level (1 for MS1, 2+ for MS/MS)   |
| `retention_time_sec`  | `float`       | Retention time in seconds            |

`len(spectrum)` returns the peak count. For `.lcd` IT-TOF files, `mz` is
currently a raw, uncalibrated time-bin index rather than physical m/z;
for `.lcd` QTOF MS2 spectra, precursor m/z is not yet populated. See
[Scan data](./scan-data) for the details and the current
[known limitations](../format/known-limitations).

```python
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum))
mz, intensity = spectrum.mz, spectrum.intensity
```

## Next

- [Reader API](./reader) (Rust)
- [Scan data layouts](./scan-data)
