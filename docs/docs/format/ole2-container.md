---
sidebar_position: 2
---

# OLE2 container

## Status: Confirmed

Shimadzu `.lcd` and `.qgd` raw data files are standard Microsoft OLE2
(Compound File Binary Format, CFBF) containers. They can be read using
standard libraries like Python's `olefile` or C's `libgsf`, without any
Shimadzu-specific tooling - `openszraw` uses the Rust
[`cfb`](https://crates.io/crates/cfb) crate for the same purpose.

## General structure

Every file contains a `File Property` stream (XML metadata, typically a
4-byte little-endian length prefix followed by
`<?xml version="1.0"?>`), plus one or more instrument-specific raw-data
directories.

## Instrument-specific directories

The raw mass-spectrometry data is stored in different root-level
directories depending on the instrument family:

| Storage | Format | Key streams |
| --- | --- | --- |
| `GCMS Raw Data/` | `.qgd` GC-MS | `MS Raw Data`, `Spectrum Index`, `Retention Time`, `TIC Data` |
| `TTFL Raw Data/` | `.lcd` IT-TOF | `MS Raw Data`, `Data Index`, `Retention Time`, `TIC Data 0`, `Sum TIC Data` |
| `QTFL RawData/` | `.lcd` QTOF (9030 series) | `Centroid Data`, `Centroid Index`, `Centroid BPC`, `Centroid SumTIC`, `Retention Time` |
| `PDA 3D Raw Data/` | PDA / UV detector (either `.lcd` family) | `3D Raw Data`, `Wavelength Table`, `Status` |

`Reader::open` checks which of these storages is present to decide how
to decode the file - see [Format variants](../guide/format-variants).

## Metadata streams

The `File Property` stream is universally present across all instrument
families and encoded as UTF-8 XML with a 4-byte size prefix. Other
metadata streams such as `2D Data Item` and `2D Data Item U` (UTF-16LE)
also carry XML.

No embedded SQLite database is used anywhere in these formats; all data
is stored directly in binary OLE2 streams.
