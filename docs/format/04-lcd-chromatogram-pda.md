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

## Factsheet (quick reference - see the dated sessions below for full evidence)

This document has grown across many investigation passes; this section
is a scannable summary, not a substitute for the detailed sections below
(each claim here links to the section that established it).

**Confirmed:**
- Every segment starts with a 24-byte header, magic `RC\x00\x00`
  (`u32[0]==17234`); `u32[3]` is the segment's total block size
  (walk-to-next-segment field). See "Segment Header".
- `u32[2]` is a **per-stream constant** equal to the PDA detector's
  monitored wavelength count (`npts`), matching the `Wavelength Table`
  stream's own leading count exactly - not a per-segment point count.
  See "Correction: `u32[2]`...".
- The payload has a 100%-verified length-checked envelope in one of two
  mutually exclusive forms ("split": `A + tail == len(payload) - 8`;
  "symmetric": both ends equal `len(payload) - 4`), confirmed on every
  segment of every locally available file. See "Confirmed payload
  envelope...".
- **Flat/real is a hard, instantaneous cliff**: body length is either
  exactly `npts` bytes (baseline, 1 byte/value, all-zero) or jumps to at
  least 1.8x `npts` - nothing in between, zero exceptions across ~32,000
  segments checked, both envelope forms. No gradual escalation, no
  early-warning signal in the segments just before the jump, and the
  transition point itself (segment index 11 or 12, varies by file) has
  no shared marker byte across files. See "cross-file pass" and
  "transition-segment comparison".
- Real-mode body length is **tightly centered near `3 * npts` bytes**
  for the symmetric (`MTBLS432`) form - close to, but not exactly, a
  uniform 3-bytes-per-value scheme. See "width-table retry on bimodal
  data". The split (`PXD025121`/`MSV000084197`) form centers lower, near
  `1.88 * npts` (range `[1.65, 2.08] * npts`, 100% of one file's
  real-mode segments checked) - a distinct, form-specific constant, not
  the same `3 * npts` figure. See the 2026-07-20 session.
- **`CheckSum` stream, 2 of 4 varying fields fully identified**: offset
  56 (`u32` LE) is the exact byte size of `PDA 3D Raw Data/3D Raw Data`;
  offset 88 (`u32` LE) is the exact byte size of `PDA 3D Raw Data/Max
  Plot` - both zero-mismatch across 77 files. This corrects an earlier
  reading of offset 58 as a "flat vs. real" flag byte (it's actually
  part of the offset-56 size field). Offsets 48 and 80 are confirmed
  genuinely content-dependent but remain unidentified after a sweep of
  19 CRC-16 polynomials, Fletcher/Adler/plain-sum checksums, and 5
  candidate byte ranges. See the 2026-07-20 session.
- **The "split" form's two regions are an exact 256-channel/remainder
  wavelength split**: region `A` always holds the first 256 wavelength
  channels, region `tail` holds the remaining `npts - 256`, confirmed
  with zero exceptions across every flat segment in all 4 locally
  available split-form files (2 distinct `npts` values). This also
  fully explains why "split" vs. "symmetric" envelope form correlates
  with wavelength count: every corpus file with `npts <= 256` uses
  symmetric, every file with `npts > 256` uses split. See 2026-07-20
  session 3.
- Width selection is very likely **driven by actual per-wavelength
  signal magnitude**, not any of: an external per-run lookup table (no
  config/instrument stream correlates with modal length - "external-
  table hunt"), a fixed positional/bitmap table (least-squares fits
  never converge to clean integer widths, even on the cleaner bimodal
  data - "width-table retry"), or an explicit flag/marker byte
  ("transition-segment comparison").
- There **is** genuine, statistically-verified (randomized-control-
  checked) temporal correlation between consecutive segments - both via
  whole-stream compression ratio and position-aligned byte deltas
  ("temporal/delta redundancy"), and independently reproduced via a
  from-scratch joint-decode DP ("joint temporal+magnitude decoder") -
  but no working decoder has been built from it yet; the joint-DP's
  scoring function is provably not selective enough (width agreement
  isn't monotonic with solution cost). **Caveat added 2026-07-20 session
  5**: the joint-DP finding's effect size was originally based on a
  single true-pair-vs-random-pair anecdote (`1.8x` cost ratio); a
  20-pair re-test on region `tail` reproduces the *direction* but at a
  much smaller, less certain effect size (`~1.13x` mean ratio, true
  pair cheaper in only 12/20 individual pairs) - cite this as a weak,
  aggregate-level signal, not a strong or reliable pair-by-pair one.
- **The region-A-isolated per-byte-position entropy analysis (2026-07-20
  session 5) reproduces the earlier whole-stream entropy session's
  numbers almost exactly** (marginal/conditional entropy within 0.01
  bits) and finds no periodic marker at either 2- or 3-byte spacing
  across region `A`'s full true 256-channel span - confirming the
  earlier entropy work was already correctly scoped and the
  low-entropy-near-`0x40`-at-residue-1 pattern really is a low-variance
  early-wavelength artifact, not a token-boundary marker.
- **Methodological: a physical-plausibility (temporal-smoothness) check
  must also test each channel's mode fraction, not just mean
  relative step.** A decode that is a frozen, repeated value most of
  the time with rare large jumps trivially minimizes mean relative step
  while being the opposite of real chromatography; 2026-07-20 session 4
  found this the hard way (a 67.8%-clean, apparently-smooth single-file
  `MTBLS432` result turned out to be up to 96% mode-dominated per
  channel) and revised the check accordingly.
- **Methodological, extending the mode-fraction lesson: even without
  outright mode-domination, low value diversity alone mechanically
  produces a good smoothness score, independent of decode correctness.**
  2026-07-20 session 6 found a `0.840` correlation (68 channels, one
  file) between a channel's smoothness score and how many distinct
  values it takes - genuinely variable (more analytically real)
  channels score *worse*, and channels that barely vary score well
  regardless of whether the decode is right. A physical-plausibility
  check should be read skeptically for low-diversity channels
  specifically, not just mode-dominated ones.
- **The flat-to-real transition has no visible byte-level structure at
  all when the transition segment's actual bytes are read by eye**
  (2026-07-20 session 6, two files checked): no leading or trailing
  quiet region, no length-prefix-looking field, zero bytes scattered
  not clustered - direct, by-eye confirmation of the already-established
  "hard, instantaneous cliff" aggregate-statistics finding.

**Ruled out** (see "This session's additional ruled-out hypotheses" and
"further ruled-out hypotheses" for full detail): standard unsigned
LEB128; pure PDP-endian-float array; single- and paired-escape-byte
schemes (full 256x256 brute force); continuation-bit varints at all 8
bit positions x both polarities x 0-11 byte header/footer skip, both
whole-body and region-A-isolated; UTF-8-style leading-byte width
prefixes; magnitude-threshold split (whole-body and region-A-isolated);
leading bitmap/nibble width-selector tables (1/2/4-bit codes, both
pooled-file and per-file least-squares); the TTFL MS RLE scheme applied
directly; an external per-run gain/width table stored elsewhere in the
file; `Max Plot` as an independently-decodable crib; a fixed-width fp16
(binary16) array (body length essentially never equals `2 * npts`); the
19-polynomial CRC-16 sweep (plus Fletcher/Adler/plain-sum, and an
Internet-checksum ones'-complement variant) against `CheckSum` offsets
48 and 80, and those two fields as any of several plain counts/derived
sizes (see 2026-07-20 sessions 1 and 2); the block-floating-point/
adaptive-scale hypothesis family in four concrete forms - explicit
header-as-threshold, classic fixed-width-per-segment, marker-bit escape
with an algebraically-derived baseline width (a real single-file signal
that fails cross-file generalization), and a leading-bitmap popcount
validator (see 2026-07-20 session 2); the region-`tail`-isolated
marker-bit escape rule, despite passing two randomized-control checks
(uniform-random bytes and same-multiset-shuffled bytes), traced via a
physical-plausibility (temporal-smoothness) check to a compensating-
error artifact affecting all but one of 65 channels, not a real
per-channel decode (see 2026-07-20 session 3); the corrected-target-
count magnitude-threshold and continuation-bit sweeps against region
`A` (target 256) and region `tail` (target `npts - 256`) alone - region
`tail`'s sharp-cliff threshold=32 hit reproduces session 3's exact
channel-0/1 artifact under a different rule, and a fresh `MTBLS432`
sweep's single-file 67.8%-clean/"66-of-68-channels-smooth" result
failed cross-file generalization and was shown to be a smoothness-
metric artifact (mode-dominated decoded values, not real drift) rather
than a genuine decode (see 2026-07-20 session 4); a fixed
bit-identical-value penalty term added to the joint temporal+magnitude
DP's cost function, intended to discourage mode-collapsed solutions -
did not improve (and on a 20-pair sample, worsened) the correlation
between solution cost and width agreement, so does not fix the
DP's previously-diagnosed selectivity weakness (see 2026-07-20 session
5); mod-2 and (beyond the already-explained early-channel effect)
mod-3 periodicity in region `A`'s per-byte-position entropy (see
2026-07-20 session 5); the leading-byte-of-a-3-byte-token hypothesis
for `MTBLS432`, checked via exact-`3*npts`-length segments across three
files - the channels that looked smooth by eye in each case turned out
to have near-zero value diversity (as low as 2 distinct values across a
6-segment run), with a `0.49`-`0.84` smoothness-vs-diversity correlation
confirming the pattern is a metric artifact, not a decode (see
2026-07-20 session 6).

**Genuinely open:**
- The exact per-value token grammar (width-selection rule and numeric
  interpretation) - the core unsolved problem.
