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
