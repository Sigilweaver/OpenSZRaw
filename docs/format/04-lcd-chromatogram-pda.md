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
