---
sidebar_position: 1
---

# Reader

The entry point is `Reader`. `Reader::open` takes a path to a `.qgd` or
`.lcd` file, opens it as an OLE2/CFBF container, and inspects which
top-level storage is present (`GCMS Raw Data`, `TTFL Raw Data`, or
`QTFL RawData`) to detect which of the three on-disk variants it is -
never from the file extension or name. See
[Format variants](./format-variants) for what distinguishes them.

```rust
use openszraw::reader::Reader;

let reader = Reader::open("sample.lcd")?;
```

`Reader` implements `openmassspec_core::SpectrumSource`, the shared trait
every vendor reader in the OpenMassSpec stack implements:

```rust
use openmassspec_core::SpectrumSource;

let metadata = reader.run_metadata();
let scan_count = reader.spectrum_count_hint().unwrap_or(0);
println!("{} ({} scans)", metadata.source_file_name, scan_count);

let mut reader = reader;
for spectrum in reader.iter_spectra() {
    println!("{}\t{}\t{} peaks", spectrum.native_id, spectrum.ms_level, spectrum.mz.len());
}
```

Each yielded `SpectrumRecord` carries `index`, `native_id`, `ms_level`,
`retention_time_sec`, and `mz`/`intensity` arrays. `spectrum_count_hint`
is exact for `.lcd` IT-TOF and QTOF, but only approximate for `.qgd` MRM
data: each monitored transition within a scan expands into its own
`SpectrumRecord`, so the real record count can exceed the raw scan count
for MRM/targeted acquisitions - see
[GC-MS (.qgd)](../format/qgd-gcms#mrm--targeted-mode).

## What the reader does not yet do

- **IT-TOF polarity / MS level**: every IT-TOF spectrum reports
  `ms_level = 1` and `polarity = None`; the channel-to-polarity mapping
  implied by dataset naming (`..._pos-neg_NN.lcd`) has not been decoded.
- **QTOF MS2 precursor m/z**: MS2 spectra carry a `precursor_native_id`
  reference to the most recent MS1 scan, but not a decoded
  `target_mz`/`selected_mz` - the real value lives in the undecoded
  `QTFL RawData/DDA` stream.
- **`.qgd` polarity**: not populated; no polarity bit has been found in
  the scan header or `Spectrum Index` stream.
- **PDA / chromatogram streams**: `PDA 3D Raw Data` (the UV detector) and
  `LSS Raw Data` (chromatogram) are out of scope for this crate; their
  segment-level delta encoding has not been decoded - see
  [Known limitations](../format/known-limitations).

## Error handling

Public functions return `openszraw::Result<T>`. The error type is
`openszraw::Error`, which wraps the failure category (`Io`, `Parse`) and
a message.
