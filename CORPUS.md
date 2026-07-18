# OpenSZRaw Validation Corpus

Current size: approximately 960 MB across 94 files, spanning 6 accessions
across three public repositories. 42 files are `.qgd` (GC-MS); 52 are
`.lcd` (LC-MS), split across the IT-TOF and QTOF instrument families.

| Accession | Source | Files | Format |
|---|---|---|---|
| PXD034978 | PRIDE | 19 | `.qgd` |
| PXD019638 | PRIDE | 23 | `.qgd` |
| PXD025121 | PRIDE | 31 | `.lcd` (IT-TOF) |
| PXD020792 | PRIDE | 5 | `.lcd` (IT-TOF) |
| MTBLS432 | MetaboLights | 15 of 93 available | `.lcd` (IT-TOF) |
| MSV000084197 | MassIVE | 1 | `.lcd` (QTOF) |

## Sources

- **PRIDE Archive** (https://www.ebi.ac.uk/pride/): Perez-Riverol Y et al.
  "The PRIDE database and related tools and resources in 2019: improving
  support for quantification data." Nucleic Acids Res. 2019;47(D1):D442-D450.
  doi:10.1093/nar/gky1106
- **MassIVE** (https://massive.ucsd.edu/): Wang M et al. "Assembling the
  Community-Scale Discoverable Human Proteome." Cell Syst. 2018;7(4):412-421.
  doi:10.1016/j.cels.2018.08.004
- **MetaboLights** (https://www.ebi.ac.uk/metabolights/): Haug K et al.
  "MetaboLights: a resource evolving in response to the needs of its
  scientific community." Nucleic Acids Res. 2020;48(D1):D440-D444.
  doi:10.1093/nar/gkz1019

All three publish datasets under CC-BY or equivalent open licences.

Note: PRIDE's generic "Shimadzu instrument model" CV term undercounts real
Shimadzu submissions - some submitters tag the specific instrument model
instead (e.g. "LCMS-IT-TOF", "GCMS-QP2010SE"), and at least one real
corpus project (PXD019638) is mistagged with no Shimadzu-identifying
metadata at all, only discoverable by searching the `.qgd` extension
directly. Any future re-sourcing pass should search file-extension
keywords alongside instrument-name keywords, not trust the `instruments`
field alone.

## Fetch tooling

Two gitignored, local-only research CLIs under `re/src/analysis/`:

    python -m analysis.pride search <query>       # find Shimadzu PRIDE projects
    python -m analysis.pride files <accession>     # list a project's Shimadzu files
    python -m analysis.pride fetch <accession>     # download .lcd/.qgd files
    python -m analysis.pride catalog               # rebuild Data/SZRaw/index.csv

    python -m analysis.external list                     # list known MassIVE/MetaboLights datasets
    python -m analysis.external files <accession>         # list a dataset's files
    python -m analysis.external fetch <accession> [--max-files N]  # download files

Both fetch commands HEAD-check remote content-length before skipping a
file that already exists locally, rather than trusting bare existence -
an earlier version of `external.py` skipped on existence alone and left
one MetaboLights file silently truncated in the corpus.

## Provenance record

`Data/SZRaw/index.csv` records which source and accession each local
file came from, plus its extension and size. To trace any file back to
its source:

    https://www.ebi.ac.uk/pride/archive/projects/<PXD_ACCESSION>
    https://massive.ucsd.edu/ProteoSAFe/dataset.jsp?accession=<MSV_ACCESSION>
    https://www.ebi.ac.uk/metabolights/<MTBLS_ACCESSION>

## Limitations

- No QQQ-specific (e.g. LCMS-8060/8050 triple-quad) sample is confirmed
  in the corpus yet - only IT-TOF, QTOF, and unspecified/older instrument
  contexts so far.
- MassIVE has only one confirmed file in the corpus; its dataset browser
  renders the file tree client-side, so no full-manifest listing API has
  been found. `MSV000084197` is the only QTOF-family instance available,
  which is why QTOF's format docs and reader coverage rest on a single
  file rather than a cross-file average.
- MTBLS432 has 93 real `.lcd` files total; only 15 are fetched (a
  deliberately representative subset, not exhaustive).
- `.gcd` (older GCsolution, non-MS GC data) is out of scope for a
  mass-spec reader and has no corpus representation by design.
