# OpenSZRaw Validation Corpus

Current size: approximately 2.9 GB across 151 files, spanning 9 accessions
across three public repositories. 47 files are `.qgd` (GC-MS: 2 established
accessions plus 1 new GC-MS/MS accession); 104 are `.lcd` (LC-MS), split
across the IT-TOF, QTOF, and (new) QQQ instrument families.

| Accession | Source | Files | Format | Instrument family | Acquisition mode |
|---|---|---|---|---|---|
| PXD034978 | PRIDE | 19 | `.qgd` | GC-MS (single quad) | MRM / targeted (Variant B, `docs/format/02`) |
| PXD019638 | PRIDE | 23 | `.qgd` | GC-MS (single quad) | Full-scan profile (Variant A, `docs/format/02`) |
| PXD025121 | PRIDE | 31 | `.lcd` | IT-TOF | Profile (RLE), polarity/MS-level not resolved (`docs/format/06` #2) |
| PXD020792 | PRIDE | 5 | `.lcd` | IT-TOF | Profile (RLE), polarity/MS-level not resolved |
| MTBLS432 | MetaboLights | 45 of 93 available | `.lcd` | IT-TOF | Profile (RLE); filenames imply alternating pos/neg, not decoded from data |
| MSV000084197 | MassIVE | 1 | `.lcd` | QTOF (LCMS-9030) | Centroid, DDA (MS1 event_id=1, MS2 event_id 2-4 seen) |
| MTBLS14820 | MetaboLights | 10 | `.lcd` | QTOF (LCMS-9030) | Centroid, DDA (MS1 event_id=1, MS2 event_id=2 only in this dataset - narrower than MSV000084197's 2-4, not contradictory) |
| MTBLS12691 | MetaboLights | 12 | `.lcd` | QQQ (LCMS-8060, triple quadrupole) | MRM-targeted (per study protocol: predefined transitions from Shimadzu's Primary Metabolites method package) - **files fetched but not yet decodable by the reader, see Limitations** |
| MTBLS11411 | MetaboLights | 5 | `.qgd` | GC-MS/MS (GC/MS-TQ8050 NX, triple quad) | Decodes via the existing Variant B (u64-index) path; scan-header `format` field reads `3` (not the `2`/implicit-`0x18` values docs/format/02 documents for Variants A/B) - decodes cleanly and passes conformance, but this field value itself is not otherwise investigated, see `docs/format/02` addendum |

Instrument family and acquisition-mode notes above are drawn from two
different kinds of evidence, called out per-row so it's clear what's
actually knowable versus inferred:
- **Format column** and IT-TOF/QTOF/QQQ/GC-MS **family**: derived from
  which top-level CFBF storage the reader itself detects
  (`docs/format/01-ole2-container.md`), or (for the two new MetaboLights
  accessions the reader can open) directly confirmed from each study's
  free-text instrument description in its own `i_Investigation.txt` -
  MetaboLights' structured "Instrument" CV field is usually left blank by
  submitters, so the free-text protocol description was checked instead,
  not the CV metadata.
- **Acquisition mode**: profile-vs-MRM and MS1/MS2 event structure is
  either read directly from the decoded scan/event data (QTOF's
  `event_id` cycle structure, GC-MS's Variant A/B split), or - where
  marked as such - taken from the submitting study's own methods
  description, which is treated as a claim about the *experiment*, not
  independently re-derived from bytes.

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

**2026-07-18 re-sourcing pass**: [OmicsDI](https://www.omicsdi.org/)'s
dataset-search REST API
(`https://www.omicsdi.org/ws/dataset/search?query=...`) turned out to be
a considerably better lead-finding tool than any single host's own
search - it aggregates PRIDE, MassIVE, MetaboLights, and GNPS results in
one place and returned relevant hits (including all three new accessions
below) on plain-English queries like "Shimadzu 9030" or "Shimadzu triple
quadrupole" that the individual archives' own instrument-filter search
either lacks or handles poorly. Worth trying first in any future
re-sourcing pass. Once a candidate accession is found this way, its
actual raw-file list still needs independent verification (per the
extension-search gotcha above) - OmicsDI's own metadata can be as sparse
as the underlying archive's.

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

`analysis.external`'s `fetch` also supports subdirectory-qualified
`known_files` entries (e.g. `"RAW_FILES_2/foo.lcd"`) for datasets whose
raw files are split across several remote subdirectories with no single
flat listing URL (see MTBLS12691 below) - the remote path is preserved in
the download URL, but the local file is flattened to just its basename,
since the corpus layout (and `analysis.pride catalog`) expects one flat
directory per accession.

`re/src/analysis/qtfl_corroborate.py` (new this session) independently
re-checks `docs/format/05-qtfl-centroid.md`/`06-known-limitations.md`'s
QTOF centroid-decode claims (intensity byte width, BPI-vs-max(intensity)
consistency, `event_id` MS1/MS2 cycle structure) against any QTFL `.lcd`
file(s) passed on the command line - used this session to check
MTBLS14820 against the original claims, which were previously backed by
only `MSV000084197`.

## Provenance record

`Data/SZRaw/index.csv` records which source and accession each local
file came from, plus its extension and size. To trace any file back to
its source:

    https://www.ebi.ac.uk/pride/archive/projects/<PXD_ACCESSION>
    https://massive.ucsd.edu/ProteoSAFe/dataset.jsp?accession=<MSV_ACCESSION>
    https://www.ebi.ac.uk/metabolights/<MTBLS_ACCESSION>

## Limitations

- **QQQ (`.lcd`, MTBLS12691) is fetched but not decodable by the reader
  yet.** These files use a `TLM Raw Data` storage (structurally closer to
  `.qgd` GC-MS's `MS Raw Data`/`Spectrum Index` pattern than to either
  `TTFL Raw Data` or `QTFL RawData`), which `crates/openszraw` does not
  currently parse. Worse, every `.lcd` file - including these QQQ ones -
  carries an always-present but *empty* `QTFL RawData` storage as
  boilerplate, and `raw::detect_variant` currently checks
  `TTFL Raw Data` then falls back to treating *any* remaining
  `QTFL RawData` presence as the QTOF variant - so these files are
  currently misdetected as (broken) QTOF and fail to open with a
  confusing "stream 'QTFL RawData/Centroid Index' not found" error,
  rather than a clear "QQQ/TLM variant not yet supported" message. Filed
  as a follow-up issue rather than fixed here (out of scope for a
  corpus/docs-only pass) - see
  [Sigilweaver/OpenSZRaw#5](https://github.com/Sigilweaver/OpenSZRaw/issues/5).
- MTBLS432 has 93 real `.lcd` files total; 45 are now fetched (up from
  15) - still a deliberately representative subset, not exhaustive.
- `.gcd` (older GCsolution, non-MS GC data) is out of scope for a
  mass-spec reader and has no corpus representation by design.
