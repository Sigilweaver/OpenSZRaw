# 08. Metadata and Post-Processing Streams: corpus-wide map

**Status**: DISCOVERY

## Summary

Every `.lcd` file carries a large set of top-level CFBF storages beyond
the raw-spectra and chromatogram/PDA streams covered elsewhere in
`docs/format/`. None of these have been examined in any prior session -
`crates/openszraw` does not read any of them, and they are absent from
`docs/format/06-known-limitations.md`'s gap list because nobody had
looked yet. This document is a first-pass map (2026-07-21), not a
decode: enough to tell a future session what's here, roughly what
format each thing is in, and which pieces look most worth pursuing
first. Surveyed across one representative file per instrument family
(IT-TOF `MTBLS432`, QTOF `MSV000084197`, QQQ `MTBLS7425`/`MTBLS2376`,
single-quad `MTBLS1960` - see `docs/format/07` for that variant's raw
MS-data storage specifically, not covered here).

## Universal presence

These top-level storages/streams appear (by name, sometimes with an
`Original`/`Org` twin - LabSolutions appears to keep an "as-acquired"
copy alongside a possibly-user-edited working copy) across every
instrument family checked: `Audit Trail`, `Batch File Comment` (+
`Original`), `Batch File Extended Info` (+ `Org`), `Batch File
Property` (+ `Original`), `Chromatogram Parameters`, `Extended
Information`, `File Comment` (+ `Original`), `File Property` (+
`Original`), `Method File Comment` (+ `Original`), `Method File
Extended Info` (+ `Org`), `Method File Property` (+ `Original`), `Mass
Data Load Format` (+ `Original`), `Mass Data Processing` (+
`Original`), `Report Format` (+ `Original`), `SystemCheckResult`.
IT-TOF's older naming uses `LC ...`/`MS ...` prefixes (`LC Data
Processing`, `MS Data Load Format`, `LC Batch Table Original`,
`LSS Configuration`) where every other family uses `LSS ...` (`LSS Data
Processing`, `LSSExtStgObj Param`, `LSS Batch Table Original`, `LSS
BaselineCheck Parameters`) - plausibly an older LabSolutions schema
generation for the IT-TOF product line specifically, not confirmed.
Tuning stream names are instrument-model-specific (`TTFL Tuning`,
`LCMSQTOF Tuning`, `LCMSMS8050 Tuning`, `LCMSMS8060NX Tuning`, `MS
Tuning` + `LCMS2011 Tuning`/`LCMSMS3030 Tuning`/`LCMSMS8080Tuning` seen
together on QQQ/single-quad files, plausibly because LabSolutions
bundles tuning records for every model the installed software supports,
not just the one that acquired this specific file - not confirmed).

## 1. `File Property` / `Method File Property` / `File Comment` (CONFIRMED format, DISCOVERY content)

**Format, confirmed by direct decode**: a 4-byte length-ish prefix (its
exact meaning wasn't pinned down - the value does not match the actual
XML byte length in every case checked) followed by UTF-8 XML text,
`<?xml version="1.0"?><FileProperty>...</FileProperty>` (or
`<MethodFileProperty>` for the method variant - both schemas were found
concatenated back-to-back with no separator in `Method File Property`,
first block is a `FileProperty`, second is the method-specific one).
Free-text fields use a project-internal escaping scheme:
`@StoX@<hex-encoded-UTF-8>` (e.g. `@StoX@53797374656D2041646D696E6973747261746F72`
decodes to `System Administrator`) - confirmed by successfully decoding
every `@StoX@...` occurrence found in the corpus files checked. Numeric
fields (`dwLowGeneratedDateTime`, `dwHighGeneratedDateTime`, etc.) are
plain decimal text, sometimes negative (representing an unsigned 32-bit
value via two's complement - confirmed by successfully round-tripping
through `& 0xFFFFFFFF`).

**Cross-validated finding**: `dwLow/HighGeneratedDateTime` and
`dwLow/HighModifiedDateTime` are a standard Win32 `FILETIME` pair (100ns
ticks since 1601-01-01), independent of the CFBF directory-entry
`created`/`modified` timestamps `docs/format/01`'s addendum (resolving
#9) already uses for `RunMetadata::start_timestamp`. On
`MTBLS7425/1_16S_Negative.lcd`, the two sources agree closely:
`File Property`'s generated-time decodes to `2023-02-17 09:47:02.466`
vs. the earliest CFBF directory `created` timestamp of `2023-02-17
09:47:02.325` (140ms apart); modified-time decodes to `2023-02-17
10:20:37.352` vs. the latest directory `created` of `2023-02-17
10:20:36.290` (~1s apart). One file, one comparison - not yet checked
corpus-wide - but a genuine independent corroboration of the existing
#9 technique, not just a repeat of it.

**Also present, not yet extracted into `RunMetadata`**: `szVersion`
(LabSolutions version, e.g. `5.01`), `szGeneratedBy`/`szModifiedBy`
(operator name - `System Administrator` in the one file checked, likely
more informative on files from real named operators rather than
automated/shared instrument accounts), `szLocGMTDiffGenDateTime`
(`+09'00'` in the one file checked - Japan Standard Time, plausible for
a Japan-based lab or Shimadzu-default system clock, not itself evidence
of anything beyond the file's own recorded timezone), `dwCodePage`
(`932` = Shift-JIS, consistent with a Japanese-locale Windows install),
and per-file GUID-like `CreateFileID`/`UpdateFileID` structures (10
integer fields each, shaped like a Win32 `GUID` split into
`Data1`/`Data2`/`Data3`/`Data4[0..8]` - not yet confirmed against any
other GUID in the corpus). `File Comment`/`Method File Comment` were
empty (single null byte) in every file checked - plausibly just unused
free-text fields, not a parsing gap.

**Concrete next step**: this is plain, already-decodable XML - the
lowest-effort item in this whole document. Extracting
operator/version/timestamp fields into `RunMetadata` (there may be no
existing schema field for some of these - check `openmassspec_core`
before assuming) is a small, contained follow-up, unlike everything
else below.

## 2. `GUMM_Information` (`GUC.*.CONFIG`/`GUC.*.METHOD`, `GUM.CONFIG`/`GUM.METHOD`) - full instrument method/config (DISCOVERY, structure only)

Present on QTOF/QQQ/single-quad files (not IT-TOF, which has its own
older `LCMS2010 Instrument Parameters`/`TTFL Instrument Param`/`LC
Instrument Parameters` streams instead - not examined this pass
either). Sub-storages named per physical module, e.g.
`ShimadzuLC.1` (LC front end) and `ShimadzuLCMS3030.1` (MS side - "3030"
here is very likely an internal config-schema/product code, not
necessarily this file's actual instrument model, since the same file's
top-level tuning stream is named `LCMSMS8060NX Tuning`; not
disambiguated this session).

**Format, partially decoded**: outer content is UTF-16LE text (not
UTF-8, unlike `File Property`), itself XML-shaped:
`<GUD Type="SystemParameter" Rev="0" RevSrc="0">...</GUD>` with nested
`<UP Name="LCMS" ...><UPD ID="Mass"><Val>...</Val></UPD>...` elements.
At least one `<Val>` observed contains a **tab-separated** value list,
one field of which is itself a long hex-encoded binary blob prefixed
with an ASCII struct/module name (e.g. `43544C4D33303330506172616D65746572...`
decodes to `CTLM3030Parameters` followed by null-padded binary) - a
serialized instrument-parameter structure embedded as hex text inside
XML inside a CFBF stream, three encoding layers deep.
`GUMM_Information/GUMMSubStg/SystemInformation` (20KB) similarly
contains UTF-16LE XML with HTML-entity-escaped *nested* XML inside
attribute-like values (`&lt;GUD Type="ConnectInfo"&gt;...`), naming a
real instrument model string (`CBM-20A`, a Shimadzu system controller)
in one file checked - genuine per-instrument configuration data, not
boilerplate.

**Not decoded further**: the tab-separated field layout, the hex-blob's
internal binary structure, and the doubly-escaped nested-XML pattern
are all real open questions. `GUM.METHOD` alone was 137KB in the one
file inspected - this is plausibly the single largest concentration of
undecoded-but-real information in the whole container (full LC gradient
program, detector settings, MS acquisition method, tune parameters),
but decoding it is a substantial, multi-layer task, not a quick win.

## 3. `Mass Data Processing` / `Mass Data Load Format` - post-acquisition compound ID/quantitation results (DISCOVERY, real content confirmed)

The single most surprising find of this pass: LabSolutions' own
compound-identification/quantitation *results* - not raw instrument
data - are stored directly in the file, previously completely unknown
to this project. `Mass Data Processing` (one file's substream sizes,
`MTBLS7425/1_16S_Negative.lcd`): `Compound Table` (32,408 bytes),
`Compound Results` (12,080 bytes), `Compound Peak Table` (136,168
bytes, the largest single substream here), `Compound Calib Curve Info`
(5,472 bytes), `Calib Data File` (5,292 bytes), `Library Search
Param`/`Library Mlb Search Parameter`, `Quantitation Parameter`,
`Peak Picking Parameter`, `Mass Correction Parameter`, `QC Check
Parameter`, `Time Program` (several variants), plus grouping/AI-peak-
picking counterparts. `Mass Data Load Format`: `Fragment Table`
(378,536 bytes - the single largest unexamined stream found this
session), `MIC Table`, `DDA Filter Parameter`, `Precursor Sort Filter
Parameter`, `Profile Load Parameter`.

**Confirmed real, not junk**: `Compound Table` contains embedded ASCII
strings `mnm5s2U` and `m3U, m5U, m1Y, m3Y` - standard nomenclature for
methylated/pseudouridine ribonucleoside modifications, which exactly
matches `MTBLS7425`'s actual study subject (16S/23S rRNA modification
analysis). This is strong evidence the stream holds genuine, specific
compound-identification results for this exact run, not a fixed
template or boilerplate. Byte layout otherwise looks like packed binary
records (mixed integers and floats by inspection) - no record boundary
or field layout identified yet.

**Why this might matter beyond curiosity**: if decoded, this would let
the reader expose LabSolutions' own quantitation results (compound
names, retention times, peak areas, calibration curve fits) alongside
the raw spectra - a genuinely different kind of value than anything
else in this project (which has so far only ever recovered raw
acquisition data, never a vendor's own processed results). Also
genuinely harder: multiple large tables, likely cross-referenced by
index/ID rather than self-contained, no obvious plaintext-XML entry
point the way `File Property` had.

## 4. `LSS Data Processing` / `LSS Batch Table Original` / `LSSExtStgObj Param` / `LSS BaselineCheck Parameters` - LC-side (non-MS) processing (not examined)

Detector-channel and baseline-correction parameters for the
conventional LC detector side (`Detector Channel Information`,
`BaselineCheck`, several `GPC...` streams - GPC = Gel Permeation
Chromatography, a technique unrelated to this project's mass-spec
focus but apparently supported by the same LabSolutions method
framework). Plausibly relevant to `docs/format/04`'s still-open `LSS
Raw Data`/`PDA 3D Raw Data` payload decode (issue #2) as configuration
context, but not cross-referenced against that work this session.

## 5. `Report Format` - print/report layout, likely low value (not examined in depth)

Structured as classic OLE object embedding: `Embedding NN/OlePres000`
(consistently 28 bytes each - looks like a fixed-size presentation
header) plus `Embedding NN/Contents` (varying size, up to 11,342 bytes
in one file). This shape (`OlePres000` + `Contents` pairs) is the
standard pattern for embedded OLE objects inside a compound document -
most likely print-report templates/layouts (chart placement, page
formatting) rather than analytical data. Lowest priority of everything
in this document; flagged only for completeness.

## 6. `Audit Trail`, `SystemCheckResult` - small, not examined

`Audit Trail/Audit Trail Property` is a tiny (80-byte) stream, plausibly
a compliance/21-CFR-Part-11-style access-log entry given the name -
not decoded. `SystemCheckResult` contains a full nested copy of
`GUMM_Information` plus a `SystemCheckResult` stream (13KB) - not
examined; the nested `GUMM_Information` duplicate is itself notable
(same config data stored twice under different top-level paths) but not
investigated for whether the two copies actually differ.

## Constraints

Clean-room only, per `CONTRIBUTING.md`. Everything above comes from
directly reading corpus bytes with `olefile`; no LabSolutions software
or vendor documentation was consulted.

## Suggested priority for whoever picks this up

1. **`File Property`/`Method File Property` extraction** (section 1) -
   already decodable, smallest effort, real `RunMetadata` value
   (operator, LabSolutions version) plus a second independent
   corroboration of the #9 timestamp technique.
2. **`Mass Raw Data` single-quad variant** - see `docs/format/07`, not
   this document, but flagged here too since it's a more urgent gap
   than anything below (files currently can't open at all).
3. **`Mass Data Processing` compound-ID/quantitation tables** (section
   3) - highest potential payoff (genuinely new capability, not just
   metadata), but also the most speculative in effort required, since
   no plaintext entry point was found and record layout is unknown.
4. **`GUMM_Information` method/config** (section 2) - large, real,
   multi-layer-encoded; valuable context for other decode work
   (especially section 3 and the still-open PDA/LSS payload, #2) even
   before it's fully decoded itself.
5. Everything else (`LSS Data Processing`, `Report Format`, `Audit
   Trail`, `SystemCheckResult`) - lower expected value, not prioritized
   this pass.
