---
sidebar_position: 1
slug: /
---

# OpenSZRaw

:::info Part of the OpenMassSpec stack

OpenSZRaw is one of the vendor readers in
[OpenMassSpec](https://sigilweaver.app/openmassspec/docs/), a Rust- and
Python-native stack for mass-spectrometry raw-file access. Sibling readers:
[OpenTFRaw](https://sigilweaver.app/opentfraw/docs/) (Thermo `.raw`),
[OpenWRaw](https://sigilweaver.app/openwraw/docs/) (Waters `.raw/`),
[OpenTimsTDF](https://sigilweaver.app/opentimstdf/docs/) (Bruker `.d/`),
[OpenARaw](https://sigilweaver.app/openaraw/docs/) (Agilent `.d/`),
[OpenSXRaw](https://sigilweaver.app/opensxraw/docs/) (SCIEX `.wiff`).

:::

OpenSZRaw is a Rust library (with Python bindings) that reads Shimadzu
LabSolutions mass-spectrometry raw data: `.qgd` GC-MS files and `.lcd`
LC-MS files, covering both the IT-TOF and QTOF (9030-series) instrument
families.

It runs with no dependency on any Shimadzu SDK or software. The format
was decoded by clean-room binary analysis of public mass-spectrometry
datasets (PRIDE, MassIVE, and MetaboLights accessions); see
[`CORPUS.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md).

## What it covers

| Component                                          | Status                                                          |
| --------------------------------------------------- | ---------------------------------------------------------------- |
| OLE2/CFBF container (shared by all three variants)  | supported                                                       |
| `.qgd` GC-MS, full-scan profile mode                | supported                                                       |
| `.qgd` GC-MS, MRM/targeted mode                     | supported                                                       |
| `.lcd` IT-TOF (run-length-encoded profile spectra)  | opens; per-file m/z calibration applied but not validated - see [known limitations](./format/known-limitations) |
| `.lcd` QTOF / 9030 series (centroid spectra)        | opens; intensities correct, m/z values wrong - see [known limitations](./format/known-limitations) |
| QTOF MS2 precursor m/z (`QTFL RawData/DDA`)         | not yet decoded                                                 |
| PDA / UV chromatogram streams                       | out of scope for this reader; encoding not decoded              |

`Reader::open` (and Python's `RawReader`) auto-detects `.qgd` vs `.lcd`
IT-TOF vs `.lcd` QTOF from the file's internal CFBF stream layout, never
from the filename or extension alone.

:::warning LC-MS m/z is not reliable yet
QTOF (`.lcd`, LCMS-9030) m/z values are wrong: the m/z axis is distorted,
so isotope peaks appear about 1100/m Da apart instead of 1.003 Da. IT-TOF
m/z values are not validated. Do not use LC-MS QTOF or IT-TOF m/z values
for identification or accurate-mass work. GC-MS `.qgd` m/z is consistent
with unit-resolution data. See
[Known issues and gaps](https://github.com/Sigilweaver/OpenSZRaw#known-issues-and-gaps)
in the README.
:::

## Status

Published on crates.io and PyPI as `openszraw`, v0.2.0. Wired into
[openmassspec-io](https://github.com/Sigilweaver/OpenMassSpec) 1.5.0+
as a `shimadzu` feature. See [Install](./install) for how to add it to
your project today.

## Next steps

- [Install](./install) from source.
- Run through the [Quickstart](./quickstart).
- Read the [Format specification](./format/overview) for the binary
  layer.

## License

OpenSZRaw is Apache-2.0 licensed. See [License](./license).
