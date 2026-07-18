# 01. OLE2 Container Structure

**Status**: CONFIRMED

Shimadzu `.lcd` and `.qgd` raw data files are standard Microsoft OLE2 (Compound File Binary Format, CFBF) containers. They can be read using standard libraries like Python's `olefile` or C's `libgsf` without requiring any vendor DLLs.

## General Structure

The files consist of multiple directories and streams. A typical file will contain:
- A `File Property` stream: Contains XML metadata (typically starting with a 4-byte little-endian length prefix followed by `<?xml version="1.0"?>`).
- One or more specific raw data directories depending on the instrument type.

## Instrument-Specific Directories

The raw mass spectrometry data is stored in different root-level directories based on the instrument generation or family:

1. **GC-MS (e.g., PXD034978, PXD019638)**
   - Root directory: `GCMS Raw Data/`
   - Key streams: `MS Raw Data`, `Spectrum Index`, `Retention Time`, `TIC Data`.
   
2. **IT-TOF LC-MS (e.g., PXD020792, MTBLS432)**
   - Root directory: `TTFL Raw Data/`
   - Key streams: `MS Raw Data`, `Data Index`, `Retention Time`, `TIC Data 0`, `Sum TIC Data`.

3. **9030 QTOF LC-MS (e.g., MSV000084197)**
   - Root directory: `QTFL RawData/`
   - Key streams: `Centroid Data`, `Centroid Index`, `Centroid BPC`, `Centroid SumTIC`, `Retention Time`.

4. **PDA / UV Detectors**
   - Root directory: `PDA 3D Raw Data/`
   - Key streams: `3D Raw Data`, `Wavelength Table`, `Status`.

## Metadata Streams

The `File Property` stream is universally present across all instrument families and is encoded as UTF-8 XML data (with a 4-byte size prefix). Other metadata such as `2D Data Item` and `2D Data Item U` (UTF-16LE) also contain GUD-formatted XML.

This confirms the clean-room finding that no embedded SQLite databases are used; all data is stored directly in binary OLE2 streams.

## Addendum: directory-entry creation timestamps carry the acquisition start time

Unlike `.wiff` (OpenSXRaw), `.lcd`/`.qgd` files do not carry a
`\x05SummaryInformation` OLE2 property set. However, every CFBF directory
entry (storage or stream) has its own `created`/`modified` `FILETIME`
fields per `[MS-CFB]` 2.6.4, exposed by both `olefile`
(`OleFileIO.getctime(path)`) and the `cfb` Rust crate
(`Entry::created() -> SystemTime`) without any extra parsing. Real
Shimadzu files populate these for nearly every entry, and LabSolutions
writes almost all of a run's top-level storages within well under a
second of each other at run start - so the earliest non-zero `created`
value in the container is a reliable acquisition-start proxy. See
`docs/format/06-known-limitations.md` section 9 for the corpus-wide
internal-consistency evidence (sequential-injection timing regularity)
and `crates/openszraw::raw::timestamp` for the implementation.
