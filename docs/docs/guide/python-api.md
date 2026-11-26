---
sidebar_position: 5
---

# Python API

The Python bindings (`crates/openszraw-py`, a PyO3 crate) expose a
small, eager reader built on the same Rust core as the
[Rust reader](./reader). Install with `pip install openszraw`, see
[Install](../install).

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
| `RawReader(path, skip_errors=False)` | constructor | Open the `.qgd`/`.lcd` file at `path` and decode it. A scan that cannot be decoded raises `RuntimeError`; with `skip_errors=True` it is left out, counted in `skipped_scans` and reported once with a `RuntimeWarning` |
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
| `retention_time_sec`  | `float \| None` | Retention time in seconds, `None` when the file has none |
| `scan_mode`           | `str \| None` | `"profile"` or `"centroid"` |
| `analyzer`            | `str \| None` | Core's analyzer label: `"sqms"`, `"tqms"` or `"tof"` for this reader |
| `warnings`            | `list[dict]`  | What the reader flagged on this spectrum, each `{"code", "message"}`; empty when nothing was |
| `extra`               | `dict[str, str]` | Reader-specific values, for example `openszraw.mz_reordered_pairs` |
| `precursor`           | `dict \| None` | Precursor of an MS2 spectrum; keys `target_mz`, `selected_mz`, `isolation_width`, `charge`, `intensity`, `collision_energies`, `precursor_native_id`, `activation`, `analyzer`, `ccs` |

`precursor["collision_energies"]` is a list of `{"value", "unit"}` dicts
(`unit` is `"ev"`, `"nce"` or `None`). This reader does not decode
collision energy, so the list is empty.

`len(spectrum)` returns the peak count. For `.lcd` QTOF files, `mz` is
wrong (distorted axis) and MS2 precursor m/z is not populated. For
`.lcd` IT-TOF files, `mz` comes from a per-file calibration derived from
the file's own TOF tuning data and is not validated.
See [Scan data](./scan-data) for the details and the current
[known limitations](../format/known-limitations).

```python
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum))
mz, intensity = spectrum.mz, spectrum.intensity
```

## `iter_spectra`

`openszraw.iter_spectra(path, skip_errors=False)` decodes spectra one at a
time without holding the acquisition in memory. A scan that cannot be
decoded raises `RuntimeError` and ends the iteration. With
`skip_errors=True` it is left out and counted: the iterator's
`skipped_scans` is the count so far, final when the iteration ends, and a
`RuntimeWarning` reports it then.

```python
stream = openszraw.iter_spectra(path, skip_errors=True)
for spectrum in stream:
    ...
print(stream.skipped_scans)
```

## Next

- [Reader API](./reader) (Rust)
- [Scan data layouts](./scan-data)
