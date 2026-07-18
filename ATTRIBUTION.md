# Credits

## Prior art

`ethanbass/chromConverter` (https://github.com/ethanbass/chromConverter,
R + Python/`olefile`) has done the most public reverse-engineering of
`.lcd` internals, but it is incomplete by its own admission (a guessed
buffer-sizing heuristic, no complete MS-stream support) and was never
used as a dependency or copied from. It was treated the same as any
other external published reference: read for orientation, then verified
independently against this project's own corpus bytes before trusting
anything, and re-derived from scratch where it fell short (notably the
`.qgd` scan-encoding variants and the IT-TOF RLE payload format, neither
of which chromConverter documents). ProteoWizard has no Shimadzu reader
at all (confirmed via its maintainer mailing list).

The `.lcd`/`.qgd` container itself is Microsoft's public Compound File
Binary Format (CFBF/OLE2) - reading its stream tree is not reverse
engineering Shimadzu's own work, only the contents and layout of the
streams inside it are. The `GCMS Raw Data`, `TTFL Raw Data`, and
`QTFL RawData` payload encodings are Shimadzu-specific and were decoded
entirely from corpus byte analysis, documented in [docs/format/](docs/format/).

PDA/chromatogram stream decoding and IT-TOF m/z calibration remain open
(see [docs/format/06-known-limitations.md](docs/format/06-known-limitations.md));
nothing in this project's documentation of those gaps was learned from
Shimadzu software, SDKs, or any third party.

## Standards

The mzML output follows the [HUPO-PSI mzML 1.1.0 specification](https://www.psidev.info/mzML)
and uses CV terms from the PSI-MS ontology (psi-ms.obo):

    Deutsch EW et al. "A guided tour of the Trans-Proteomic Pipeline."
    Proteomics. 2010;10(6):1150-9. doi:10.1002/pmic.200900375

## Validation corpus

Corpus files were downloaded from the PRIDE Archive, MassIVE, and
MetaboLights:

    Perez-Riverol Y et al. "The PRIDE database and related tools and resources in 2019:
    improving support for quantification data." Nucleic Acids Res. 2019;47(D1):D442-D450.
    doi:10.1093/nar/gky1106

    Wang M et al. "Assembling the Community-Scale Discoverable Human Proteome."
    Cell Syst. 2018;7(4):412-421. doi:10.1016/j.cels.2018.08.004

    Haug K et al. "MetaboLights: a resource evolving in response to the
    needs of its scientific community." Nucleic Acids Res. 2020;48(D1):D440-D444.
    doi:10.1093/nar/gkz1019

## Rust dependencies

- [cfb](https://github.com/mdsteele/rust-cfb) -- Compound File Binary
  Format (CFBF/OLE2) reader (Matthew D. Steele, MIT)
- [byteorder](https://github.com/BurntSushi/byteorder) -- little/big-endian binary decoding (Andrew Gallant, MIT/Unlicense)
- [thiserror](https://github.com/dtolnay/thiserror) -- derive macro for Error impls (David Tolnay, MIT/Apache-2.0)
