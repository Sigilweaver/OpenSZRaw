# 04. LC-MS (.lcd) Chromatogram and PDA Structures

**Status**: PARTIAL

The `.lcd` files use a different encoding scheme for chromatograms (`LSS Raw Data`) and Photo-Diode Array (PDA) UV data (`PDA 3D Raw Data`) than the primary MS raw data streams.

**Corpus note (this session)**: `LSS Raw Data/Chromatogram Ch1`..`Ch16` -
the literal path named in this document's title and in
Sigilweaver/OpenSZRaw#2 - is present in every locally available corpus
file's directory listing but is **0 bytes (empty) in all of them**.
There is no real `LSS Raw Data` chromatogram payload to analyze in this
corpus. All of this document's segment-level findings (both the prior
session's and this session's) come from `PDA 3D Raw Data/3D Raw Data`,
which is real and populated in every file that has a PDA detector
(`MTBLS432/*.lcd`, `PXD025121/*.lcd`, `MSV000084197/20190607_NM16.lcd`).
See "LC Raw Data - a different, unrelated chromatogram stream" near the
end of this document for the one real chromatogram-shaped stream found
locally, at a different path than the issue names.

## Segment Header

Chromatogram and PDA streams are divided into chunks or **segments**. Every segment begins with a **24-byte header**.

**Header Layout:**
Assuming a 6-element array of 32-bit (u32 LE) integers:
- **u32[0]**: Magic Number (`17234` which corresponds to the ASCII string `RC\x00\x00`).
- **u32[1]**: Unknown / Version (e.g., 1).
- **u32[2]**: **Corrected this session - a per-stream constant, not a per-segment point count** (see below).
- **u32[3]**: Segment Block Size in bytes, inclusive of this 24-byte header (e.g., 353, 562).
- **u32[4]**: Zero padding.
- **u32[5]**: Zero padding.

By reading `u32[3]`, a parser can successfully jump to the next segment header (the next `RC\x00\x00` signature).

### Correction: `u32[2]` is the PDA channel's wavelength count, not a per-segment data-point count