- Whether `LSS Raw Data` (the issue's literally-named path) uses the
  *same* encoding as `PDA 3D Raw Data` at all - untested, since every
  locally available file has it empty. See "Further avenues" at the end
  of this document.
- The `LC Raw Data/Chromatogram Ch5`/`Ch6` stream (real, populated, but
  a different/simpler-looking encoding, out of this issue's named scope)
  - not decoded, not attempted beyond the initial characterization. See
  "LC Raw Data..." near the end of this document.
- `CheckSum` offsets 48 and 80 (confirmed content-dependent, not
  identified as any standard CRC/checksum algorithm nor as a plain
  count/derived size - see 2026-07-20 sessions 1 and 2).
- Whether fp16 (binary16) numeric interpretation or spectral-domain
  (wavelength-to-wavelength) delta coding is the right *value*
  interpretation for a token, once a token-boundary rule is found -
  both remain plausible as validators, neither has been testable yet
  since no token-boundary rule has been found to validate. See
  2026-07-20 session 1.
- Whether a more elaborate adaptive-scale mechanism than the four forms
  ruled out in 2026-07-20 session 2 exists (e.g. a per-segment gain
  header using a non-linear transform, or region-local rather than
  whole-body marker placement) - untested, but the simplest and most
  natural versions of the idea do not fit.

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

## 2026-07-19 session: further ruled-out hypotheses (per-region parsing, width-table, byte volatility)

Building on the "split"/"symmetric" envelope structure and the prior
sessions' ruled-out per-point encodings (above), this session tried
three more approaches against `MSV000084197/20190607_NM16.lcd`'s
`PDA 3D Raw Data/3D Raw Data` stream (3502 segments, wavelength count
321, "split" envelope form). None produced a clean decode; all are
ruled out as stated below.

- **Per-region parsing instead of whole-body parsing**: every prior
  varint/threshold sweep (LEB128-style continuation bit, magnitude
  threshold) had been run across the *entire* body (header/footer
  stripped, but region A and region tail concatenated and walked as one
  continuous token stream). This session re-ran both sweeps with region
  A parsed **in isolation**, targeting exactly `npts` (321) decoded
  values from region A alone rather than from the combined body. Neither
  sweep improved on the whole-body numbers: the continuation-bit sweep's
  best combination (`cont_bit=0x40`, bit-set-means-continue) reached only
  31/3502 clean segments, and the magnitude-threshold sweep's best
  (`threshold=0x33`/`0x34`, 2-byte wide token) reached 202/3502 - both
  weaker than or comparable to the whole-body results already documented
  above, not better. This rules out "region A alone holds all 321
  wavelength values under one of the previously-tried token schemes,
  and region tail is unrelated trailing data" as an explanation for why
  the whole-body sweeps fell short.
- **Leading bitmap/nibble width-selector table** (a new hypothesis, not
  previously tried): the idea that each region starts with a fixed-size
  packed table of small per-wavelength "width codes" (1, 2, or 4 bits
  each, covering all 321 wavelengths - table sizes 41, 81, and 161 bytes
  respectively), followed by a value area whose per-value byte width is
  looked up from that table, would explain variable segment size without
  needing an inline continuation bit. This was tested structurally
  rather than positionally: for each candidate bit-width, per-code value
  counts were tallied per segment (300 segments checked) and a
  least-squares fit was solved across all of them simultaneously for a
  single global set of per-code byte-widths, since a real width table
  must use the *same* code-to-width mapping in every segment. No
  bit-width produced anything close to integer, low-error widths - mean
  absolute error per segment ranged from 6.8 bytes (4-bit codes) up to
  21.8 bytes (1-bit/bitmap codes), with worst-case errors over 100 bytes
  - which rules out a fixed leading width-selector table (at these
  common bit-widths) for both region A alone and the combined body.
- **Byte-level volatility between same-length segments** (a
  cross-check, not a decode attempt): if segment payload length alone
  determined byte layout (e.g. a fixed-position array whose element
  widths only depend on total length), two segments with identical
  payload length occurring close together in time - where the underlying
  UV spectrum is physically similar - should look byte-similar. Checked
  all 163 adjacent-segment pairs in the stream that happen to share the
  same payload length: in the busy (non-baseline) part of the run, an
  average of ~80% of bytes differ between such pairs (e.g. 482-531
  bytes differing out of ~594-613-byte payloads). This confirms the
  encoding is genuinely variable-width per value (matching byte length
  is coincidental, not indicative of matching structure), not a
  disguised fixed-width array.

Scripts written this session (all under `re/src/analysis/`, gitignored
per this repo's convention): ad hoc, run via `python3 -c`/heredoc and
not separately saved, since each was a short, disposable brute-force or
statistical check built directly on `iter_segments` from
`pda_varint_bruteforce.py`.

No fine-grained entropy analysis of the PDA payload itself had been done
before this session - `re/src/analysis/stream_inspector.py`'s Shannon
entropy tooling was used only in the very early, whole-stream
type-identification phase (`re/ROADMAP.md`'s "Hex-dump and
entropy-analyze each distinct stream type" checkbox), not applied at the
per-segment or per-byte-position level against this specific payload.
That is a promising unexplored angle for a future pass: per-byte-position
entropy across length-aligned segments, or conditional/first-order-Markov
entropy, could reveal where within a token a header/marker byte lives
even without knowing the full token grammar up front.

## 2026-07-19 session (continued): per-byte-position entropy analysis and a body-length-variability finding

Picking up the "promising unexplored angle" noted just above, this pass
ran the fine-grained entropy analysis that had never been done on this
payload: per-byte-position entropy (aligned to region start, absolute
position 0-39) across all 3502 segments of
`MSV000084197/20190607_NM16.lcd`'s region A, plus conditional entropy
and a compression-ratio check. It also cross-checked one finding against
`MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` (the "symmetric"
envelope form, 68 wavelengths - structurally different from the "split"
form used for the rest of this section). Net result: one real, useful,
previously-undocumented structural fact was found (body length is
per-value variable-width, ranging from exactly `npts` bytes up to
roughly `3 * npts` bytes), but the entropy-by-absolute-position pattern
that looked at first like a clean encoding-level marker turned out, on
closer inspection, to be something else - a corrective finding worth
recording so a future session doesn't chase it again.

- **Initial (misleading) signal**: per-absolute-position entropy over
  the first 40 bytes of region A, aggregated across all 3502 segments,
  shows a striking repeating pattern - position 1 (mod 3) has entropy
  ~2.2-2.9 bits with one byte value (`0x40`) accounting for 69-75% of
  all occurrences, while positions 0 and 2 (mod 3) sit at 6-7.9 bits
  (much closer to random). Taken at face value this looks exactly like
  a 3-byte token with a near-constant marker/tag byte in the middle
  position.
- **Why it's not that (the corrective part)**: aggregating by byte
  *value histogram* per residue class across the **entire** region
  (not just the first 40 bytes) shows all three residue classes (`i mod
  3 == 0, 1, 2`) converge to essentially the *same* distribution,
  dominated by the same handful of bytes (`0x3f`, `0x20`, `0x5f`,
  `0x3e`, ...) in similar proportions (order-of-magnitude ~88,000-98,000
  occurrences each, out of ~565,000 samples per class). A true per-token
  marker byte would stay skewed at its residue class everywhere in the
  region, not converge to the same distribution as the other two
  classes once you stop restricting to the first 40 bytes. The real
  explanation is positional, not structural: the first ~40 bytes of
  every segment correspond to the *lowest-index wavelengths* in the
  scan (region A is wavelength-ordered, consistent with the
  already-established `Wavelength Table` cross-reference above), and
  those low-index wavelengths apparently carry low-variance,
  near-constant readings across time in this corpus file (plausibly a
  baseline/edge-of-range wavelength with little real signal) - which
  produces low entropy *at those specific early byte offsets* for a
  mundane data reason, not because of any fixed-width tokenization or
  marker-byte grammar. **Any future session should not read the
  position-1-mod-3/`0x40` pattern as evidence of a 3-byte token
  grammar** - this pass looked hard for that and it isn't there.
- **A real, novel structural finding: body length is consistent with
  1-3 variable bytes per value, not a fixed width.** Checked
  `MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` (symmetric
  form, `npts` = 68 wavelengths, 2674 segments): body length (`len(payload)
  - 4`) ranges from exactly **68 bytes at the minimum** (i.e. exactly
  `npts`, one byte per value) up to **209 bytes at the maximum**
  (close to `3 * npts` = 204), with the single most common length being
  **194 bytes in 1767/2674 segments (66%)** - consistent with most
  segments using a mix of 2- and 3-byte tokens (e.g. 194 bytes across
  68 values is exactly what you'd get from 10 values at 2 bytes and 58
  values at 3 bytes, though this specific split is only one
  arithmetically-consistent example, not independently confirmed).
  Only 311/2674 segments (11.6%) have a body length divisible by 3 at
  all, ruling out a clean fixed-3-byte grid outright, and the exact
  `npts`-byte minimum at baseline is the same "1 byte per value when
  values are near zero" pattern already documented for the "split" form
  above - this confirms the same variable-width-per-value character
  holds across both envelope forms, not just the one this document has
  focused on. The precise 1-vs-2-vs-3-byte selection rule remains
  unresolved (this is consistent with, but does not solve, the
  magnitude-threshold partial signal already documented above).
- **Conditional entropy confirms genuine local structure, not noise**:
  `H(byte)` vs `H(next_byte | current_byte)` for `20190607_NM16.lcd`'s
  region A is 6.279 bits vs 5.408 bits (a 0.87-bit, ~14% reduction);
  for region tail, 5.783 vs 4.697 bits (a 1.09-bit, ~19% reduction).
  Both regions are meaningfully more predictable one-byte-ahead than
  their marginal entropy alone would suggest, i.e. there is real
  recoverable local structure in both regions (consistent with a
  float-like or delta-like internal grammar), even though the exact
  grammar remains undecoded.
- **Compression ratio - data is genuinely dense, but not fully random**:
  each region-A payload compresses individually to ~93% of its original
  size with zlib level 9 (essentially incompressible alone, as expected
  for float-mantissa-like bytes), but concatenating all 3502 region-A
  payloads from one stream and compressing as a single blob reaches 78%
  - meaningfully better than any individual segment, indicating real
  cross-segment (i.e. cross-time) redundancy that no decode attempt so
  far has tried to exploit (every attempt to date, including this
  session's, has been per-segment/local only). A **temporal** approach
  (e.g. delta-coding against the previous segment's decoded values,
  once *a* per-segment decode exists to delta against) is a plausible
  next avenue distinct from anything tried so far.

Script: `re/src/analysis/pda_entropy.py` (new this session, not
gitignore-exempt - lives under the existing `re/` sandbox convention).
Reuses `iter_segments` from `pda_varint_bruteforce.py`.

## 2026-07-19 session (cross-file pass): a real, format-general structural finding, plus a methodological gotcha

Prompted by a specific ask to compare entropy and byte alignment *across
files* rather than within one representative file, this pass loaded up
to 15 `MTBLS432/*.lcd` files (symmetric form, 68 wavelengths) and 3
`PXD025121/*.lcd` files (split form, 327 wavelengths) side by side.
Still no full decode, but this surfaced one genuinely new, well-verified,
format-general fact, corrected one over-eager interpretation of it, and
flagged a real methodological trap for future cross-file entropy work.

- **Cross-file byte-identity check (the "line up across files" ask,
  literally)**: the first 11 segments of `PDA 3D Raw Data/3D Raw Data`
  are **byte-identical, including length**, across 10 different
  `MTBLS432` files (10 different mice/samples) - `440000...0044` and so
  on. This is a real cross-sample structural confirmation (not just a
  within-file baseline observation as before): the "all near-zero -> 1
  byte of `0x00` per wavelength" encoding is deterministic and
  sample-independent for a genuinely flat spectrum, and the run-start
  blank period is the same length (11 segments) regardless of sample.
  First divergence (in both length and content) is segment index 11.
- **A real, format-general structural finding: baseline and "real
  signal" encoding are two hard modes with no gradual escalation between
  them.** Checked body-length distributions in all 9 `MTBLS432` files
  that have any real (non-flat) PDA signal at all (see next bullet for
  the other 6), plus 3 `PXD025121` files: in every one of these 12
  files, body length takes the value `npts` (exactly 1 byte/value, the
  flat/baseline case) or jumps straight to at least **1.8x `npts`** -
  never anything in between. For `MTBLS432` (`npts`=68): flat is exactly
  68 bytes, the next length that ever occurs is 182-202 bytes depending
  on the file (a **114-134 byte gap**, i.e. body length 69-181/69-201 is
  simply never observed in any of the 9 files' 2673-2674 segments each).
  For `PXD025121` (`npts`=327, split form, body = `paylen - 8`): flat is
  exactly 327 bytes, the next length is 479-513 bytes (a **152-186 byte
  gap**) across all 3 files checked. This rules out a smoothly
  escalating variable-width scheme (e.g. "small values cost 1 byte,
  slightly bigger values cost a little more") in favor of a **hard mode
  switch**: either the whole spectrum is quantized to the cheap 1-byte
  representation, or essentially the whole spectrum switches to a much
  more expensive representation at once. This held with zero exceptions
  across both envelope forms and all 12 files checked, so it's read as a
  format-level fact, not a per-file quirk.
- **Correcting an over-eager reading of that cliff**: the first pass at
  this (single-file, `MTBLS432/..._12_65...lcd`) initially looked at the
  entropy jump at absolute position 68 within pooled non-baseline
  segments and read it as evidence for "a fixed 68-byte coarse
  1-byte-per-wavelength array, plus an appended variable extension" -
  i.e. that position 68 itself was a meaningful *boundary inside every
  segment*. Checking the raw length histogram directly (see above)
  shows this is wrong: there is no population of segments with body
  length in, say, 90-150 that would need a "coarse array + growing
  extension" - length is strictly bimodal (68, or 182+), so the
  apparent "boundary at position 68" was actually just the edge of the
  much shorter "flat" population dropping out of the pooled sample as
  position increases past 68, not a within-segment structural
  boundary. Recorded so a future session doesn't re-read that artifact
  as a coarse-array theory again.
- **The "real" mode's typical length is per-run, not a global format
  constant.** Across the 9 `MTBLS432` files with real signal, the
  smallest non-flat length varies 182-202 and the *modal* real-segment
  length varies file to file - one file's most common real length is
  194 bytes (1767/2674 segments), another's is 205 (724/2674), another's
  203 (445/2674), another's 199 (649/2674), etc. - with no length shared
  as "the" mode across files. Since all 9 files share the same
  instrument, method, and `npts`=68, a truly fixed per-wavelength-index
  byte-width table (e.g. "channel 5 is always 2 bytes, channel 6 is
  always 3 bytes, for every run on this instrument") would predict the
  same modal length everywhere. It doesn't, which rules that theory out;
  whatever selects each value's width in the "real" mode must depend on
  this specific acquisition's own dynamic range per channel, not a
  hardcoded per-channel constant.
- **Methodological gotcha, worth recording since it looked like a
  finding at first**: pooling per-byte-position histograms across
  *multiple files* at a fixed body length (e.g. "all segments from any
  file with body length exactly 194") produced a striking pattern at
  first glance - entropy 2-3 bits at nearly every position (versus
  ~5.5-7+ bits for whole-region entropy elsewhere in this doc) and one
  byte value accounting for ~50% of occurrences at almost every
  position. This is **not** a real per-position structural marker: two
  of the 15 pooled files (`..._12_65...lcd` with 1767 segments,
  `..._33_72...lcd` with 1091) supplied ~89% of the length-194 sample
  pool between them, so the "50% dominant byte" pattern mostly reflects
  those two files' own segment-to-segment redundancy, not a
  cross-file-general structural fact. Directly checked: pairwise
  byte-diffs between different length-194 segments *within* one file
  are 92-100% different (169-194 of 194 bytes), confirming the
  underlying per-timepoint values are genuinely varying, not near-
  duplicated - the pooled low-entropy signal was a sample-imbalance
  artifact of unequal per-file contribution to the pool, not evidence of
  a repeating template. Any future cross-file entropy pooling should
  weight or cap per-file contribution (or check contribution counts
  before trusting a pooled pattern), or this same false signal will
  likely reappear.
- **Corpus-triage side finding**: at least 6 of the first 15 `MTBLS432`
  files checked (`..._1_63`, `..._20_66`, `..._30_71`, `..._37_74`,
  `..._3_64`, `..._44_76`) have **zero real PDA signal for their entire
  run** - every one of their 2673-2674 segments is exactly 68 bytes
  (fully flat/baseline), not just the run-start blank period. These
  files are not useful for further payload-decode attempts against real
  data; future sessions should prioritize the confirmed real-signal
  files listed above (`..._12_65`, `..._34_73`, `..._26_68`, `..._27_69`,
  `..._28_70`, `..._33_72`, `..._10_26`, `..._13_27`, `..._21_29`).

None of this decodes the "real" mode's actual per-value encoding - that
remains open. But the hard-cliff finding is a solid, format-general
constraint any future decode theory now has to satisfy (a real theory
should predict a length distribution with this exact bimodal gap, not a
smooth ramp), and the corpus-triage list saves a future session from
re-discovering which files are worth analyzing.

Scripts: ad hoc, run via `python3 -c`/heredocs this pass and not
separately saved (each was a short, disposable cross-file query built
directly on `iter_segments` from `pda_varint_bruteforce.py`); a future
session wanting to repeat the cross-file length-histogram or
byte-identity checks can rebuild them quickly from the numbers cited
above.

## 2026-07-19 session (external-table hunt): ruled out - no external per-run width/gain table exists

The previous section found that within one `MTBLS432` file, the "real
mode" body length is nearly *constant* across that file's non-flat
segments, but the *modal* length varies from file to file (194, 205,
203, 199, 211, 193, 202, 185 bytes, for `npts`=68 across the 9
real-signal files). This raised a reframing: maybe per-wavelength byte
width isn't computed inline from each value's magnitude at all, but set
once per acquisition by a calibration/gain parameter stored *elsewhere*
in the file (a "look here first" table rather than an inline rule) -
which would need to be re-derived per run rather than searched for as a
single inline token grammar.

This is now **ruled out**, cleanly, by direct inspection: every stream
that plausibly could hold such a table turns out to be **byte-identical
across all 15 `MTBLS432` files checked** (all 9 real-signal files with
different modal lengths, plus the 6 fully-flat files), despite the
files having entirely different underlying chromatography/PDA data:

- `LSS Configuration/PDA Configuration` (192 bytes) - byte-identical
  across all 15 files.
- `PDA Instrument Parameters/Analog Output Parameter` (324 bytes) -
  byte-identical across all 15 files.
- `PDA Instrument Parameters/SPD-MXA Parameter` (104 bytes) -
  byte-identical across all 15 files.
- `PDA 3D Raw Data/Instrument Info` (800 bytes) - byte-identical across
  all 15 files.
- `PDA 3D Raw Data/Background Status` (404 bytes) - byte-identical
  across all 15 files.
- `PDA 3D Raw Data/Wavelength Table` (280 bytes) - byte-identical across
  all 15 files (checked all 280 bytes, not just the leading count -
  confirms the per-wavelength nm values documented above are also a
  fixed method parameter, not per-run).

Two streams do vary per file - `PDA 3D Raw Data/CheckSum` (112 bytes,
differs at byte offsets 48-49, 56-58, 80-81, 88-89) and `PDA 3D Raw
Data/Status` (404 bytes, differs at byte offsets 4, 20-21) - but neither
correlates with modal length. `CheckSum`'s values at these offsets look
exactly like what the stream's name says (checksums/counts derived from
each run's actual segment data, naturally different per run because the
data differs) rather than a pre-set gain table: plotted against modal
length (185, 193, 194, 194, 199, 202, 203, 205, 211 bytes across the 9
files, sorted), the values at every varying offset are non-monotonic
with no visible relationship (e.g. offset-48 `u16`: 39649, 24543,
30639, 56115, 63931, 51649, 5637, 3341, 5163 - no correlation with the
modal-length ordering). One incidental finding from this check: the
byte at `CheckSum` offset 58 is `0x09` in all 9 real-signal files and
`0x03` in all 6 flat files - a clean flat-vs-real flag - but this only
says whether real data exists at all, not anything about its encoding
width, so it doesn't help decode the payload itself.

**Correction (2026-07-20 session)**: offset 58 is not a dedicated flag
byte. It is the high 16 bits of a little-endian `u32` field starting at
offset 56, and that `u32` is the exact byte length of the
`PDA 3D Raw Data/3D Raw Data` stream itself (e.g. `0x0009_1A64` =
`596580`, the `12_65` file's exact stream size, decomposing into the
`0x1A64`/`6756` seen at offset 56-57 and the `0x09` seen at offset 58).
The apparent "flat vs. real flag" reading of the lone offset-58 byte was
a coincidence of magnitude: flat-only files' `3D Raw Data` streams
happen to stay small enough that byte 58 (bits 16-23 of the size) reads
`0x03` in this corpus's sample, and real-signal files' larger streams
happen to read `0x09` - not a purpose-built boolean. See the 2026-07-20
session below for the full re-characterization, including the
analogous offset-88 field (`Max Plot` stream size) and a from-scratch
CRC/checksum sweep against the two still-unidentified offset-48 and
offset-80 fields.

**Verdict: this thread is closed.** Since the method configuration,
instrument parameters, and wavelength table are provably identical
byte-for-byte across files whose "real mode" encoding width differs, the
per-wavelength byte-width genuinely must be determined by the actual
per-timepoint intensity values themselves (i.e. some function of the
real, varying signal - consistent with the already-documented
magnitude-threshold partial signal and PDP-endian-float observation),
not read from any static per-run calibration/gain table anywhere in the
file. A future session should not spend more time searching for an
external width table in this corpus; the decode has to come from the
data-dependent angle.

Scripts: ad hoc, run via `python3 -c`/heredocs this pass and not
separately saved; built directly on `olefile.OleFileIO` stream reads, no
new helper functions beyond what existing scripts already provide.

## 2026-07-19 session (Max Plot crib-drag): negative result - Max Plot is not a simpler encoding, it's the same open problem restated

Tried using `PDA 3D Raw Data/Max Plot` as a known-plaintext crib source:
decode it (presumed simpler, single-channel-per-timepoint) to get real
numeric values, then search for their byte representation inside the
corresponding `PDA 3D Raw Data/3D Raw Data` segment. This did not work -
Max Plot turns out to share the main stream's unresolved variable-width
encoding rather than being a simpler, already-crackable format, so there
was no independently-known value to crib with. Recorded in detail so a
future session doesn't re-attempt the same "Max Plot must be easier"
assumption without new information.

- **Max Plot confirmed structurally as "one value per PDA timepoint"**:
  it uses the exact same 24-byte `RC\x00\x00` segment header as the main
  stream (magic, version, `npts`, `blocksz`, 2 zero-padding words), but
  as a single giant segment spanning the entire stream (`blocksz ==
  len(stream)`), the same pattern already documented above for `LC Raw
  Data`. Its `npts` field is **exactly equal to the main `PDA 3D Raw
  Data/3D Raw Data` stream's segment count**, checked in three files of
  different wavelength counts and envelope forms: `MSV000084197/
  20190607_NM16.lcd` (Max Plot npts=3502, PDA segcount=3502),
  `MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` (2674/2674),
  `PXD025121/1.lcd` (6190/6190). This confirms Max Plot really is a
  per-timepoint summary series aligned 1:1 with the main stream's
  segments (consistent with its name - almost certainly `max(intensity
  across wavelengths)` per timepoint), which is useful confirmation of
  its semantic identity even though the byte-level encoding remains
  closed.
- **Ruled out: fixed-width array of any kind.** `20190607_NM16.lcd`'s
  Max Plot payload is 6795 bytes for 3502 values (1.94 bytes/value
  average) - **too few bytes to even fit a full `u16` array**
  (`3502 * 2 = 7004 > 6795`), let alone a `float32` array
  (`3502 * 4 = 14008`). This rules out any fixed-width interpretation
  outright without needing to test one; Max Plot must use the same kind
  of variable-width-per-value encoding as the main stream, not a
  simpler fixed format.
- **Threshold/varint brute force targeting the exact `npts` count
  reproduces the main stream's over-parameterization trap.** Sweeping
  header-skip (0-11) x footer-skip (0-11) x magnitude-threshold (0-255)
  x wide-token-width (2-4) against `20190607_NM16.lcd`'s Max Plot
  payload, requiring an *exact* zero-leftover decode to precisely 3502
  values, produced **38 different parameter combinations that all
  "succeed"** by this count-only criterion (e.g. `header_skip=6,
  footer_skip=0, threshold=0x13, width=2` and many close variants).
  Manually decoding under the single most parsimonious of these
  (`header_skip=6`, no footer trim) produces values that are **not** a
  plausible chromatogram: consecutive decoded values jump randomly
  across almost the full `u16` range (e.g. first four values `28450,
  13887, 56639, 36640`, no smooth trend), and the indices of its 20
  largest decoded values (`[5, 43, 45, 49, 59, 130, 155, 193, 241, ...]`)
  do not overlap at all with the main stream's independently-known 15
  busiest (longest-payload) segment indices
  (`[90, 91, 92, 135-139, 2971, 3209, 3313, 3352, 3431, 3440, 3466]`).
  This confirms - the same way the entropy-pass session already flagged
  for pooled cross-file histograms - that matching an exact target
  *count* alone, with several free framing parameters available, is not
  reliable evidence of a correct decode; it will produce spurious
  "hits" from a large enough search space regardless of whether the
  decode is actually right.
- **PDP-endian float32 also fails here, unlike the main stream's
  isolated-window observation.** Tried the main stream's existing
  PDP-endian (word-swapped 16-bit halves) `float32` interpretation, at
  all 8 possible byte alignments (`offset 0-7`) and both remaining
  endiannesses, over the first 200 bytes of Max Plot's payload. Unlike
  the main stream's `03`/`04`-documented observation of individually
  plausible `0.5-2.0`-range floats in isolated windows, every alignment
  here produces a mix of degenerate zeros and wildly non-physical
  magnitudes (values like `5.9e+28`, `-8.3e+35`, `-10341.5`) with no
  alignment giving a clean, bounded, sane-looking run of floats. This is
  a genuine difference from the main-stream PDP-float observation worth
  flagging: whatever partial signal motivated the PDP-float theory
  there does not reproduce cleanly here, which weakens (without fully
  ruling out, since the main stream's own PDP-float signal was already
  known to be partial/inconsistent) confidence that fixed 4-byte float
  framing is the right general model for either stream.
- **Coarse cross-stream shape correlation: inconclusive, not
  supportive.** As a decode-free sanity check, computed a 40-byte
  rolling-mean "energy" proxy directly over Max Plot's raw payload
  bytes (no decode assumptions) and compared its highest-energy regions
  against the main stream's independently-known busiest segment
  indices. They did not line up (Max Plot energy peaked around segment
  index ~242-257; the main stream's actual busiest segments are at
  90-92, 135-139, 2971, 3209, etc.). This is weak evidence at best -
  raw byte magnitude is not a reliable proxy for encoded value magnitude
  under an unknown variable-width scheme, so this doesn't rule anything
  out - but it did not provide the hoped-for independent confirmation
  either.

**Verdict: this thread is closed for now, not worth further time without
a new idea.** Max Plot is not a simpler, independently-crackable stream
that could bootstrap the main payload decode via known-plaintext - it is
governed by what looks like the same unresolved variable-width encoding,
just applied to a single channel. Using it as a crib requires decoding
it first, which is circular given the main stream is exactly what's
still unsolved. If a future session wants to revisit Max Plot
specifically, the one thing that *would* be worth trying (not attempted
this pass, out of scope for a bounded single-pass effort) is checking
whether Max Plot's total-bytes-per-value ratio across many files
correlates with the main stream's own per-file modal "real mode" length
findings from the cross-file pass above - if both streams' encoders
scale together per-acquisition, that's still consistent with "same
encoder, different channel count" rather than offering a shortcut.

Scripts: ad hoc, run via `python3` heredocs this pass and not saved as
standalone artifacts under `re/src/analysis/` - all checks were built
directly against `iter_segments`-style parsing and `olefile` stream
reads (no reusable abstraction was warranted for a negative result).

## 2026-07-19 session (width-table retry on bimodal data): a real refinement (exact 3x-`npts` centering) but no clean decode

Retried the leading bitmap/nibble width-selector-table hypothesis
(previously ruled out on `MSV000084197`'s continuously-varying lengths,
see the "further ruled-out hypotheses" section above) against
`MTBLS432`'s cleaner bimodal data instead, reasoning that a large
same-length population of "real" segments within one file might make a
per-segment inline table more solvable. It didn't produce a clean decode,
but surfaced a genuine refinement of the cross-file pass's bimodal-cliff
finding, worth recording precisely.

- **Refinement: "real" mode length is centered on exactly `3 * npts`,
  not just "at least 1.8x `npts`".** Checked all 9 confirmed real-signal
  `MTBLS432` files' full real-mode body-length distributions (not just
  the min/max/modal points the cross-file pass reported): in every one
  of the 9 files, `3 * npts` (`204` for `npts`=68) is itself one of the
  observed lengths (with real segment counts at that exact length
  ranging from 21 to 644 depending on the file), and **98.5-100% of each
  file's real-mode segments fall within a +-20-byte window centered on
  `3 * npts`** (worst case `6-wk_KO_CC_male_13_27...lcd` at 98.5%, every
  other file at 100.0%). This sharpens the cross-file pass's "jumps to at
  least 1.8x `npts`" observation (which was reporting the minimum
  observed length, i.e. the near edge of this same band) into a tighter,
  more falsifiable claim: real-mode encoding reads as "3 bytes per value,
  nominally, with a modest number of individual values landing at 2 or 4
  bytes instead" - a small deviation from a 3-byte baseline, not an
  open-ended variable width. Any future decode theory should predict this
  specific centering, not just a lower bound.
- **Escape-byte and threshold-tier decodes built on that refinement:
  tried, and they don't hold up.** Modeled the deviation directly: walk
  the body in nominal 3-byte steps, with either (a) a single escape byte
  value that shrinks a token to 2 bytes, (b) the mirror version that
  grows a token to 4 bytes, or (c) a two-threshold, three-tier scheme
  (byte-value ranges select 2/3/4-byte width, all 6 permutations of
  width-to-range assignment swept) - all tested against
  `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`'s 2662 real-mode segments,
  requiring an exact `npts`-token, zero-leftover decode to count as
  clean. Single escape byte: weak, ~20% clean at best (83/400) regardless
  of which byte value or direction (shrink-to-2 vs grow-to-4) was tried.
  Three-tier threshold: looked much stronger on a small sample (181/300,
  60%, at `byte<0x40`->2 bytes / `byte>0xB4`->4 bytes / else 3 bytes) but
  **did not generalize** - re-run against a larger 600-segment sample,
  the same class of configuration topped out at 292/600 (49%), i.e. the
  small-sample result was itself an instance of the "too many free
  parameters produces spurious matches" trap this doc already flags
  elsewhere (six width-permutations times ~48x48 threshold pairs is a
  large enough search space to fit a sizeable minority of segments by
  chance). No single-byte or threshold-tier rule cleanly explains the
  encoding.
- **Bitmap-table least-squares, redone with real-mode segments only**:
  the original attempt (ruled out on `MSV000084197`) pooled flat and real
  segments together, which biases any width fit since flat segments
  trivially contribute all-zero-width codes. Redid the fit using only
  `12_65`'s 600 real-mode segments, for 1-bit and 2-bit per-wavelength
  codes: mean per-segment error dropped from the original attempt's
  6.8-21.8 bytes to 2.7-2.9 bytes, and fitted widths landed closer to
  plausible integers (`~2.7-2.85`, near the 2/3/4-byte range this
  session's other findings suggest) - a real improvement, but still not
  a clean, near-zero-error fit, so this is recorded as a partial signal
  and not treated as a working decode. A literal fixed leading
  bitmap/nibble table remains unconfirmed.

**Verdict**: this thread narrowed the target (real-mode width selection
centers tightly on 3 bytes/value with a modest 2-or-4-byte minority) but
did not crack the actual per-value selection rule. The `3 * npts`
centering fact is solid and worth any future session anchoring on, but
naive escape-byte/threshold/bitmap approaches built directly on top of
it were tried here and fell short - a future attempt should look for a
*data-dependent* trigger (the actual absorbance magnitude at that
wavelength, not a fixed byte-position rule) given that the external-table
hunt already ruled out anything read from elsewhere in the file, and this
session's own finding that this file's real-segment lengths spread across
18 distinct values (not one constant) confirms the deviation genuinely
varies per-timepoint, consistent with dynamic-range-driven selection
rather than a static per-channel rule.

Scripts: ad hoc, run via `python3` heredocs this pass and not saved as
standalone artifacts (all built directly on `iter_segments` from
`pda_varint_bruteforce.py` plus `numpy.linalg.lstsq` for the bitmap fit).

## 2026-07-19 session (transition-segment comparison): negative result - no mode-switch marker found, and a small correction to the transition-index claim

Following up on the cross-file pass's observation that segments 0-10 are
byte-identical across 10 `MTBLS432` files and "first divergence is
segment index 11," this pass targeted the transition point itself
directly: does the specific segment where flat encoding ends and "real"
encoding begins carry any shared structural marker (a flag byte, a
narrow value range, anything) across files, as opposed to the bulk
busy-region comparisons the cross-file pass already ran? Checked all 9
confirmed real-signal `MTBLS432` files. Result: **no marker found; this
sub-thread is closed.**

- **Small correction to the earlier transition-index claim**: re-verified
  the exact first-real-segment index per file (comparing decoded body
  length, i.e. `len(payload) - 4` for this symmetric form, against
  `npts`, not just payload length) rather than relying on the
  byte-identical-first-11-segments observation alone. The transition is
  **not always segment 11**: 5 of the 9 files (`..._34_73`, `..._26_68`,
  `..._28_70`, `..._33_72`, `..._13_27`) transition at index 11 as
  reported, but 4 of the 9 (`..._12_65`, `..._27_69`, `..._10_26`,
  `..._21_29`) transition one segment later, at index 12. The earlier
  "first divergence is segment index 11" claim was accurate for what it
  checked (10 files' first 11 segments are identical to each other) but
  should not be read as "every file's transition happens at exactly
  index 11" - it varies by one segment across this sample. Not a
  large correction, but worth recording so a future session doesn't
  hard-code index 11 universally.
- **No early-warning signal**: checked the four segments immediately
  before each file's own transition point (indices 7-10, i.e. well
  before either 11 or 12) across all 9 files - **every single one of
  these 36 segments (9 files x 4 segments) is exactly `npts` bytes of
  literal `0x00`, with zero nonzero bytes**, no partial buildup, no
  subtle divergence in content despite identical length. This confirms
  the mode switch is a genuine instantaneous cliff at the data level
  (consistent with the cross-file pass's length-histogram finding),
  not something foreshadowed a segment or two early by a soft signal
  this check could detect.
- **No shared marker at the transition segment itself**: compared the
  9 files' first-real segment directly - body length (192-204 bytes,
  no two files matching exactly except by coincidence), the envelope's
  `A`/`tail` values (equal to each other per the established symmetric
  form, no new information), the raw 24-byte segment header (`version`
  field is `1` and both padding words are `0` in all 9, same as every
  other segment in the corpus - unremarkable), and the body's first and
  last 16 bytes position-by-position. None of these show a consistent
  value, narrow range, or shared byte-run across the 9 files: the first
  body byte alone spans `0x00`, `0x40`, `0x41`, `0x44`, `0x5d`, `0x5e`
  across the 9 files with no overlap pattern, and no fixed-position byte
  anywhere in the first or last 16 bytes checked matches across even a
  majority of the 9 files. If a literal mode-switch flag byte exists in
  this stream, it is not visible as a fixed-position, fixed (or
  narrow-range) value at the start or end of the transition segment's
  body - the switch is inferred purely from body length jumping, not
  from any inline flag this check could find.
- **Interpretation**: taken together with the width-table retry pass's
  finding that "real" mode's per-value width looks data-magnitude-driven
  rather than table- or position-driven, and the external-table hunt's
  finding that no config stream carries an external threshold, the
  simplest remaining explanation is that the "flat vs. real" cliff is
  not a protocol-level flag at all - it is very plausibly just the
  natural consequence of the same per-value magnitude-driven width
  rule already suspected for individual values, applied to a spectrum
  where either every wavelength is at/near instrument noise floor
  (cheap 1-byte code for all 68 values) or a real UV peak has arrived
  and most/all wavelengths pick up enough signal to need the expensive
  code - i.e. the "hard cliff" could be an emergent property of a
  per-value rule, not evidence of a separate segment-level mode switch
  that needs its own explanation. This is a plausible reframing, not a
  proven fact - it was not independently verified beyond the absence of
  a marker in this check.

**Recommendation**: this specific sub-thread (hunting for an explicit
mode-switch marker at the transition point) is closed - none was found
across 9 files, and the negative results collectively point toward "no
separate flag; the cliff is emergent from the same unresolved per-value
width rule" rather than "there's a marker we haven't found yet." A
future session's time is better spent on the per-value magnitude-driven
width rule directly (per the width-table retry pass's recommendation to
look at magnitude-vs-width correlation) than on further transition-point
byte-hunting.

Scripts: ad hoc, run via `python3` heredocs this pass, not saved as
standalone artifacts (built on `iter_segments` from
`pda_varint_bruteforce.py`).

## 2026-07-19 session (temporal/delta redundancy): real, verified cross-segment correlation found, but not yet decodable

Strategy 5 of 5 for this day's investigation. Followed up on the earlier
entropy pass's compression finding (individual region-A payloads
compress to ~93% with zlib, but concatenating all 3502 of
`MSV000084197`'s region-A payloads in true temporal order reaches ~78%)
to check whether that gap reflects genuine per-value temporal
redundancy, or just a structural artifact (e.g. repeated header/envelope
bytes). Net result: **the redundancy is real and temporal, confirmed
with a proper randomized control**, but raw byte-level alignment alone
is not enough to turn it into a decode.

- **Order-shuffle test reproduces and localizes the effect**: re-ran the
  concatenated-compression check on `MSV000084197`'s region A (3502
  segments) three ways: mean individual-segment ratio 93.2%, true-order
  concatenation 77.9%, and **randomly shuffled segment order** (same
  3502 segments, same total bytes, order randomized) 79.8% across two
  different random seeds. True order compresses measurably better than
  any random order of the exact same data (~2 percentage points, both
  seeds agreeing to within 0.02pp) - this rules out "it's just a
  fixed-dictionary/window-amortization artifact of concatenating many
  chunks" (which would show no order-dependence) and confirms the
  effect is genuinely about *temporal adjacency*, not just aggregate
  size. The same true-vs-shuffled test on `MTBLS432`
  (`..._12_65...lcd`, its first 500 real-mode segments, both full
  24-byte-header+payload and body-only framings) showed close to *no*
  order-dependence (84.57% vs 84.83% full-segment; 94.40% vs 94.60%
  body-only) - the temporal signal is present in `MSV000084197` but
  much weaker or absent in this `MTBLS432` slice, plausibly because that
  slice's body length stays in a narrow, low-dynamic-range band
  (192-209 bytes, see the width-table retry section above) rather than
  tracking a real rising/falling elution peak the way `MSV000084197`'s
  segments 4-9 do.
- **Naive offset-alignment byte-diffing found nothing** (tried first,
  before the shuffle-control approach above): byte-diffed consecutive
  segments during a genuinely smooth, monotonically-growing run
  (`MSV000084197` segments 4 through 9, lengths 538->550->557->565->
  567->571, a real early elution-peak buildup) at every small alignment
  offset from -8 to +8 bytes. Best offset per pair still left 85.9-88.4%
  of overlapping bytes different - no offset reveals a low-diff
  alignment even during smooth, physically-continuous growth. This
  hypothesis (simple fixed byte-shift between consecutive segments) is
  ruled out.
- **Position-aligned delta (subtraction mod 256) shows real
  concentration near zero, verified against a randomized-pairing
  control - this is the positive finding**: computed `(byte_k in
  segment[i+1]) - (byte_k in segment[i]) mod 256`, interpreted as
  signed, for every *temporally consecutive* pair of same-length region-A
  segments in `MSV000084197` (76,416 byte positions total, all pairs
  with `len(a) == len(b)`): **22.4%** of deltas are exactly zero and
  **33.0%** fall within +/-8 of zero. To check this isn't just an
  artifact of the marginal byte-value distribution being spiky (the
  `0x20`/`0x3f`-leader-byte concentration already documented above), ran
  the identical computation on **randomly paired** same-length segments
  that are *not* temporally adjacent (830,063 byte positions, pairs
  drawn from indices at least 5 apart): only **12.0%** exactly zero and
  **22.8%** within +/-8. Temporally consecutive pairs show a real,
  roughly-doubled concentration near zero compared to the random-pairing
  baseline (22.4% vs 12.0% exact-zero) - this is genuine evidence of
  recoverable position-aligned temporal structure, not just a
  restatement of the marginal distribution's own spikiness. Note this
  finding is consistent with, not contradictory to, the already-documented
  "~80% of bytes differ between same-length adjacent segments" fact
  further up this document (`1 - 0.224 = 0.776`, matching almost
  exactly) - it's the same underlying data viewed through a
  signed-delta lens with a randomized control added, not a new
  contradiction.
- **What this does NOT give us**: raw delta entropy is actually *higher*
  than raw-byte entropy (7.20 bits/byte for the delta stream vs 6.08 for
  raw bytes, sampled over the first 500 segments) - the near-zero spike
  coexists with a heavier tail elsewhere, consistent with position `k`
  not corresponding to the same wavelength channel across segments whose
  *internal token boundaries* differ even when total length happens to
  match (variable per-value width means a byte-position match in total
  length doesn't guarantee semantic alignment throughout the body). So
  this result is evidence that a real, decodable temporal/delta
  relationship exists, not itself a working decoder - naive fixed byte
  position alignment captures only part of it.
- **Sparse "only-changed-channels get a token" hypothesis (speculative
  extension) not pursued further this session**: worth checking in a
  future pass (does deviation from a file's modal real-mode length
  correlate with anything computable independently, like the local
  rate of change between consecutive segments), but doing this
  rigorously requires either a partial decode or a much larger,
  more careful analysis than this session's remaining budget allowed;
  flagged rather than half-tested.

Scripts: ad hoc, run via `python3` heredocs this pass, not saved as
standalone artifacts (built on `iter_segments` from
`pda_varint_bruteforce.py`).

### Closing summary of 2026-07-19's five-strategy investigation

Across all five strategies attempted today (per-region parsing/width-table
sweeps, per-position/conditional entropy, cross-file byte-alignment,
external-table hunt, Max Plot crib-drag, width-table retry on bimodal
data, transition-segment comparison, and this temporal/delta pass), the
payload's exact per-value grammar remains **undecoded**. What's now
solidly established, taken together:

- Width selection is very likely **driven by the actual signal
  magnitude itself**, not a lookup table (no external config stream
  correlates with per-run modal length; ruled out), not a fixed
  per-channel/positional table (least-squares fits don't converge to
  clean integer widths even on the cleaner bimodal `MTBLS432` data), and
  not flagged by an explicit marker byte (no consistent value found at
  the flat-to-real transition point across 9 files).
- The flat/real split is a **hard, instantaneous cliff** with no
  gradual escalation and no early-warning signal in the segments just
  before it.
- Real-mode body length is **tightly centered near `3 * npts` bytes**
  for the symmetric (`MTBLS432`) form, i.e. close to but not exactly a
  uniform 3-bytes-per-value scheme.
- There **is** genuine recoverable temporal structure between
  consecutive segments (this pass's finding), confirmed against a
  randomized-pairing control, but it hasn't yet been turned into an
  actual decoder - naive fixed-position delta framing is necessary but
  not sufficient, since variable per-value width breaks simple
  positional alignment once segments diverge enough.

**Recommendation for a future session**: the single most promising
unexplored thread is combining the last two facts - a **joint
temporal-plus-magnitude decoder** that walks two segments' bytes in
lockstep, allowing per-value width to vary token-by-token (not fixed
per position), and uses "does this token's decoded value stay close to
the previous segment's corresponding-channel value" as the *acceptance
criterion* for candidate token widths, rather than trying to nail the
width rule from single-segment magnitude/position analysis alone (which
today's strategies 1-4 already pushed about as far as they profitably
can). This reframes the problem from "find the width rule" to "find a
width-and-value assignment that is simultaneously self-consistent
within a segment (exact byte-count match) AND smooth against the
previous segment's decode" - a joint constraint neither single-segment
nor naive fixed-position cross-segment analysis alone could exploit,
but which this session's verified near-zero-delta finding suggests
should be satisfiable for a meaningful fraction of channels.

## 2026-07-19 session (joint temporal+magnitude decoder): real but weak cross-segment signal confirmed via an independent method, no clean decode, and a concrete reason why not

Direct follow-up to the "temporal/delta redundancy" pass above, acting on
its recommendation: instead of trying to derive the per-value width rule
from one segment's bytes in isolation (strategies 1-4, all exhausted),
this pass tried using agreement between *two temporally adjacent
segments* as the acceptance criterion for candidate token widths - a
joint decode rather than a per-segment one. Built against
`MSV000084197`'s `PDA 3D Raw Data/3D Raw Data` stream (`npts` = 321,
split-envelope form), using the already-established body extraction
(`payload[4:len(payload)-4]`, i.e. treating the `A`+`tail` regions as one
contiguous body, consistent with the "no vocabulary difference between
regions" finding documented earlier).

**Method**: an exact, feasibility-pruned dynamic program walks two
segments' bodies in lockstep, one shared channel index at a time. At each
of the 321 channel steps, it considers all 16 combinations of candidate
token width (1-4 bytes) for the two segments' current token, decodes each
candidate as a magnitude-comparable scalar (`int.from_bytes(chunk,
'little') / 256**width`, projecting any width onto a common `[0,1)`
scale), and accumulates `abs(value_a - value_b)` as cost. States are
pruned at every step to only those where the remaining tokens can still
exactly reach both segments' true body lengths (`remaining*1 <=
bytes_left <= remaining*4`), which keeps the state space tractable
(peaked at 59,220 states for the pairs tested, well under both the 300k
and 500k caps used, so results below are genuine global optima for the
stated cost function, not beam-search approximations) and guarantees any
solution found lands exactly on both segments' true byte boundary with
zero leftover - the DP cannot produce a partial-consumption result.

- **True-neighbor pair (segment 4, segment 5)**, both real (non-flat),
  body lengths 530 and 542 bytes: the DP finds a zero-leftover joint
  parse with total cost **2.3789** (cost/channel **0.00741**) and
  per-channel width agreement between the two segments of **178/321
  (55.5%)**.
- **Random-pair control (segment 4 vs. segment 1000)**, a temporally
  distant, unrelated real segment, body lengths 530 and 602: cost
  **4.3407** (cost/channel **0.01352**, ~1.8x higher than the true
  neighbor), width agreement **154/321 (48.0%)**.
- Both numbers move in the same direction, independently reproducing
  (via a completely different method) what the temporal/delta-redundancy
  pass above already found via whole-stream compression and
  position-aligned byte deltas: **true temporal neighbors really are
  more self-consistent than random pairs**, on both cost and width
  agreement. This is a second, independent confirmation that real
  cross-segment structure exists.
- **But there is a concrete, verified reason this doesn't turn into a
  decoder**: re-running the *same* true-neighbor pair (4, 5) at
  deliberately looser search budgets exposes a degenerate solution
  space. A beam-width-200 approximation (suboptimal, cost 10.57 -
  meaningfully worse than the true optimum) produced **63.6%** width
  agreement; beam-width-500 (cost 7.40, still suboptimal) produced
  **61.7%**; the true, exhaustively-verified global optimum (cost 2.38)
  produced the **lowest** agreement of the three, 55.5%. Width agreement
  does not track solution quality monotonically - it should, if
  minimizing this cost function were actually recovering a single
  correct per-channel width assignment. Instead, many structurally
  different token-width alignments achieve similarly low cost, meaning
  the `raw_int/256**width` magnitude-comparable scoring function is not
  selective enough to pin down a unique decode, even though in aggregate
  it does distinguish true neighbors from random pairs.
- **Not extended to a longer chain**: each exact-DP call costs roughly
  100-120 seconds in this (Python, dict-based) implementation for
  `npts`=321, which was judged an inefficient use of remaining budget
  given the degeneracy problem just demonstrated - extending to a 10-20
  segment chain with the *same* underlying objective function would not
  be expected to resolve into a clean, generalizable decode without first
  fixing the objective (see recommendation below), so this pass stopped
  after establishing the true-vs-random-pair comparison and the
  degeneracy diagnostic, rather than grinding through many more
  ~2-minute DP calls for a result likely to remain similarly ambiguous.

**Recommendation for a future session**: the joint-decode framing is
still the right direction (it is the only approach so far that has
*independently reproduced* the temporal-structure signal via a different
method than compression/delta-histograms), but the specific acceptance
criterion needs to be sharper before it can function as an actual
decoder. Three concrete ways to tighten it, in rough order of expected
payoff: (1) replace the generic `raw/256**width` magnitude proxy with an
actual physically-motivated candidate decode (e.g. genuine
PDP-endian-float interpretation per the earlier-documented lead, rather
than a width-agnostic proxy scale) so the comparison is between real
candidate physical values, not an arbitrary normalization; (2) add an
explicit width-parsimony term that penalizes a channel's width
*changing* between neighboring segments (not just its decoded value
differing), which directly targets the degeneracy shown above; (3) chain
three or more segments jointly rather than pairs, since a spuriously
low-cost alignment is far less likely to remain low-cost simultaneously
against two neighbors than against just one - this would need a faster
(vectorized/numpy, or non-Python) DP implementation first, given the
~100s/pair cost already observed for a 2-segment joint parse alone.

Scripts: ad hoc, run via disposable Python files outside the repo (under
`/tmp`, not saved to `re/src/analysis/`) this pass, since each was a
short iterative experiment (initial beam-search prototype, an
unbounded-beam "exact" DP with backtracking, and the true-vs-control
comparison script) built directly on `iter_segments` from
`pda_varint_bruteforce.py`.

## 2026-07-20 session: full `CheckSum` field identification (2 of 4), a CRC/checksum sweep that rules out the standard algorithms for the other 2, and a feasibility note on the fp16/spectral-delta avenues

Picked up two of the "further avenues" this document's previous session
left open: fully characterizing the 112-byte `CheckSum` stream (avenue
3), and trying IEEE-754 half-precision floats and spectral-domain
(wavelength-to-wavelength) delta encoding (avenues 1 and 2). Net result:
one genuine structural correction with full corpus verification, a
thorough negative result on the two still-unidentified `CheckSum`
fields, and an analytical finding that the fp16/spectral-delta ideas -
as pure *framing* hypotheses - don't actually open new search space
beyond what earlier sessions' continuation-bit and magnitude-threshold
sweeps already covered exhaustively. **The per-value payload grammar
remains undecoded.**

- **`CheckSum` offsets 56 and 88 fully identified: exact stream byte
  sizes, not checksums.** Read as little-endian `u32` (not `u16`) at
  each offset, offset 56 equals the exact byte length of
  `PDA 3D Raw Data/3D Raw Data` and offset 88 equals the exact byte
  length of `PDA 3D Raw Data/Max Plot`, both with **zero mismatches
  across all 77 locally available files with a real `CheckSum` stream
  and both sub-streams present** (45 `MTBLS432` files - both the 9
  real-signal and the fully-flat ones, 31 `PXD025121` files, and
  `MSV000084197/20190607_NM16.lcd`; checked with `olefile`'s own
  `get_size()` against each file's independently-read stream length, no
  parsing of the payload itself required). This directly corrects the
  external-table-hunt session's reading of offset 58 as a "flat vs.
  real" flag byte - see the correction note inserted in that section
  above. The two `u32` fields' high words (bytes 50-51 and 82-83) were
  also confirmed `0x0000` in every file checked, consistent with them
  being plain sizes rather than some other 32-bit quantity that happens
  to coincide with a stream length.
- **Offsets 48 and 80 remain unidentified, but are now more precisely
  characterized as genuinely data-dependent (not a per-session token or
  random field), and a real checksum/CRC sweep has been run against
  them and come up empty.** First, a cheap cross-file check
  distinguishes "depends on file content" from "random/session-specific":
  among the 15 `MTBLS432` files sampled, 5 fully-flat files that all
  share the same segment count (`nseg=2674`) - and therefore, per the
  cross-file pass's byte-identity finding two sessions ago, byte-identical
  `3D Raw Data` stream content - all report the **exact same** offset-48
  value (`30350`) and offset-80 value (`28638`); a sixth flat file with a
  different segment count (`nseg=2673`) reports different values
  (`12815`/`57449`) for both. This rules out "opaque per-run token" (it
  would not repeat exactly across independent acquisitions) and confirms
  these fields are a deterministic function of the stream's actual byte
  content, exactly what the stream's name (`CheckSum`) implies - but a
  wide, explicit sweep against both fields still found no match:

  - **Algorithms tried**: 19 CRC-16 polynomials (including CCITT,
    IBM/ANSI, XMODEM, standard reciprocal/reversed variants, and several
    less common published polynomials) crossed with `{reflect-in,
    reflect-out} x {0x0000, 0xFFFF, 0x1D0F, 0x800D, 0xB2AA, and two
    data-derived seeds (stream length, segment count)} x {xorout 0x0000,
    0xFFFF}` - 19 x 2 x 2 x 7 x 2 = 1064 parameter combinations per
    byte-range tried, implemented with a from-scratch table-based CRC-16
    (no vendor or third-party crypto library beyond Python's own
    `zlib`/`crcmod`, both open-source, general-purpose, format-agnostic
    checksum implementations, consistent with `CONTRIBUTING.md`'s
    "independent open-source parsers used purely as format checkers"
    allowance - `crcmod` was used only to spot-check the from-scratch
    implementation's own predefined-name variants agree, not as a source
    of Shimadzu-specific knowledge). Also tried: `zlib.crc32` (both
    16-bit halves), `zlib.adler32` (low 16 bits), Fletcher-16, Fletcher-32
    (low 16 bits), and plain 16-bit additive/XOR byte sums.
  - **Byte ranges tried**: the full `PDA 3D Raw Data/3D Raw Data` stream
    bytes (segment headers and payloads together), payload-only
    (24-byte segment headers stripped), body-only (the split/symmetric
    envelope's 4/8-byte header-and-footer further stripped), the 24-byte
    segment headers alone concatenated, and (for the offset-80 field)
    the `PDA 3D Raw Data/Max Plot` stream bytes directly - the natural
    candidate given offset 80 sits in the same header-then-size
    structural slot, immediately before offset 88's confirmed Max Plot
    size, that offset 48 occupies relative to offset 56's confirmed `3D
    Raw Data` size.
  - **Result: no match, on any combination, against any of the four
    files checked simultaneously** (`12_65`, `1_63`, `34_73`, and
    `20_66` for the initial pass; `12_65`, `1_63`, `34_73`, and `21_29`
    for the widened rerun). A simple direct-sum-of-real-segment-bytes
    check (mod 65536) was also tried and did not match either offset.
  - This is a real negative result (a specific, bounded, reproducible
    search that came up empty), not proof no algorithm exists - Shimadzu
    could use a nonstandard polynomial, a different data range than the
    five tried, or a non-CRC construction entirely. But it means the
    "brute-force small CRC polynomials against known bytes" plan from
    the previous session's "further avenues" note does not immediately
    pay off with the standard polynomial set, so a future session
    revisiting this should either widen the byte-range search further
    (e.g. per-segment individual checksums rather than one whole-stream
    value) or treat this thread as lower-priority than the per-value
    grammar itself.

- **fp16 (binary16) as a fixed-width array: ruled out immediately by
  body length, as expected.** If real-mode bodies were a plain array of
  2-byte half-floats, body length would equal exactly `2 * npts` on
  every real segment. Checked directly: in `MSV000084197`'s split-form
  stream (`npts=321`, 3502 segments), body length equals `2 * npts`
  (642 bytes) in exactly **1 of 3498 real-mode segments** (segment index
  92, in the middle of a smoothly-growing run of neighboring lengths
  620/637/652/**642**/622/618/614 - a coincidental crossing, not a
  population), and in `MTBLS432`'s symmetric-form stream (`npts=68`),
  `2 * npts` (136 bytes) occurs in **0 of 2662** real-mode segments.
  This is the same kind of hard ruling-out the document's existing
  "byte-level volatility" and length-histogram findings already
  established for a generic fixed-width hypothesis, now specifically
  against fp16's 2-byte width.
- **A precise, previously-only-anecdotal characterization: the
  split-form's real-mode body length centers near `1.88 * npts`, not
  `3 * npts`.** The document's existing "tightly centered near `3 *
  npts`" finding was established only for the symmetric (`MTBLS432`)
  form; the split form's centering had only been described anecdotally
  via one early-buildup example (segments 4-9 of `MSV000084197`, lengths
  530-571 for `npts=321`, i.e. roughly 1.65-1.78x). Measured precisely
  across all 3498 real-mode segments of that same stream: mean real
  body length is **604.8 bytes = 1.884 * npts**, median **609 bytes =
  1.897 * npts**, and the full observed range is **530 to 668 bytes
  (1.651x to 2.081x npts)** - i.e. **100% of this file's real-mode
  segments fall in a `[1.6, 2.2] * npts` band**, a tighter and
  differently-centered analogue of the symmetric form's `3 * npts`
  finding, not the same constant. This is consistent with (not
  contradictory to) the existing magnitude-threshold sweep's
  best-performing configuration on this same file being a *2-byte* wide
  token with a 1-byte cheap case (`threshold=0x1f`/`0x20`), since a mix
  of mostly-2-byte with some 1-byte tokens naturally averages below 2.0
  and above 1.0 per value - it does not, by itself, favor fp16 over the
  already-tried magnitude-threshold framing.
- **Why neither fp16-with-escape nor spectral-domain delta actually
  reopens new search space, analytically** (no new code needed for this
  part - a scoping finding, recorded so a future session doesn't spend a
  session re-deriving it): both ideas change how a *token's bytes* are
  turned into a *number*, not which bytes belong to which token. The
  already-exhausted continuation-bit sweep (all 8 bit positions x both
  polarities x 0-11 byte header/footer skip) and magnitude-threshold
  sweep (all 256 threshold values x 2-4 byte wide-token width) already
  covered every possible token-boundary assignment those searches were
  capable of expressing - swapping the *numeric interpretation* of a
  found 2-byte or 4-byte token from "raw magnitude-threshold scalar" to
  "IEEE-754 binary16" or "delta from the previous wavelength's value"
  changes nothing about which byte offsets get grouped into which
  token, so it cannot by itself turn a boundary search that already
  plateaued at ~6% clean segments into a working decode. Both ideas
  remain genuinely useful, but only as **validators/tie-breakers** for
  candidate token-boundary assignments a future width-selection
  breakthrough would produce (does the resulting number look like a
  plausible fp16 absorbance value; does the resulting per-wavelength
  sequence look smoother when spectral-differenced) - not as
  independent framing strategies in their own right. This reframes two
  of the previous session's four "further avenues" from "untried
  decode strategies" to "untried validators," which is a more accurate
  scoping for whoever picks this up next.
- **Not pursued this session, for budget reasons, still open**: avenue
  4 (aligning the flat-to-real transition to elapsed retention time
  rather than segment count) was not attempted - there is no existing
  per-segment retention-time decode for the PDA stream in this
  codebase to cross-reference against (unlike TTFL's RT index), so
  establishing one would be its own sub-investigation before the
  time-alignment question could even be asked. Avenue 5 (physical
  plausibility/peak-shape as a soft validator) was not attempted either,
  for the same reason avenue 1's fp16 analysis above landed on: it
  requires a candidate token-boundary assignment to validate, and none
  of the framing searches to date (including this session's) have
  produced one to test it against.

Scripts: ad hoc, run via disposable Python files under this session's
own scratch directory (outside the repo, not saved to
`re/src/analysis/`), built on a from-scratch `olefile`-based
`iter_segments`/`body_of` pair mirroring the shape of
`pda_varint_bruteforce.py` from earlier sessions, plus a from-scratch
table-based CRC-16 implementation (`crc_fast.py`-equivalent) and
`crcmod`/`zlib` for cross-checking standard-named variants.

## 2026-07-20 session 2: block-floating-point/adaptive-scale hypothesis family tested and ruled out; CheckSum offsets 48/80 tested against count-based candidates; a quantified false-positive base rate

Direct follow-up to session 1 above, prompted by a specific architectural
observation: every hypothesis ruled out so far assumed a single *global*
rule (fixed threshold, fixed escape byte, fixed continuation-bit
position) applied uniformly across an entire file, but the fact that
real-mode body length tracks each segment's own signal magnitude
continuously (not a fixed multiple of `npts`) is exactly the signature
of a *per-segment adaptive* quantizer - block floating point (a shared
per-block scale/exponent with narrower per-value mantissas), as used in
some real instrument firmware, being the leading concrete example. This
session tried four concrete variants of that family against the payload,
plus a fresh pass at the two still-unidentified `CheckSum` fields as
plain counts/derived sizes rather than checksums. **No decode was
found; the per-value payload grammar remains undecoded.** One negative
result is worth flagging as a genuine methodological contribution in its
own right: a directly quantified false-positive base rate for this
document's own "exact npts count, zero leftover" acceptance criterion,
which puts a hard number on a pattern this document has flagged
qualitatively many times before (Max Plot's 38 spuriously-successful
configs, the width-table retry's 60%->49% sample-size collapse, etc.).

- **(a) Explicit per-segment header as a threshold source: ruled out,
  weakly.** Hypothesized the body's first 1-2 bytes are a small
  scale/gain field, separate from the first value token, and that a
  *global* (same for every segment/file) formula converts that field
  into a per-segment magnitude threshold for an otherwise-unchanged
  2-tier walk (byte-under-threshold = 1-byte token, else a wide token).
  Tried header lengths 0/1/2 bytes x four threshold-derivation formulas
  (`header_byte_0` directly, `header_byte_1`, low byte of a 2-byte
  header read as `u16`, `255 - header_byte_0`) x wide-token widths
  2/3/4, against `MTBLS432/..._12_65...lcd`'s 2662 real-mode segments.
  Best combination: **21/2662 (0.8%) clean** - weaker than the
  document's existing fixed-global-threshold sweep, not better. An
  explicit small header directly usable as a raw threshold value is not
  the mechanism (though this doesn't rule out a header that needs a
  more complex, not-yet-guessed transform).
- **(b) Classic fixed-width-per-block (true block floating point:
  every value in one segment shares the same width): ruled out.**
  If each segment's body were `H` header bytes plus `npts` values of a
  single *per-segment* (but not per-value) width `W`, then `(body_len -
  H)` should divide evenly by `npts` for the great majority of real
  segments, for some small `H`. Checked `H` from 0 to 5 bytes against
  the same 2662-segment sample: the best case (`H=0`) has only
  **70/2662 (2.6%)** of segments dividing evenly into an integer
  quotient in the plausible 1-6 byte range (all landing on quotient
  `3`, consistent with the existing `3 * npts` centering finding
  already documented, but far short of "most segments"), and every
  nonzero `H` tried does no better. Per-value width genuinely varies
  *within* a segment, not just *between* segments - ruling out a pure
  shared-exponent/fixed-mantissa-width model.
- **(c) Marker-bit escape with an algebraically-derived baseline width:
  a real within-file signal that does not survive cross-file testing.**
  The most promising variant tried: for each segment, `base_w = body_len
  // npts` and `n_wide = body_len - npts * base_w` are *exactly*
  computable in advance from body length alone (no fitting) under a
  "every token is `base_w` or `base_w + 1` bytes" model - so the only
  free choice is which bit, in which byte of each token's `base_w`-byte
  prefix, signals "this token is the wider one." Swept all 8 bit
  positions x 2 polarities x {marker in the token's first byte, marker
  in its last byte} (32 combinations) against 4 confirmed real-signal
  `MTBLS432` files (`..._12_65`, `..._34_73`, `..._26_68`, `..._21_29`
  - all `npts=68`, same instrument/method). Individual files show
  real-looking signal - `bit=5, polarity=set, marker=first-byte` reaches
  **868/2663 (32.6%)** clean on `..._34_73` and **1100/2663 (41.3%)** on
  `..._26_68` - with successes spread across a representative range of
  body lengths (207-220 bytes out of an overall 202-219 range on
  `..._34_73`, not clustered at a degenerate edge case) - but **the same
  configuration gets only 1.1% on `..._12_65` and 8.5% on `..._21_29`**,
  and no single configuration reaches even 15% on all four files
  simultaneously: the best worst-case (minimum-across-all-four-files)
  rate across the full 32-combination sweep is **1.2%**. Since all four
  files share the same instrument, method, and `npts`, a real shared
  encoding rule should generalize across them the way the envelope and
  `u32[2]` facts already do - this doesn't, so it's ruled out as the
  real rule despite looking compelling on any single file in isolation.
- **(d) Leading bitmap whose popcount should equal `n_wide`: ruled
  out.** A cheaper-to-test alternative to (c): instead of an inline
  per-token marker bit, hypothesized a leading `ceil(npts/8)`-byte
  (9 bytes for `npts=68`) bitmap at the start of the body (after 0-3
  header bytes) whose set-bit count equals the algebraically-required
  `n_wide` for a `base_w=2` model. Checked header offsets 0-3 against
  both `..._12_65` (2566/2662 segments applicable to the `base_w=2`
  assumption) and `..._34_73` (only 28/2663 applicable, since that
  file's real-mode segments center higher, nearer `3 * npts`): **zero
  matches in every case**. Ruled out.
- **A quantified false-positive base rate for this document's own
  "exact count, zero leftover" acceptance criterion** (a methodological
  finding, not a decode attempt): to sanity-check how much weight a
  single segment's "a threshold that decodes it cleanly" result should
  carry, checked - for a random sample of 400 real-mode segments from
  `..._12_65` - what fraction admit *at least one* successful
  `(wide_width, threshold)` combination out of the 3 x 256 = 768 tried
  (the same free 2-tier walk as (a) above, but with no header at all).
  **191/400 (47.75%) of segments admit at least one spuriously "clean"
  decode**, averaging **18.8 different working threshold values per
  successful segment**, and - most tellingly - **body-length statistics
  for segments that admit a lucky threshold are statistically
  indistinguishable from those that don't** (mean 202.6 bytes for
  "admits a threshold" vs. 202.8 bytes for "does not," essentially
  identical). This means, for this document's typical `npts` scale
  (dozens to low hundreds of tokens per segment), "there exists a
  threshold under which this one segment decodes cleanly" is close to a
  coin flip with no relationship to the segment's actual content -
  putting a concrete number on why this document has repeatedly found
  (Max Plot's 38-configuration false-positive set; the width-table
  retry's 60%->49% sample-size collapse; this session's own item (c)
  above) that single-file or small-sample "clean decode" rates cannot be
  trusted without an explicit cross-file or cross-segment generalization
  check, and why every genuinely-confirmed fact in this document (the
  envelope, `u32[2]`, the two new `CheckSum` size fields) was checked
  against every segment of multiple independent files before being
  written down as confirmed.
- **`CheckSum` offsets 48 and 80 tested against plain counts and derived
  sizes, not just checksum algorithms: no match found.** Before
  resuming the checksum-algorithm search from session 1, tested whether
  offset 48 (and separately offset 80) is simply a count or a
  differently-scoped size rather than a hash, using the two files with
  precisely known values (`..._1_63`/`..._20_66`/`..._30_71`/`..._3_64`/
  `..._44_76`, all `nseg=2674`, offset 48 = `30350`; `..._37_74`,
  `nseg=2673`, offset 48 = `12815`). First verified the assumption
  the candidate-count hypotheses depend on - that every segment always
  declares exactly `npts` points (i.e. total declared point count really
  is `nseg * npts`) - directly against `u32[2]` across both files' full
  segment lists: true in both, zero exceptions. Checked offset 48/80
  against: `nseg` (2674/2673), `npts` (68), `nseg * npts` (181832/181764),
  total stream bytes (already confirmed as offset 56, not 48), total
  payload bytes with segment headers stripped (192528/192456), total
  body bytes with the envelope also stripped (181832/181764, matching
  `nseg * npts` exactly since these are fully-flat files - a useful
  independent confirmation that flat-mode body length truly is exactly
  `npts` with no other overhead, but not a match for offset 48/80
  either), total 24-byte segment-header bytes (64176/64152), and the
  `Status` stream's own varying fields (offset 4 confirms `nseg`
  exactly, as already suspected, but offset 20 - `6784`/`6144` - turned
  out to depend only on `nseg` too, identical between `..._1_63` and two
  *different-content* real-signal files sharing the same `nseg`, so it
  cannot be the source of offset 48/80's real-vs-real variation either).
  **None matched.** Also tried an Internet-style (RFC 1071) 16-bit
  ones'-complement running-sum checksum - initially looked promising
  (one accidental exact match against `..._1_63`'s `Max Plot` stream
  for offset 80), but this did not reproduce against `..._12_65` or
  `..._34_73` with the same formula and is discarded as coincidental (a
  flat file's checksum has limited entropy to begin with, making a
  1-in-3-file accidental match unsurprising). This strengthens, rather
  than replaces, session 1's conclusion: offsets 48 and 80 are
  genuinely content-dependent (not simple counts, not opaque
  per-session tokens) but their exact algorithm remains unidentified
  after both a broad standard-checksum sweep (session 1) and this
  session's count/derived-size and Internet-checksum checks.

**Verdict**: the block-floating-point/adaptive-scale hypothesis family,
in the four concrete forms tried, is ruled out - most instructively via
(c), which demonstrates a real within-file statistical signal that
fails a proper cross-file generalization check, and the quantified
false-positive base-rate finding explains *why* that kind of signal is
expected to appear by chance at this token-count scale. This does not
close the door on every possible adaptive-scale mechanism (a genuine
per-segment gain header using a transform more complex than the ones
tried in (a), or a marker mechanism at a byte offset within `A`/`tail`
subregions rather than uniformly across the concatenated body, remain
untested), but the most natural, simplest versions of the idea do not
fit. `CheckSum` offsets 48/80 remain the least-understood pair of
fields in this document; a future session should treat them as lower
priority than the per-value grammar itself unless a stronger structural
clue emerges (per session 1's recommendation, unchanged by this
session).

Scripts: ad hoc, run via disposable Python files under this session's
own scratch directory (outside the repo, not saved to
`re/src/analysis/`), extending session 1's `common.py`
(`iter_segments`/`body_of`) with new `walk_fixed2tier` (2-tier
magnitude-threshold walker with a skippable header) and `walk_bfp`
(marker-bit block-floating-point walker) helper functions.

## 2026-07-20 session 3: the "split" form's two regions are a real, exact 256-channel/remainder wavelength split (resolved); a promising marker-bit signal traced to a compensating-error artifact, not a decode

Direct follow-up to session 2's region-local suggestion. While setting
up a per-region rerun of session 2's marker-bit test, this session first
had to establish how many of a stream's `npts` wavelength channels
actually live in region `A` versus region `tail` - previously confirmed
to exist (session 1's envelope work) but explicitly flagged as
"never explained" (session 1's factsheet). That turned out to have a
clean, exact answer, which is this session's first real result. The
second half of the session used that answer to retry session 2's
marker-bit hypothesis region-by-region, found a statistically genuine
(not-by-chance) signal in region `tail` - but then, applying the
"physical plausibility as validator" idea from this document's own
"further avenues" list for the first time, showed that signal does not
correspond to a correct per-channel decode. **The per-value payload
grammar remains undecoded**, but one previously-open structural
question is now closed, and a methodologically important cautionary
result is recorded precisely.

- **Resolved: region `A` always holds exactly the first 256 wavelength
  channels; region `tail` holds the remaining `npts - 256`.** Checked
  directly against every flat/baseline segment (not just the first one)
  in all four locally available split-form files: `MSV000084197/
  20190607_NM16.lcd` (`npts=321`) and `PXD025121/1.lcd`, `/10.lcd`,
  `/11.lcd` (`npts=327` each). At baseline, every value is exactly 1
  byte (session 1's already-confirmed flat-mode fact), so a flat
  segment's region-`A`/`tail` *byte* lengths are also directly its
  region *value counts* - and in **every flat segment checked across
  all four files, `len(A) == 256` exactly, with zero exceptions**
  (`321 - 256 = 65` values in `tail` for `MSV000084197`; `327 - 256 =
  71` for the three `PXD025121` files). This resolves session 1's
  "genuinely open" question about what the two declared-length regions
  mean: they are not an arbitrary or data-dependent split, or a
  "coarse array + exception list" (already ruled out in session 1 by
  the vocabulary check) - they are a fixed partition of the wavelength
  axis at channel index 256, extremely plausibly reflecting a
  256-entry (`2^8`, i.e. one-byte-addressable) hardware buffer or
  register width in the PDA detector's acquisition electronics. This
  also gives a complete, satisfying answer to session 1's other open
  question - why "split" vs. "symmetric" envelope form correlates with
  wavelength count: **every corpus file with `npts <= 256` uses the
  symmetric (single-region) form, and every file with `npts > 256`
  uses the split (two-region) form**, consistent with a design where a
  wavelength count that fits inside one 256-slot buffer needs no split,
  and one that doesn't gets divided into "the first 256" plus "the
  overflow." (This corpus has no file with `npts` near exactly 256 to
  test the boundary itself, so the precise cutoff semantics - e.g.
  whether `npts == 256` exactly would still split - remain unconfirmed,
  but the pattern is exact and exceptionless for every `npts` value
  actually present locally: `68`, `321`, `327`.)
- **Retrying session 2's marker-bit escape hypothesis per-region (now
  that the true per-region value counts are known) finds a real,
  well-controlled signal in region `tail` alone.** Using `n_A = 256`
  and `n_tail = npts - 256` (exact, not estimated) to compute each
  region's own `base_w`/`n_wide` independently, and sweeping the same
  32 marker-bit configurations from session 2 separately against each
  region: **region `A` shows no comparable signal** (best config only
  29/3498 = 0.8% clean on `MSV000084197`, no better than session 2's
  whole-body attempts). **Region `tail` shows something real**: with
  `bit_pos=5, polarity=set-means-wide, marker-in-first-byte`, clean
  (exact `n_tail`-token, zero-leftover) walks succeed on **2479/3498
  (70.9%) of `MSV000084197`'s real-mode segments**, and the *same*
  configuration, unchanged, succeeds on **2040/6187 (33.0%)**,
  **2129/6179 (34.5%)**, and **2384/6179 (38.6%)** of the three
  `PXD025121` files respectively - a real cross-file signal, unlike
  every marker-bit config tried against the symmetric-form `MTBLS432`
  files in session 2. Two independent randomized controls confirm this
  is not a base-rate artifact of the search space (the concern
  session 2's false-positive-rate finding raised): (1) replacing each
  segment's real tail bytes with **uniformly random bytes of the same
  length** gives **0/3498 (0.0%)** clean on `MSV000084197` and
  1.4%-5.2% on the three `PXD025121` files (vs. 33-39% real); (2) the
  stricter control - **shuffling each segment's own real bytes** (same
  exact byte-value multiset, scrambled order, controlling for the
  payload's well-documented non-uniform leader-byte distribution, not
  just for byte value frequency) - gives **7/3498 (0.2%)** on
  `MSV000084197`, vs. 70.9% for the real, unshuffled order. Real byte
  *order*, not just byte *content*, matters to this rule succeeding -
  genuine evidence of positional structure in region `tail`'s bytes.
- **But decoding actual values under this rule and checking them for
  physical plausibility - the "further avenues" list's avenue 5,
  attempted here for the first time - shows the walk is not finding
  real per-channel boundaries.** Decoded all `n_tail` channel values
  (raw little-endian integer per token) for every segment that walked
  cleanly under the rule above, then measured each channel's
  segment-to-segment smoothness (mean relative step size between
  consecutive successfully-decoded segments; lower means smoother,
  more chromatogram-like). **Channel index 0 of the tail region is
  dramatically smooth (mean relative step 0.024)** - but **every other
  channel (indices 1 through 64) ranges from 0.25 to 1.13**,
  statistically indistinguishable from a temporally-shuffled-order
  control's aggregate score (0.96) and from pure noise. Only one
  channel out of 65 looks like real data; the other 64 do not. Traced
  this to a concrete mechanism, not just a vague "it's noisy": in every
  one of the **3445 segments** where the marker fires "wide" for
  channel 0 (base 1 byte extended to 2), **the second byte of that
  "2-byte" token is exactly `0x00`, 3445/3445 times, no exceptions**.
  This is much more consistent with channel 0's real width being a
  *plain 1 byte* (the marker misfiring on a data byte that happens to
  have bit 5 set, which is common given the payload's already-documented
  `0x20`/`0x3f`-leader-byte concentration) and the walk's "second byte"
  actually being **channel 1's own real first byte** - which happens to
  be `0x00` essentially every time, i.e. wavelength channel 257 in this
  file is very plausibly a genuinely dead/unmonitored edge channel that
  reads exactly zero. The walk still reaches the correct *total* token
  count and byte length despite this misclassification because the
  very next channel's true value is trivially small - a directly
  observed, concrete instance of the **compensating-error phenomenon**
  session 2's false-positive-rate finding predicted abstractly: passing
  the "exact count, zero leftover" test (even one that also beats two
  different randomized controls) does not guarantee the token
  boundaries themselves are correct, only that *some* combination of
  possibly-wrong boundary choices happened to sum to the right total.
- **Follow-up: confirmed unconditionally, cross-file, with no
  marker-bit assumption at all.** Dropped the marker-bit framing
  entirely and checked directly: across all four split-form files
  (`MSV000084197` plus all three `PXD025121` files, **15,043 real-mode
  segments total**), region `tail`'s raw byte at position 1 is exactly
  `0x00` in **100.00% of segments, zero exceptions in any file**, and
  the raw byte at position 0 stays in a narrow, smoothly-varying range
  in every file (`87`-`141` across the four files' means of `107.3`-
  `120.6`) fully consistent with the already-observed channel-0
  temporal smoothness. This is strong, clean, marker-bit-independent
  evidence that region `tail` position 0 is genuinely a plain 1-byte
  token and position 1 is a hard-wired/dead channel reading a constant
  zero - reproducible across every file checked, not a `MSV000084197`
  quirk. Byte position 2 onward, by contrast, is dominated by the same
  `0x20`/`0x3f`/`0x3e`-ish leader-byte values already documented
  throughout this document for the undecoded payload in general (checked
  via a byte-value histogram at position 2: the four most common values,
  accounting for the large majority of segments, are `0x3c`-`0x3f`
  and `0x20`-`0x21`) - i.e. the genuine open grammar problem picks back
  up at channel index 2, unchanged from the rest of this document; only
  the first two channels of region `tail` are special-cased edge
  channels, not a hint about the general per-value rule.
- **Verdict**: region `tail`'s bytes do carry genuine, non-random,
  order-dependent structure that this specific marker-bit rule
  partially captures (the real-vs-shuffled-byte comparison is
  decisive on that point) - but the rule itself is not the correct
  per-channel grammar, as the physical-plausibility check demonstrates
  concretely rather than just abstractly. This is a case where a
  hypothesis survives a strong statistical control yet still fails a
  physical-plausibility check, which is exactly why this document's
  "further avenues" list included the physical-plausibility validator
  in the first place - and it is the first time that validator has
  actually been applied to a candidate decode, rather than remaining a
  suggestion with nothing yet to validate. A future session picking
  this up should not restart from session 2's whole-body marker-bit
  framing; the more promising, narrower thread is region `tail`
  specifically, and the concrete clue that channel 257 (tail-region
  index 1) reads as a near-constant `0x00` across thousands of segments
  in this file is a specific, reproducible, low-effort thing to verify
  and build on directly (e.g. check whether it holds in the
  `PXD025121` files too, and whether channel 0 is genuinely 1 byte wide
  in the great majority of segments once decoded without the spurious
  marker).

Scripts: ad hoc, run via disposable Python files under this session's
own scratch directory (outside the repo, not saved to
`re/src/analysis/`), extending session 2's `walk_bfp` with a
`split_regions` helper (extracts `A`/`tail` sub-bodies from a "split"
form payload using the already-confirmed `A`/`tail` length prefix/
suffix fields) and a `decode_bfp` variant that returns actual decoded
values (not just walk success) for the physical-plausibility check.

## 2026-07-20 session 4: re-running the threshold/continuation-bit sweeps with the corrected per-region target counts finds two more dramatic-looking cliffs, both root-caused to artifacts - plus a methodological correction to the physical-plausibility check itself

Direct follow-up to session 3, prompted by a specific observation:
session 1's original "per-region parsing instead of whole-body parsing"
sweep (in the "further ruled-out hypotheses" section, well before
session 3 established the 256-channel region boundary) isolated region
`A` but still required it to decode to exactly `npts` tokens - an
internally inconsistent test, since region `A` is now known to hold
only the first 256 of `npts` channels, not all of them. This session
re-ran the magnitude-threshold and continuation-bit sweeps against both
regions with the corrected target counts (`256` for region `A`,
`npts - 256` for region `tail`), plus a fresh sweep against `MTBLS432`
(no region split to get wrong in the first place, since `68 <= 256`).
**No working decode was found.** Two sweeps produced dramatic,
cliff-shaped signals that initially looked like real hits - both were
run all the way through to actual value decoding and a
physical-plausibility check (per this document's own established
practice) and both turned out to be artifacts, for two different
reasons. The second of those reasons is a genuine methodological
correction to the physical-plausibility check itself, worth any future
session internalizing before trusting a "smooth" result.

- **Region `A` (target=256): no signal, at either sweep.** Magnitude
  threshold: best `239/3498 (6.8%)` at `threshold=31, wide_w=2` -
  barely above the levels already documented as noise elsewhere in this
  document. Continuation-bit: best `76/3498 (2.2%)`. Physical
  plausibility of the threshold sweep's best candidate: only `2/256`
  channels smooth by the (subsequently-revised, see below) smoothness
  metric - consistent with edge-channel artifacts, not a real decode.
  Region `A`, even at its correct 256-channel target, remains as
  undecoded as the rest of this document's whole-body attempts.
- **Region `tail` (target = `npts - 256`): a dramatic, razor-sharp cliff
  - traced to the same channel-0/1 artifact session 3 already
  diagnosed, not new structure.** The magnitude-threshold sweep found
  something that initially looked like a breakthrough: `threshold=32,
  wide_w=2` decodes **2518/3498 (72.0%)** of `MSV000084197`'s region
  `tail` cleanly, and the *exact same* configuration reaches **33.4%,
  35.3%, and 40.2%** on the three `PXD025121` files - a real cross-file
  signal, confirmed against both a uniform-random-byte control
  (0.1%-9.4% across the four files) and a same-multiset-shuffle control
  (4.7%-14.8%), both decisively beaten. Critically, the sweep shows a
  **razor-sharp cliff, not a gradual peak**: `71.9%` at `threshold=31`
  down to essentially `0%` at `threshold=33` for the whole-tail walk (a
  drop of two full orders of magnitude over a threshold change of `1`),
  which mechanically traces to `0x20` (decimal `32`) being the single
  most common byte value in this payload (already documented
  extensively elsewhere in this document) - flipping the threshold from
  `32` to `33` reclassifies every occurrence of that one very common
  byte value from "wide" to "narrow" at once, which is enough on its own
  to explain a sharp transition without it necessarily marking a true
  semantic boundary. Decoding actual values and checking channel-by-
  channel temporal smoothness reproduced **exactly** session 3's
  finding: channel 0 alone is smooth (mean relative step `0.024`, all
  four files), every other channel is noise-like (`0.6`-`1.1`). Directly
  confirmed this is the *same* artifact, not a coincidentally similar
  one: `byte>=32` and `bit5-set` (session 3's marker rule) agree on
  `82.4%` of individual bytes checked, channel 0 is classified "wide"
  in **100%** of segments under this rule too, and its supposed second
  byte is `0x00` in 100% of those cases - the identical compensating-
  error mechanism session 3 root-caused, now shown to reproduce under a
  completely different-looking rule (a plain magnitude threshold, not a
  specific bit position), which is itself informative: **the artifact
  is robust to how the "wide" decision is framed, because it isn't
  really about a decision rule at all - it's a structural fact about
  channel 1 always being a hard-zero edge channel, which any rule that
  classifies channel 0 as "wide" will trip over the same way.**
  Stripping the confirmed 2-byte fixed prefix (channel 0 = 1 byte,
  channel 1 = constant `0x00`) and re-sweeping the remaining `n_tail -
  2` channels with the corrected target found only a smooth, gradual
  peak (not a cliff) topping out at **976/3498 (27.9%)** at
  `threshold=27` - and decoding that candidate showed **0 of 63**
  channels smooth, with per-channel mode-fractions of only `0.7%-1.9%`
  (i.e. genuinely close to uniformly random, the opposite of
  mode-dominated). Channels 2 and beyond in region `tail` remain
  undecoded.
- **`MTBLS432` (symmetric form, no region-boundary confound): a
  single-file result that looked like a genuine decode under this
  document's existing smoothness metric, and was not - which surfaced a
  real flaw in the metric itself.** A fresh magnitude-threshold sweep
  against `..._12_65...lcd`'s 2662 real-mode segments found
  `threshold=1, wide_w=3` (i.e. "a literal `0x00` byte is a 1-byte
  zero, anything else starts a 3-byte token") decodes **1805/2662
  (67.8%)** cleanly - and, alarmingly, the existing mean-relative-step
  smoothness check reported **66 of 68 channels "smooth"** (values
  `0.03`-`0.36`, only two channels above the `0.3` cutoff this document
  has been using). Before accepting this, cross-file testing (the same
  discipline session 2/3 already established) was run first: the exact
  same configuration reaches only **14/2663 (0.5%)** clean on
  `..._34_73...lcd`, **612/2663 (23.0%)** on `..._26_68...lcd`, and
  **363/2662 (13.6%)** on `..._21_29...lcd` - all four files sharing
  the same instrument, method, and `npts=68` - an immediate
  disqualifying sign on its own. But the more important finding came
  from asking *why* `..._12_65...lcd` alone looked so clean: **the
  mean-relative-step metric is fooled by highly repetitive (mode-
  dominated) decoded sequences.** Checking the value distribution per
  channel (not just the step-to-step differences) shows every one of
  the 68 channels has a single dominant repeated value accounting for
  **41%-96% of its 1805 decoded segments** (most channels 80%-95%), with
  the *non-repeated* values landing all over an essentially uniform
  24-bit range (min/max spanning `~10^4` to `~1.67*10^7`, no
  concentration) rather than smoothly drifting from the dominant value.
  A signal that is "the same fixed number 90% of the time, then jumps
  to an unrelated large random-looking number 10% of the time" trivially
  minimizes a *mean* relative-step statistic (nearly all steps are
  exactly zero) while being the *opposite* of physically plausible
  chromatography, which should show continuous, non-repeating drift as
  a peak rises and falls. This is consistent with, and mechanically
  explained by, an already-documented fact: `MTBLS432` real-mode body
  length is tightly centered near `3 * npts` bytes (the "width-table
  retry" session, well before this one), so a rule that treats nearly
  every byte as the start of a 3-byte token will often land on the
  *correct total length* by roughly chunking the body into thirds,
  without the chunk boundaries corresponding to genuine per-channel
  values - and apparently tends to re-derive the *same* misaligned chunk
  boundary (hence the same wrong "value") on the majority of segments
  in this one file specifically, which is plausible given how much of
  this file's own body-length distribution is already known to cluster
  tightly (`194` bytes in `1767/2674` segments per the factsheet). A
  continuation-bit sweep against the same file found nothing better
  (`34/2662`, `1.3%`). **This route is closed for `MTBLS432` as tried.**
- **Methodological correction, worth recording precisely so it doesn't
  recur**: this document's physical-plausibility check (introduced in
  session 3) computed only the mean relative step between consecutive
  decoded values per channel. This session shows that statistic alone
  is insufficient - it can be minimized by a decode that is mostly a
  frozen, repeated (mode-dominated) value with rare large excursions,
  which is not real physical smoothness. **Any future
  physical-plausibility check should also report each channel's mode
  fraction** (the proportion of decoded values equal to that channel's
  single most common value) **and treat a high mode fraction (this
  session saw up to 96%) as disqualifying, not confirming** - genuine
  chromatographic data should show low repetition and continuous drift,
  not a dominant constant. Both of this session's sweeps were re-checked
  against this corrected two-part test (low mean relative step *and* low
  mode fraction); neither survived it.

**Verdict**: the corrected per-region target counts did not produce a
working decode, and the two most promising-looking candidates this
session found were both run through to real value decoding and a
(now-corrected) physical-plausibility check rather than being reported
as hits on the strength of a clean walk rate or a sharp threshold cliff
alone - both failed once actually inspected, for concretely identified,
different reasons. The per-value payload grammar for both the split and
symmetric envelope forms remains undecoded.

Scripts: ad hoc, run via disposable Python files under this session's
own scratch directory (outside the repo, not saved to
`re/src/analysis/`), building `region_sweep.py` (generalized magnitude-
threshold and continuation-bit walkers/sweepers parameterized by target
token count and optional header skip, reusable against any region) and
`decode_threshold.py` on top of session 3's `regions.py` helper.

## 2026-07-20 session 5: region-A-isolated entropy re-check (reproduces, doesn't extend, the earlier finding); a corrected-target joint DP with an anti-mode-collapse penalty term, tested more rigorously than the original single-pair anecdote

Two follow-ups, both aimed at leads that fall directly out of the
256-channel region boundary (session 3) and the mode-fraction lesson
(session 4). **Neither produced a working decode.** Both are honest,
fully-executed negative results with genuine new information in them
(a reproduced-not-contradicted entropy characterization, and a more
rigorous re-test of a claim this document had previously only supported
with a single-pair anecdote), not abandoned attempts.

### Region-A-isolated entropy analysis: reproduces the earlier whole-stream numbers almost exactly, confirms no periodic marker

The 2026-07-19 per-byte-position entropy session ran before the
256-channel region boundary was known, but - important scoping fact
worth stating precisely - that session's "region A" and "region tail"
labels were *already* byte-accurate: the `A`/`tail` byte ranges come
from the envelope's own length-prefix fields (confirmed since session
1), which session 3 never touched. What session 3 corrected was the
*target token count* used by the threshold/varint decode sweeps, not
which bytes belong to region `A`. So re-running the entropy analysis
with the "correct" region boundary was a real thing to check (the
earlier session's byte extraction could in principle still have been
subtly wrong), but was not guaranteed to change anything - and it
didn't:

- **Marginal and conditional entropy reproduce the 2026-07-19 figures
  to three decimal places.** Region `A`: `H(byte) = 6.281` bits,
  `H(next|current) = 5.409` bits (13.9% reduction) - versus the
  earlier session's `6.279` / `5.408` (13.8%). Region `tail`: `H(byte)
  = 5.793`, `H(next|current) = 4.696` (18.9% reduction) - versus the
  earlier session's `5.783` / `4.697` (18.8%). Region `A`'s
  individual-vs-concatenated compression ratio (`93.3%` vs `77.9%`)
  also matches the earlier session's numbers closely. This is a
  genuine confirmation, not a redundant no-op: it directly verifies the
  entropy session's region extraction was already correct, so no
  periodicity or marker was being smeared out or hidden by a region
  misalignment that has now been fixed - the entropy characterization
  itself needed no correction.
- **New check: region `tail` conditional entropy with the two known
  edge channels (position 0 and 1, identified in session 3) excluded**,
  to see whether they were inflating the "genuine structure" finding.
  Channels 2 onward: `H(byte) = 5.753`, `H(next|current) = 4.726`
  (17.9% reduction) - only marginally lower than the full-region figure
  (18.9%). The edge channels contribute a small amount to the
  conditional-entropy reduction (unsurprising, since channel 1 being a
  hard-wired constant given channel 0 is trivially predictable) but
  most of the "genuine local structure, not noise" finding survives
  their removal - real structure exists in the still-undecoded
  channels 2+ too, consistent with session 4's finding that those
  channels are not simply noise-passing-as-signal, just not yet
  decoded.
- **No new periodicity found at region `A`'s true 256-channel scale.**
  Checked mod-3 (the residue class the original entropy session flagged
  and then explained away as a low-variance-early-channel artifact, not
  a token marker) across the *entire* region `A` span (not just the
  first 40 bytes): the low-entropy-at-residue-1 pattern is strong only
  for roughly the first 60 bytes (entropy `2.1`-`3.7` bits, one byte
  value `0x40` accounting for up to 75% of occurrences) and fades to a
  flat `~5.3`-`5.7` bits at all three residues by position `~120` and
  beyond - reproducing, not contradicting, the earlier session's
  "low-variance early wavelengths, not a 3-byte-token marker"
  conclusion, now checked against the true 420-536-byte region `A` span
  rather than an approximate one. Also checked mod-2 (motivated by
  region `A`'s own average width, `1.89`-`1.91` bytes/channel, being
  closer to 2 than 3): no distinction between residues at any position
  range checked (`[0,40)`, `[40,120)`, `[120,250)`, `[250,420)`) - both
  residues track each other closely everywhere, ruling out a period-2
  marker just as cleanly as period-3 was already ruled out.

**Verdict**: this was a legitimate check to run (the region boundary
*could* have changed the entropy picture), but it didn't - the earlier
session's entropy characterization holds up exactly, and no
periodicity at either of the two most plausible token widths (2 or 3
bytes, matching region `A`'s and region `tail`'s own observed average
widths) was found anywhere in region `A`'s true span. If a marker byte
exists, it is not visible as a fixed-position, fixed-periodicity
entropy dip - consistent with (not a new contradiction of) the
transition-segment-comparison session's earlier "no marker byte found"
conclusion for a different part of this problem.

### Joint temporal+magnitude DP, re-scoped to region-correct target counts, with an anti-mode-collapse cost term

The 2026-07-19 joint-DP session built and ran its dynamic program
before the 256-channel boundary was known, walking the *combined*
`A`+`tail` body as one `npts`-token problem. Rebuilt the DP from
scratch this session (the original implementation was not saved,
consistent with that session's own "ad hoc, not saved" note) with two
corrections: region `A` and region `tail` scored as independent
sub-problems with their own correct target counts (`256` and `npts -
256` respectively), and an optional cost term that adds a fixed penalty
whenever the two segments' decoded values for a channel come out
*bit-identical*, directly targeting the mode-collapse/frozen-channel
failure mode session 4 diagnosed (a real decode should show continuous
drift, not exact repeats).

- **Region `tail` (small enough to run at scale - `nT=65`, ~2-3
  seconds/pair): the true-vs-random-pair effect reproduces in
  aggregate, but is markedly weaker and less certain than the original
  session's single-pair anecdote implied.** The 2026-07-19 session's
  entire evidentiary basis for "true neighbors are more self-consistent
  than random pairs" was **one** true pair versus **one** random pair
  (segment 4-vs-5, cost `2.3789`, versus segment 4-vs-1000, cost
  `4.3407`, a `1.8x` difference). This session re-ran the comparison
  across **20 true-neighbor pairs and 20 matched random-pair controls**
  (same anchor segment, a random partner at least 50 segments away),
  with `exact_match_penalty=0` (i.e. the original, unmodified cost
  function, applied for the first time to region `tail` alone at its
  correct target count). The aggregate direction replicates - mean
  cost/channel `0.0113` (true) vs `0.0128` (random), true lower as
  expected - but the *effect size* is much smaller than the original
  `1.8x` anecdote (`~1.13x` here), and a **per-pair sign test shows
  true-neighbor cost was lower in only 12 of 20 pairs (60%)** - not
  distinguishable from chance at this sample size. This is a real,
  useful correction to how confidently this document should state the
  original finding: the *direction* of the temporal-correlation claim
  holds up under a more rigorous multi-pair re-test, but the
  *magnitude* the original single anecdote suggested was likely
  optimistic, and a future session citing this finding should say
  "weak, aggregate-level signal, not reliable pair-by-pair" rather than
  repeating the original `1.8x` figure as if it were typical.
- **The anti-mode-collapse penalty term did not improve, and on this
  sample appeared to worsen, the DP's selectivity - a negative result
  for the specific fix tried, not a validation of it.** The original
  session's diagnosed weakness was that width agreement does not track
  solution cost monotonically. As a proxy for "does the objective
  reward genuinely-correct-looking solutions," computed the correlation
  between per-pair cost/channel and per-pair width agreement across the
  same 20 true-neighbor pairs: with no penalty (`exact_match_penalty =
  0`), correlation is `-0.135` (weakly in the hoped-for direction -
  lower cost mildly associated with higher agreement); with the
  penalty active (`exact_match_penalty = 1.0`), correlation flips to
  `+0.338` (moderately in the *wrong* direction - lower cost associated
  with *lower* agreement). This is the opposite of what the penalty
  term was designed to achieve. It is a small sample (20 pairs, one
  file, one penalty value) and should not be read as definitively
  ruling out every possible anti-repeat cost term, but the specific,
  concretely-implemented version tried this session - a fixed penalty
  for bit-identical decoded values between two segments - does not fix
  the selectivity problem and should not be assumed to without
  evidence. **This route, as implemented, is closed.**
- **Region `A` (target=256): computationally intractable for the exact
  DP as implemented, confirming and sharpening the original session's
  own noted limitation.** The original whole-body DP already reported
  needing 100-120 seconds per pair at `npts=321` on the combined
  `A`+`tail` body. Region `A` alone, at its corrected target of 256
  channels, is worse: a single true-neighbor pair (body lengths 441 and
  443 bytes) did not converge within a 3,000,000-state budget in 50
  seconds of search - state space growth outpaced the feasibility
  pruning that kept the original whole-body DP tractable (peaking at
  "only" 59,220 states there). This is plausibly because region `A`'s
  own average width (`~1.7`-`1.9` bytes/channel, closer to a boundary
  between width-1 and width-2 tokens) leaves the width-1-vs-2 choice
  ambiguous at many more positions than the combined-body case did,
  which is exactly the kind of denser branching that defeats this
  pruning strategy. Not pursued further at larger state budgets this
  session, given the already-substantial time cost for a single pair
  with no sign of convergence; a genuinely faster (vectorized, or
  non-Python) implementation - which the original session's own
  recommendation already anticipated needing for a longer chain - would
  need to come first before region `A` specifically is tractable for
  this method at all.

**Verdict**: both corrections from this session's two leads were
implemented and tested honestly, neither produced a working decode, and
the more rigorous multi-pair re-test of the original DP's headline
claim found the underlying temporal-correlation signal real but weaker
than previously documented - a useful correction for any future session
citing that finding, even though it doesn't change this document's
overall undecoded status.

**What's left, in the requester's own words, if a future session wants
to keep going here**: (1) a faster (non-Python, e.g. Rust or a
vectorized numpy formulation) joint-DP implementation, to make region
`A`'s larger state space and a longer multi-segment chain (beyond
pairs) actually tractable - both were already recommended by the
2026-07-19 session and remain the single most concrete unblocked next
step; (2) a different, non-fixed-penalty formulation of the
anti-mode-collapse idea (e.g. penalizing a *run* of 3+ consecutive
identical values specifically, rather than any single bit-identical
pair, since real adjacent samples of a slowly-varying signal can
legitimately coincide occasionally) - this session tested only the
simplest version and found it unhelpful, not every version; (3) fully
manual, by-hand byte inspection of a handful of segments (not a
parameterized sweep or DP), which no session including this one has
actually done - every attempt to date has been an automated search over
some hypothesis family, and the corpus of things automated search
finds unpromising is now large enough that manual inspection may turn
up structure a parameterized search wasn't shaped to find.

Scripts: `regions.py` (session 3, reused), `joint_dp.py` (new this
session - a from-scratch reimplementation of the 2026-07-19 joint DP,
since the original was not saved, extended with the optional
`exact_match_penalty` term and generalized to take an arbitrary target
token count so it works against an isolated region rather than only a
combined `npts`-token body).

## 2026-07-20 session 6: manual byte-level reading (no sweep, no hypothesis-first search) - a genuine new inspection method, a strong but ultimately artifact-explained "leading byte" lead, and an eye-verified confirmation of the hard-cliff transition

Every session to date has been hypothesis-first: guess a scheme, sweep
parameters, check zero-leftover, then (since session 3) check physical
plausibility. This session deliberately inverted that order - close,
manual, hypothesis-light reading of actual bytes, reaching for a
parametrized test only once something specific in the raw data
suggested one. **No new testable hypothesis survived scrutiny, and the
per-value payload grammar remains undecoded** - but the manual read
surfaced a genuinely new observational technique (inspecting segments
whose body length exactly equals `3 * npts`, which requires no decode
algorithm at all to align to a token grid) and one quantified, honest
account of *why* a promising-looking signal from that technique
dissolves under scrutiny, distinct from - but related to - session 4's
mode-fraction lesson.

### Byte-diffing two adjacent real-mode `MTBLS432` segments by hand

Started with `..._12_65...lcd` (symmetric form, `npts=68`, no
region-A/tail split to worry about), segments 30 and 31 (lengths 201
and 203, a quiet/plateau part of the run away from both the transition
and any sharp peak). A naive fixed-offset byte-by-byte diff (printed in
8-byte rows with a running differing-byte count) reproduced the
already-documented "80-92% of bytes differ" finding almost exactly - 7
or 8 of every 8 bytes differ at nearly every row, which is expected
given genuinely variable-per-value width means a fixed byte offset
essentially never lines up with the same channel in both segments once
any earlier channel's width has diverged.

- **A closer, signed-delta-by-position read (not an aggregate percentage)
  found something real: a cluster of small deltas at byte positions
  that are consistently multiples of 3, and only when the byte value at
  that position falls in the `0x40`-`0x5f` range** - the same
  dominant-leader-byte range this document has flagged since its very
  first PDP-endian-float observation. Of 68 real channels' worth of
  byte positions checked by hand (0 through 202), roughly a dozen
  positions showed `|delta| <= 4` while their neighbors showed deltas
  in the hundreds - and every one of those small-delta positions landed
  on a position `≡ 0 (mod 3)`, consistent with (not a new contradiction
  of) this document's already-established `~3 * npts` centering for
  this envelope form. Tried decoding 4-byte windows starting at these
  positions as `float32` under all four `{LE, BE} x {plain,
  PDP-word-swapped}` byte orderings already explored in earlier
  sessions: none gave consistently plausible small-magnitude values
  across even the pairs of positions checked, so a straightforward
  "this byte starts a 4-byte float" reading does not explain the small
  deltas on its own.
- **A new, genuinely useful observational technique: segments whose
  body length is exactly `3 * npts` need no decode algorithm to align
  to a token grid at all.** If a segment's real-mode body is exactly
  204 bytes for `npts=68`, and the true encoding really does put most
  values at 3 bytes each, then position `channel * 3` is very likely
  each channel's true leading byte, checkable by direct indexing with
  no width-inference, no threshold, no walk. `..._12_65...lcd` has 70
  such segments (out of 2662 real-mode segments), including one run of
  **4 temporally consecutive** exact-204 segments (indices 133-136).
  Reading the leading byte of each channel's 3-byte group across this
  run by eye: channels 0-8 and 14 are strikingly smooth across segments
  133, 134, and 135 (e.g. channel 4 reads `93, 93, 93` exactly; channel
  5 reads `64, 64, 64`; channel 0 reads `71, 69, 68`, a plausible small
  drift) - but segment 136 shows a **simultaneous jump across every
  channel at once** (channel 4 jumps to `225`, channel 5 to `198`,
  etc.), which reads as segment 136 not actually being a uniform
  3-bytes-per-value body despite its total length coincidentally
  matching `3 * npts` (consistent with this document's own
  already-established fact that only a mix of widths, not a strict
  per-value 3-byte rule, explains most 3-multiple lengths).
- **Extending this check to all 70 exact-204 segments (not just the one
  lucky run) shows the initial "smooth!" impression does not hold up,
  and traces to a specific, quantifiable artifact.** Only 15 of the 70
  exact-204 segments are temporally adjacent to another exact-204
  segment (15 total adjacent pairs, mostly isolated pairs plus the one
  run of 4) - using all 15 pairs, mean per-channel relative step
  (session 3/4's smoothness metric) is `0.516`, and only **15 of 68
  channels** fall under the `0.3` "plausible" threshold used elsewhere
  in this document. Checking *which* 15: they are exactly the channels
  at the extreme low-index (`0`-`8`) and high-index (`63`-`67`) ends of
  the wavelength range - i.e. exactly the "low-variance early
  wavelength" channels this document's 2026-07-19 entropy session and
  this session's own region-A-isolated re-check (earlier in this same
  session set) already identified as inherently low-dynamic-range, not
  evidence of a correct decode. Quantified directly: **correlation
  between a channel's smoothness score and its number of distinct
  leading-byte values across the 70 exact-204 segments is `r = 0.840`**
  - channels that are more genuinely variable (higher value diversity,
  presumably more analytically active/real) systematically score
  *worse* on the smoothness metric, and channels that barely vary at
  all trivially score well. This is a distinct but closely related
  trap to session 4's mode-fraction lesson: session 4 showed a
  literally mode-*dominated* (one repeated value 80-96% of the time)
  decode can look smooth; this session shows that even without hitting
  that extreme, a channel's *low value diversity alone* - independent
  of whether the decode is correct - mechanically produces a good
  smoothness score. **A physical-plausibility check should be read
  skeptically for any channel with few distinct decoded values, not
  just ones with one dominant repeated value.**
- **Cross-checked against the general (non-exact-204-only) walk from
  session 4** (`threshold=1, wide_w=3`, which parses 67.8% of this
  file's real segments): extracting only the leading byte of each wide
  token (rather than the full 3-byte integer session 4 used) does
  *not* fix that walk's mode-collapse problem - mean per-channel mode
  fraction is still `0.887`, indistinguishable from session 4's
  original finding. This confirms session 4's root cause was correctly
  diagnosed as being about the walk's *token boundaries* themselves
  (which segments outside the lucky exact-204 case do not reliably
  find), not about which bytes of a correctly-bounded token get turned
  into a value.

### Hand-inspecting the flat-to-real transition segment

Read the exact first-real-mode segment body (immediately following the
last all-zero baseline segment) byte-by-byte, by eye, against the
all-zero segment before it, for two files: `..._12_65...lcd` (first
real segment, 200 bytes, following 11 segments of pure `0x00`) and
`..._13_27...lcd` (first real segment, 192 bytes, the file among the 9
confirmed real-signal `MTBLS432` files with the most zero bytes in its
transition segment, on the theory that more zeros might mean a gentler,
more legible onset). Also checked zero-byte counts in all 8 other
confirmed real-signal files' first-real segments for comparison (range:
1 to 7 zero bytes out of 192-204).

- **No file shows a "mostly still zero, one or two channels just
  starting to show signal" onset.** Every first-real segment checked is
  immediately as densely populated with the same `0x40`-`0x5f`/leader-
  byte-dominated content as any other real-mode segment deeper into the
  run - reading `..._13_27...lcd`'s first-real segment byte-by-byte
  (its 7 zero bytes, the most of any file checked, are scattered at
  positions `0, 49, 54, 80, 81, 100, 167` - not clustered at the start
  or in any other single contiguous run) shows no leading quiet region,
  no trailing quiet region, and nothing that reads as a length-prefix
  or marker field distinct from the rest of the body.
- This is a genuine, eye-verified **confirmation**, not a new finding
  by itself: the 2026-07-19 transition-segment-comparison session
  already established via aggregate statistics that the flat-to-real
  switch is an instantaneous cliff with "zero nonzero bytes" in the
  segments immediately before it and no shared marker at the transition
  segment itself. This session read the *actual bytes* of the
  transition segment directly (not just its length or a checksum) for
  the first time, across two files, and found the same thing a human
  reading the raw hex would conclude: there is no visible structural
  seam at the transition, only genuinely dense, high-entropy real data
  starting immediately in every channel at once.

**Verdict**: this session's inverted, manual-first approach found one
real, reusable technique (exact-`3*npts`-length segments as an
algorithm-free way to inspect a token grid) and used it honestly enough
to catch its own initially-promising result as an artifact, rather than
reporting the encouraging-looking first four-segment run as a hit. The
transition-segment read adds direct, by-eye confirmation to an
already-well-supported fact rather than contradicting or extending it.
Neither line of inquiry produced a specific byte pattern, marker, or
periodicity that suggests a new parametrized hypothesis worth building
and sweeping. Per this round's own instructions, this is being recorded
as a legitimate close to this manual-inspection lead for now rather than
forced further: the payload's per-value grammar remains undecoded after
six same-day sessions of systematic and, this round, manual
investigation, and this document's own historical pattern (this
session's finding included) is that promising-looking signals at this
scale of investigation keep resolving to already-known artifacts
(edge-channel low variance, value-diversity-driven metric bias,
compensating token-boundary errors) rather than to the real grammar.

Scripts: ad hoc, run via disposable Python files under this session's
own scratch directory (outside the repo, not saved to
`re/src/analysis/`), reusing `common.py`'s `iter_segments`/`body_of`;
no new reusable helper modules were warranted for a manual-reading
session.

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

## Further avenues for a future session

The 2026-07-19 closing summary's single recommendation (sharpen the
joint temporal+magnitude decoder's scoring function) was attempted in
2026-07-20 session 5, with a negative result for the specific
anti-mode-collapse penalty term tried - see that session for detail and
for three concretely scoped follow-ups it left open (a faster/non-Python
DP implementation, a run-length-based rather than single-pair-based
anti-collapse penalty, and fully manual byte inspection). The third of
those was attempted in session 6 - general manual byte-level reading
(not targeted at region `tail` specifically) of `MTBLS432` real-mode
segments and the flat-to-real transition segment, with no new testable
hypothesis surfacing, though one reusable inspection technique
(exact-`3*npts`-length segments as an algorithm-free token-grid anchor)
came out of it. The region-`tail`-specific manual read described in the
next bullet remains untried. The items below are additional,
not-yet-tried directions - none executed this round, so treat them as
leads, not findings:

- **(New, from 2026-07-20 session 3; followed up in session 4 with a
  negative result) Push the region-`tail` walk past channel index 2.**
  Session 3 confirmed, unconditionally and across all four split-form
  files (15,043 segments, zero exceptions), that region `tail` position
  0 is a plain 1-byte token in a narrow smoothly-varying range and
  position 1 is a hard-wired constant `0x00` - real, reproducible,
  edge-channel behavior. Session 4 followed up automatically (stripping
  the 2-byte prefix, re-running the magnitude-threshold and
  continuation-bit sweeps against the remaining `n_tail - 2` channels
  with the correspondingly corrected target count): found only a weak,
  gradual (not sharp-cliff) peak topping out at 27.9% clean, and the
  decoded values under that candidate are indistinguishable from random
  noise (0 of 63 channels physically plausible, per-channel mode
  fractions of only 0.7%-1.9%). The automated sweep infrastructure does
  not find anything here. What remains genuinely untried is fully
  manual, by-hand byte inspection of a handful of individual segments'
  channel-2-onward bytes (not a parameterized sweep) - channel index 2
  onward reverts to the same `0x20`/`0x3f`-leader-byte-dominated bytes
  as the rest of this document's undecoded payload, so this is really a
  narrower, smaller restatement of the core open problem, not a
  shortcut around it, but the much smaller channel count (63-69 instead
  of 256-327) may still make manual inspection more tractable than it
  is against a full segment.
- **(New, from 2026-07-20 session 6) Re-run the exact-`3*npts`-length
  byte-diff technique against longer exact-`3*npts` runs: tried this
  session, same artifact confirmed on two more files, this specific
  avenue is now closed too.** Session 6's manual read of
  `..._12_65...lcd`'s exact-204-byte segments found a handful of
  channels that looked smooth, but all of them turned out to be the
  same already-known low-variance edge channels (indices `0`-`8` and
  `63`-`67`), with a `0.840` smoothness-vs-value-diversity correlation
  across that file's 4-segment run. To check this wasn't an artifact of
  that one short run, this session went on to find and check the two
  longest same-length consecutive runs across all 9 confirmed
  real-signal `MTBLS432` files: `..._26_68...lcd` (6 consecutive
  exact-204-byte segments, indices `2036`-`2041`) and `..._10_26...lcd`
  (another 6-long run, indices `1293`-`1298`). Both reproduce the same
  pattern - `..._26_68...lcd`'s apparently-smooth *interior* channels
  (e.g. channels 9-11, values like `[84, 83, 84, 83, 84, 84]`) turned
  out to have only **2 distinct values** across the 6-segment run, and
  the smoothness-vs-diversity correlation is `0.718` (`..._26_68`) and
  `0.489` (`..._10_26`) - both substantial, both in the same
  artifact-confirming direction as the original file. This rules out
  "the 4-segment run was just too short to see real interior-channel
  structure" as an explanation; longer runs on two independent files
  show the identical low-diversity artifact, not new signal.
- **Decode the simpler, real `LC Raw Data/Chromatogram Ch5`/`Ch6`
  stream instead of (or before) `PDA 3D Raw Data`.** It's populated,
  structurally simpler (one giant segment per channel, not thousands of
  small ones), and a "quiet" channel (`Ch5`) is dominated by a single
  repeating 2-byte value - a much easier starting point than the
  321-wavelength PDA case, and would still deliver real chromatogram
  support even though it's outside this issue's literally-named path.
  Nobody has run any of this document's decode hypotheses against it
  yet.
- **Widen the corpus search specifically for a non-empty `LSS Raw
  Data/Chromatogram Ch*`** - the issue's literally-named target is
  0 bytes in every locally available file. It's untested whether that's
  universal (e.g. `LSS Raw Data` is simply unused/legacy in this
  LabSolutions version) or just a corpus gap (e.g. only populated for
  acquisitions that use a conventional UV/RID detector instead of, or
  alongside, PDA). Worth a targeted fetch pass (PRIDE/MetaboLights/
  MassIVE, searching for older instrument models or explicitly
  UV/RID-detector method descriptions) before assuming `PDA 3D Raw
  Data` is a valid stand-in for the named stream.
- **(Revisited 2026-07-20, still open) Test spectral-domain
  (wavelength-to-wavelength) delta encoding, not just temporal
  (segment-to-segment) delta encoding.** The 2026-07-20 session found
  this idea, taken as a pure token-boundary/framing hypothesis, does not
  actually expand the search space beyond what the continuation-bit and
  magnitude-threshold sweeps already covered exhaustively - delta-vs-
  absolute is a reinterpretation of an already-parsed token's *value*,
  not a new rule for *which bytes form a token*. It remains open and
  useful, but only as a **validator** for a candidate token-boundary
  assignment a future width-selection breakthrough would produce, not
  as an independent decode strategy in its own right. See the
  2026-07-20 session for the full reasoning.
- **(Revisited 2026-07-20, still open) Try IEEE-754 half-precision
  (binary16) floats directly**, not just the word-swapped "PDP-endian"
  full float32 interpretation already explored. A fixed-width fp16
  array is now ruled out directly (real-mode body length equals `2 *
  npts` in only 1 of 3498 real-mode segments checked in one file, 0 of
  2662 in another). Like spectral-domain delta above, fp16 numeric
  interpretation only matters once a token-boundary rule exists to
  apply it to - it doesn't independently help find that rule. See the
  2026-07-20 session.
- **(Attempted 2026-07-20, partially resolved) Characterize the
  `CheckSum` stream (112 bytes) more fully.** Two of the four varying
  fields are now fully identified: offset 56 and offset 88 (each a
  little-endian `u32`, not `u16`) are the exact byte sizes of the `3D
  Raw Data` and `Max Plot` streams respectively, verified with zero
  mismatches across 77 files - this also corrects the previous session's
  reading of offset 58 as a boolean "flat vs. real" flag (it's actually
  the high word of the offset-56 size field). The other two fields
  (offsets 48 and 80) were confirmed genuinely content-dependent (not
  a per-session token) but a sweep of 19 CRC-16 polynomials plus
  Fletcher/Adler/plain-sum checksums against 5 candidate byte ranges
  found no match - the "brute-force small CRC polynomials" plan this
  bullet originally proposed has now been tried against the standard
  polynomial set and come up empty; a future attempt should widen the
  byte-range search (e.g. per-segment checksums) rather than repeat the
  same polynomial sweep. See the 2026-07-20 session for full detail.
- **(Not attempted 2026-07-20, still open) Check whether the
  flat-to-real transition aligns with a fixed elapsed *time* rather than
  a fixed segment *count*.** The 2026-07-20 session did not attempt this
  - there is no existing per-segment retention-time decode for the PDA
  stream in this codebase to cross-reference against (unlike TTFL's RT
  index), so establishing one is itself a prerequisite sub-investigation.
  The transition-segment pass found the index varies (11 or 12) across
  files with the same instrument/method - worth checking the actual
  retention-time value at the transition point (cross-referencing
  whatever RT stream the reader already decodes elsewhere in this repo,
  e.g. the TTFL retention-time index) instead of segment index, in case
  it's a fixed elapsed-time firmware constant (e.g. "N seconds of lamp
  warm-up/blanking before real acquisition starts") rather than a
  coincidence of segment count.
- **(Not attempted 2026-07-20, still open) Use physical plausibility
  (peak shape) as a soft validator, not just exact byte-count
  matching.** The 2026-07-20 session did not attempt this either, for
  the same reason as the fp16/spectral-delta ideas above: it requires a
  candidate token-boundary assignment to validate against, and none of
  the framing searches to date (including this session's CRC sweep and
  fp16/spectral-delta scoping analysis) produced one. The joint-decoder
  pass's core problem was
  a degenerate scoring function admitting many equally-cheap but
  structurally different alignments. Plotting a candidate decode's
  values over time per channel and checking for smooth, single-peaked
  (roughly Gaussian-ish) elution-curve shapes - rather than only
  checking zero-leftover exactness - could help disambiguate between
  otherwise-tied candidate parses, the same way a human would sanity-
  check a chromatogram by eye.
- **(New, from 2026-07-20 session 2) Region-local rather than
  whole-body marker placement for the block-floating-point/marker-bit
  family.** Session 2's item (c) tested a per-token marker bit against
  the whole concatenated body (the `A`+`tail` regions treated as one
  token stream, per session 1's "no vocabulary difference between
  regions" finding). It did not try treating `A` and `tail` as two
  *independently scaled* blocks, each with its own `base_w`/`n_wide`
  derived from its own sub-length rather than the combined body length -
  a real block-floating-point scheme could plausibly reset its
  effective scale at the region boundary even if the byte-level
  vocabulary looks similar on both sides (vocabulary similarity says
  nothing about whether the *width-selection arithmetic* is shared).
  This is a cheap variant of an already-built test (`walk_bfp` from
  session 2) - re-run it with `A` and `tail` split before computing
  `base_w`/`n_wide` for each, rather than merged.
- **(New, from 2026-07-20 session 2) A non-linear or table-driven
  transform from an explicit per-segment header to a threshold.**
  Session 2's item (a) only tried using a candidate header byte
  *directly* as a threshold (or its bitwise complement). A real gain
  header more plausibly indexes a small lookup table (as in ADPCM step
  tables) rather than being usable as a raw magnitude cutoff - this
  wasn't tried since it requires guessing the table itself, which is a
  much larger search than session 2's budget allowed, but is a more
  faithful test of the "explicit per-segment scale field" idea than
  what was actually tried.
