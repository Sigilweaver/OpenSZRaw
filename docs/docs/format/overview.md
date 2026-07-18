---
sidebar_position: 1
---

# Overview

Shimadzu LabSolutions and GCMSsolution raw data files (`.lcd`, `.qgd`)
are standard Microsoft OLE2 (Compound File Binary Format, CFBF)
containers - the same public container format used by legacy Microsoft
Office documents. Reading the container's stream tree is not reverse
engineering Shimadzu's own work; only the contents and layout of the
streams inside it are.

```
sample.qgd   - CFBF/OLE2 container: GCMS Raw Data/ storage
sample.lcd   - CFBF/OLE2 container: TTFL Raw Data/ or QTFL RawData/ storage
```

A `.lcd` file can be either of two unrelated instrument families
underneath (IT-TOF or QTOF) - the extension alone never tells you which;
see [Format variants](../guide/format-variants).

## Pages

| Page | Covers | Status |
| --- | --- | --- |
| [OLE2 container](./ole2-container) | Shared container structure across all variants | Confirmed |
| [GC-MS (.qgd)](./qgd-gcms) | `.qgd` scan index and payload encoding (profile and MRM modes) | Partial |
| [LC-MS IT-TOF (.lcd)](./lcd-ittof) | `.lcd` IT-TOF scan header and run-length-encoded payload | Partial |
| [LC-MS QTOF (.lcd)](./lcd-qtof) | `.lcd` QTOF centroid index and payload | Confirmed for the payload shape, with corrections |
| [Known limitations](./known-limitations) | What's deliberately unresolved across all variants | - |

## Clean-room provenance

Every byte-level claim on these pages came from binary analysis of
public mass-spectrometry datasets (PRIDE, MassIVE, and MetaboLights
accessions - see
[`CORPUS.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md))
plus the public CFBF/OLE2 container specification. No Shimadzu SDK,
LabSolutions/GCMSsolution/GCsolution software, or other vendor tooling
was used at any point - see
[`CONTRIBUTING.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CONTRIBUTING.md#vendor-software-and-clean-room-policy).

These pages are a curated summary. The full byte-level research record -
every field, every hypothesis tried and rejected, and the raw
verification evidence - lives in
[`docs/format/`](https://github.com/Sigilweaver/OpenSZRaw/tree/main/docs/format)
in the repository.