The previous version of this doc described `u32[2]` as "likely the
number of data points or a related parameter." Checked properly this
session against every segment in three different files' `PDA 3D Raw
Data/3D Raw Data` streams: **`u32[2]` never changes within a stream** -
`MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` has exactly one
distinct value (`68`) across all 2674 segments (and the same holds for
every other `MTBLS432` file checked: `..._male_1_63...lcd`,
`..._KO_CC_male_10_26...lcd`), `PXD025121/1.lcd` and `PXD025121/10.lcd`
and `/11.lcd` have exactly one value (`327`) across 6189-6190 segments
each, and `MSV000084197/20190607_NM16.lcd` has exactly one value (`321`)
across all 3502 segments. A genuine per-segment point count could not
behave this way (segment byte sizes in the same files vary continuously
- `353` up to `637`+ bytes - which would be impossible if the encoded
point count were truly fixed and the encoding were anything but strict
1:1 fixed-width, which "Segment Payload" below already rules out).

`u32[2]` instead matches the leading `u32` of that file's own
`PDA 3D Raw Data/Wavelength Table` stream exactly (`68` in the MTBLS432
files, `327` in the `PXD025121` files) - i.e. `u32[2]` is the **number of
wavelengths monitored by the PDA detector for this acquisition**, a
fixed per-run acquisition parameter, not a per-segment sample count. This
is a within-corpus cross-check (one Shimadzu-format stream's declared
count against another's, both independently parsed with `olefile`, no
vendor tooling involved), not a guess. Each `RC\x00\x00` segment is
therefore very likely one time-point's UV/PDA spectrum (intensity across
all monitored wavelengths) - conceptually the PDA analogue of one TTFL MS
scan - which is consistent with segment byte size growing when the
underlying spectrum has more/larger nonzero values (an elution peak) and
shrinking back down during a flat baseline, exactly as `docs/format/03`
describes for MS profile scans.

## Delta Encoding Payload

Following the 24-byte header, the segment payload contains the actual intensity or absorbance data.
- Because the data payload size is frequently an odd number of bytes (e.g., a 353-byte segment has 329 data bytes), it cannot be a simple array of 32-bit or 16-bit integers.
- This strongly indicates a **bit-packed or variable-length delta encoding** scheme.
- *Failed Hypotheses*: 
  - Standard unsigned LEB128 (7-bit continuation) was tested against a segment but yielded 426 decoded values rather than the 321 points explicitly declared in the segment header (`u32[2]`).
  - Interpreting the raw bits revealed frequent patterns of `0x3F`, suggesting possible 32-bit floats. Interpreting the bytes as **PDP-endian floats** (Middle Endian `3412`, e.g., swapping 16-bit words) actually yields perfectly valid float values (e.g., `0.914`, `1.782`). However, the mathematically odd byte sizes of the segments mean the payload cannot exclusively be an array of PDP-endian floats.
- The exact bit-masking or delta-compression logic remains an open reverse-engineering problem.

## Confirmed payload envelope: an 8-byte (or 4-byte) length-checked wrapper around the segment body

This session found and exhaustively verified (100% of segments, every
locally available file checked) a byte-exact structural fact about the
payload that sits *around* the still-undecoded per-point values: reading
the payload's first 2 bytes and last 2 bytes as `u16` little-endian
integers reveals a fixed, deterministic relationship to the payload
length, in one of two mutually exclusive forms depending on the file:

- **"Split" form** (`PXD025121/*.lcd`, `MSV000084197/20190607_NM16.lcd`
  - wavelength count 327 or 321): let `A` = the payload's first `u16`
  and `tail` = the payload's last `u16`. Then **`A + tail ==
  len(payload) - 8`, exactly, on every segment** - `6190/6190` segments
  in `PXD025121/1.lcd`, `6189/6189` in `/10.lcd`, `6190/6190` in `/11.lcd`,
  `3502/3502` in `20190607_NM16.lcd`. This means the payload is a fixed
  4-byte header (`A` plus 2 more bytes) and fixed 4-byte footer (2 more
  bytes plus `tail`), wrapping a body that appears to declare itself as
  two length-tagged regions of `A` bytes and `tail` bytes respectively
  (`A` typically 250-500+ bytes and growing with segment complexity;
  `tail` typically 65-125 bytes and comparatively stable). Checked
  whether the two regions differ in byte-value "vocabulary" (which would
  support a theory like "coarse array then exception list"): they do
  not - both regions are dominated by the same leader bytes (`0x20`,
  `0x3f`, see below) in similar proportions on both sides of the `A`
  boundary, so the functional purpose of the two-region split is **not
  resolved** by this observation alone.
- **"Symmetric" form** (`MTBLS432/*.lcd` - wavelength count 68): the
  payload's first `u16` and last `u16` are **equal to each other and
  both equal `len(payload) - 4`, exactly, on every segment** -
  `2674/2674` segments in each of three `MTBLS432` files checked
  (`..._12_65...lcd`, `..._1_63...lcd`, `..._KO_CC_male_10_26...lcd`).
  This reads as a single undivided body (`len(payload) - 4` bytes)
  with its length redundantly stamped at both ends, rather than a
  two-region split.

Both forms were checked against every segment (not a sample) in every
locally available file exhibiting each wavelength count, with zero
exceptions. It is not resolved from the corpus on hand whether the
"split" vs. "symmetric" form is selected by the wavelength-count
magnitude specifically or by an independent instrument/firmware
property - every file sharing one form in this corpus also happens to
share its wavelength count, so the two variables are confounded here.
The 24-byte segment header's own `u32[1]` ("version") field is `1` in
every file checked regardless of which payload form is used, so it does
not distinguish them either.

Scripts: `re/src/analysis/pda_varint_bruteforce.py` (`iter_segments`,
reused by the scripts below), ad hoc verification one-liners in this
session's transcript (not separately saved as scripts, since they were
short `python -c` checks against `iter_segments`).

## Cross-check against the TTFL MS Raw Data RLE scheme (negative result)

`TTFL Raw Data/MS Raw Data` (the IT-TOF MS payload, see `docs/format/03`)
turned out this session to use a run-length scheme: a `u16` marker word
`0x8000 | run_length` (terminator `0x8000` when `run_length==0`),
followed by a `u16` skip word and `run_length` raw `u16` values,
confirmed byte-exact (zero leftover) across 109,336 real MS scans.

Tried applying the *exact same* decoder directly to PDA segment payloads
(`re/src/analysis/pda_rle_test.py`, tested against
`MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`'s `PDA 3D Raw
Data/3D Raw Data` stream, segments after the 24-byte `RC\x00\x00`
header): **it does not decode cleanly**. The first several segments in
that stream are ~72-byte payloads that are almost entirely zero bytes
(just a leading `u16` matching `u32[2]`'s point count, e.g. `44 00` =
68, then zero padding - plausibly a genuinely blank/baseline region
early in the run), and the later, clearly-populated segments (dense,
high-entropy byte patterns, no long zero runs at all) don't parse as
TTFL-style marker words at all (`ok=False`, full leftover every time).
So the PDA payload is evidently a **different** encoding from the MS RLE
scheme, not a shared cross-stream-type compression as speculated -
whatever it is, it looks denser/less sparse than the MS profile data,
consistent with the earlier PDP-endian-float observation (UV absorbance
across many wavelengths per timepoint is unlikely to have the long
sparse zero-runs that MS profile spectra have, so a sparse RLE scheme
would be a poor fit for PDA data anyway, in hindsight). Not pursued
further this session; still open.

## This session's additional ruled-out hypotheses for the per-point value encoding

Building on the prior session's ruled-out LEB128 and PDP-endian-float
attempts (above), and on the "split"/"symmetric" envelope structure just
established (so these were tested against the body region *between* the
4-byte or 2-byte header and footer, not the raw payload), the following
were tried against `MSV000084197/20190607_NM16.lcd`'s `PDA 3D Raw
Data/3D Raw Data` stream (3502 segments) and did not produce a clean,
zero-leftover, exact-point-count decode across the stream:

- **Exact-value single-byte escape** (a specific byte value triggers a
  2-byte token, every other byte is a 1-byte literal): brute-forced all
  256 possible escape byte values, and all pairs of values drawn from the
  best-performing singles. Best single value achieved only 4/3502 clean
  segments (the near-blank baseline segments, which decode "clean" under
  almost any escape choice simply because they contain no bytes matching
  it); the best pair reached at most 8/3502. The specific pair `{0x20,
  0x3f}` - motivated by the prior session's PDP-endian-float observation,
  since both bytes are exactly the leading byte of an IEEE-754 float in
  `[0.5, 2.0)` - looked promising on the first few busy segments (321
  vs. a declared 321, off by only 2-3), but the miscount **grows
  unboundedly** on busier segments (up to +214 on the busiest segments
  checked), so it is not the real rule; it just happens to be close on
  low-complexity segments.
- **LEB128-style continuation bit at other bit positions**: grid-searched
  all 8 bit positions (`0x01` through `0x80`) x both polarities (bit set
  vs. bit clear meaning "more bytes follow") x header/footer skip 0-11
  bytes each (2304 combinations). Best combination (`cont_bit=0x10`,
  bit-clear-means-continue, header=0, footer=1) reached 88/3502 clean
  segments - a weak signal, well short of a real decode.
- **UTF-8-style leading-byte width prefix** (count the leading `1`-bits
  in a byte, from the MSB, to determine total token width, as in real
  UTF-8): 4/3502 clean, no better than the trivial baseline-only result.
- **Magnitude-threshold split** (any byte `>= THRESHOLD` starts a wide
  token, everything else is a 1-byte literal; swept `THRESHOLD` from
  `0x00` to `0xff` and wide-token width from 2 to 4 bytes): the best
  configuration, `THRESHOLD=0x1f`/`0x20` with a 2-byte wide token, reached
  **218/3502 (~6%) clean segments** - the strongest partial signal found
  this session, and notably centered on the same `0x20`/`0x3f` byte
  values the prior session flagged from the PDP-endian-float angle. It
  still diverges on the majority of segments as they get busier, so a
  single fixed magnitude threshold is not the complete rule; whatever
  determines token width more precisely was not found this session.
- **Region-boundary vocabulary check** (does the `A`-byte region and the
  `tail`-byte region of the "split" form use visibly different byte
  encodings, e.g. one is a coarse array, the other an exception list?):
  checked leader-byte frequency histograms on both sides of the boundary
  in several segments - no detectable qualitative difference; both
  regions are dominated by the same `0x20`/`0x3f`-ish leader bytes in
  similar proportions. This rules out the "two different token formats"
  version of the two-region theory, though it doesn't explain what the
  two declared-length regions actually are.

None of these fully explain the payload. The strongest remaining lead is
the magnitude-threshold-near-`0x20` partial fit combined with the
PDP-endian-float observation from the prior session: something in the
neighborhood of "most values are represented compactly when their
magnitude keeps them under roughly 1.0-2.0 in whatever fixed-point or
float-like unit this is, with a wider representation kicking in above
that" is plausible, but the precise per-token width rule (which byte, or
combination of bytes, actually decides 1-byte vs. 2-byte vs. wider) was
not found. A promising next avenue for a future session: since `u32[2]`
is now known to be a wavelength count and each segment is very likely
one full spectrum, cross-referencing decoded region lengths against the
actual `PDA 3D Raw Data/Wavelength Table` contents (not just its leading
count field, which is all this session used) might reveal whether the
`A`/`tail` split lines up with a real wavelength sub-range boundary
(e.g. a monitored subset vs. the full range) rather than being an
encoding-internal detail.

Scripts written this session (all under `re/src/analysis/`, gitignored
per this repo's existing convention for the `re/` sandbox - see
`CONTRIBUTING.md`): `pda_segment_survey.py`, `pda_hexdump.py`,
`pda_escape_test.py`, `pda_escape_bruteforce.py`, `pda_escape3_search.py`,
`pda_varint_bruteforce.py`, `pda_varint_tune.py`, `pda_varint_tune2.py`,
`pda_threshold_test.py`, `pda_threshold_test2.py`, `pda_threshold_test3.py`,
`pda_utf8style_test.py`.

## LC Raw Data - a different, unrelated chromatogram stream

While looking for real `LSS Raw Data` chromatogram content (all empty in
this corpus, see the note at the top of this document) a different,
populated chromatogram-shaped stream was found at a different top-level
storage: `LC Raw Data/Chromatogram Ch5` and `Ch6` in the `PXD020792/*.LCD`
files (e.g. `UY01-03-01p95.LCD`, 14540 and 7422 bytes respectively).
These are worth flagging even though they're outside this issue's named
scope (`PDA 3D Raw Data` / `LSS Raw Data`):

- They do begin with the same 24-byte `RC\x00\x00` segment header
  (`u32[2]` = 7200 for both channels in this file, plausibly a time-point
  count for this LC-only, non-PDA acquisition), but `u32[3]` (block size)
  spans the **entire rest of the stream** - i.e. the whole stream is one
  giant segment, not many small ones like the PDA case.
- Byte content looks qualitatively different from the PDA payload: `Ch5`
  (a "quiet"/flat channel) is dominated by one repeating 2-byte pattern
  (`82 00` over and over, i.e. a constant `u16` value) for almost its
  entire length; `Ch6` (a "busier" channel) has small clustered
  single-byte values mostly in `0x00`-`0x20`, with no obvious `0x3f`/`0x20`
  escape-pair pattern like the PDA stream.
- This was not decoded and no further hypotheses were tried against it
  this session - it is a different stream, likely a different detector
  type (conventional UV/RID rather than PDA), and pursuing it was out of
  scope for Sigilweaver/OpenSZRaw#2. Flagged here so a future session
  investigating `LSS Raw Data` or general LC-channel chromatogram support
  doesn't have to rediscover that the populated data lives under `LC Raw
  Data`, not `LSS Raw Data`, in files that have it.
