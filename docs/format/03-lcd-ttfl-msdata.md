# 03. LC-MS IT-TOF (.lcd) MS Data Structures

**Status**: PARTIAL (payload run-length encoding scheme: CONFIRMED;
index-to-m/z calibration: CONFIRMED, see section 3c; scan metadata
prefix: still open)

The `TTFL Raw Data` directory in IT-TOF `.lcd` files contains the primary
mass spectrometry raw data (e.g., MTBLS432, PXD020792). It uses a
sophisticated indexing scheme to handle interleaved acquisition modes
(e.g., rapidly alternating between positive and negative ionization, or
MS1/MS2).

## 1. Retention Time and Data Index Streams

Unchanged from the prior write-up, re-confirmed independently this
session against fresh corpus reads (`TTFL Raw Data/Retention Time` is a
plain `u32[N_RT]` array in milliseconds, `TTFL Raw Data/Data Index` is
`N_RT * 64` bytes, one 64-byte entry per RT point, each split into four
16-byte subsets).

### 16-byte subset layout (repeated 4 times per 64-byte entry)

| Offset | Type | Description |
|--------|------|-------------|
| 0x00   | u32  | Absolute byte offset into `MS Raw Data` for this event's scan. |
| 0x04   | u32  | Always 0 in every file/entry checked this session. |
| 0x08   | u32  | RT entry index (0-based), i.e. matches the position of this 64-byte entry in the Data Index. |
| 0x0C   | u32  | Global event counter, incrementing by exactly 1 for every valid (non-sentinel) subset in file order across the *whole* Data Index (verified: `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` has 6840 valid subsets with evctr running 0..6839, step-1 100% of the time). `0xFFFFFFFF` in entry 0 only (see below). |

`sub_i` (the subset's position 0-3 within its 64-byte entry) is a fixed
per-channel slot across the entire file - verified `Counter` over the
whole Data Index in `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` shows
sub position 0/1/2/3 each occur exactly once per valid entry, 1710 times
each (out of 1711 RT points; entry 0 is a special case, see below).

**Entry 0 correction**: entry 0's four subsets all carry `evctr =
0xFFFFFFFF`, which the prior doc described as "unused/sentinel." That
wording was misleading - **entry 0's scans are real, decodable data**
(e.g. offsets 0/1842/2560/2922 in
`6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` point to real payload with
103 real peaks after decoding, see Section 3). `0xFFFFFFFF` simply marks
"no previous global-counter value to continue from" for the very first
RT point, not "no data."

## 2. MS Raw Data Scan Header (64 bytes)

Re-derived from scratch this session (all 16 `u32` fields dumped and
cross-checked across dozens of consecutive scans per channel, in 2
different files). Indices below are `u32[0..15]` (byte offset = index*4):

| Field | Meaning | Evidence |
|---|---|---|
| `u32[0]` | Global scan sequence number across the whole `MS Raw Data` stream (0-based, counts entry-0's 4 scans too - so it runs 4 higher than `evctr` from the Data Index). | Increments by 1 per scan in file order, verified over 30 consecutive scans. |
| `u32[1]` | RT entry index (1-based here, i.e. `entry_i + 1`... actually matches `entry_i` for the very first appearance - see note). | Matches Data Index `subidx` pattern. |
| `u32[2]` | **Retention time in milliseconds - an exact copy of the `Retention Time` stream value for this RT point.** | Verified byte-for-byte: RT stream for `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd` is `0, 1000, 2000, ...`; `u32[2]` for every scan at RT entry N is exactly `N*1000` in that file, and exactly matches the (non-uniform) `Retention Time` stream values in `PXD020792/UY01-03-01p95.LCD` (`0, 11000, 21900, 32700, ...`). |
| `u32[3]` | A precise per-event timestamp: `RT_ms + <per-channel offset>`. **This is NOT a peak/data counter** - see correction below. |
| `u32[4]` | The channel/subset index (`sub_i`, 0-3), confirming which of the 4 interleaved event slots this scan belongs to. |
| `u32[5]` | Constant `0x10000` or `0x10001` depending on channel (bit 0 flips between channel pairs 0/1 vs 2/3 - looks like a coarse mode/polarity flag, not investigated further). |
| `u32[6]` | Constant `0xFFFF0000` in every scan checked. |
| `u32[7]` | `((entry_i+3) << 16) | 0xFFFF` - a deterministic function of the RT entry index, not scan data. |
| `u32[8]` | Constant `0`. |
| `u32[9]` | Constant `0xFFFFFFFF` (sentinel). |
| `u32[10]` | Duplicate of `u32[0]`. |
| `u32[11]` | `entry_i + 1`. |
| `u32[12]` | Constant `2`. |
| `u32[13]` | Constant `0`. |
| `u32[14]` | Constant `0x1900000` (26214400). |
| `u32[15]` | Constant `0x1911238` (26280312) - looks like a format/version tag. |

### Correction to the previous PARTIAL doc: `u32[3]` is not a peak counter

The previous version of this doc hypothesized that `u32[2]`/`u32[3]`
were "cumulative indices or counters" correlating with per-scan data
amount. Fresh, direct measurement this session **disproves that**:

- In `6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`, `u32[3] - u32[2]`
  (the "diff") is **exactly constant per channel across the entire
  30-minute run**: channel 0 -> 100, channel 1 -> 297, channel 2 -> 592,
  channel 3 -> 789, unchanged across all 1711 RT points checked, while
  the actual payload byte length for those same scans varies wildly
  (e.g. channel 1's payload ranges from 438 to 4604 bytes across the
  first 30 RT points alone). A real per-scan peak count cannot be a file
  constant while the encoded data size varies 10x.
- In `PXD020792/UY01-03-01p95.LCD` (a different acquisition method with
  irregular RT spacing and data-dependent MS/MS), the diff is **not even
  constant** - e.g. channel 0 gives diff values of 100, 0, 0, 0 across
  the first 4 RT points while that channel's payload is a near-constant
  ~22-25 KB each time (i.e. diff=0 alongside tens of thousands of bytes
  of real, decodable peak data - conclusively ruling out "diff = peak
  count").
- The diff values (100 < 297 < 592 < 789 in the MTBLS432 file) are
  monotonically increasing in channel order and land within one nominal
  RT cycle (1000 ms in that file) - consistent with `u32[3]` actually
  being **a precise sub-cycle acquisition timestamp** (this channel fired
  at +100 ms into the RT bin, that one at +297 ms, etc.), i.e. a
  finer-grained clock reading than the RT stream's rounded value. This
  is a hypothesis, not fully proven, but it is what the evidence points
  to; what's proven is simply that it is **not** a data/peak count.

The true per-scan peak count is only recoverable by fully decoding the
payload (Section 3) - it is not stored as a plain integer anywhere in
the 64-byte header.

## 3. MS Raw Data Payload: CONFIRMED run-length-encoded sparse profile

Every scan payload (`MS Raw Data` bytes after the 64-byte header) splits
into two parts:

```
[ scan metadata prefix, variable length, undecoded ] [ RLE profile stream ]
```

### 3a. RLE profile stream (CONFIRMED)

The tail of every payload is a run-length-encoded sparse array of 16-bit
intensity samples over an implicit index axis (very likely raw
digitizer/TOF channel number - see open question below about mapping
this to m/z). Reading as little-endian `u16` words:

```
repeat:
    marker := u16          # 0x8000 | run_length
    if marker == 0x8000:   # run_length == 0
        STOP               # terminator, stream ends here
    run_length := marker & 0x7FFF
    skip := u16             # number of implicit zero-intensity samples
                             # to skip before this run starts
    values[run_length] := u16 x run_length   # raw intensity samples
```

Reconstructing (position, intensity) pairs:
```python
pos = 0
peaks = []
for skip, values in runs:
    pos += skip
    for v in values:
        peaks.append((pos, v))
        pos += 1
```

**Verification methodology and results**: implemented in
`re/src/analysis/ttfl_rle_decode.py` (`decode_rle`) and
`re/src/analysis/ttfl_rle_verify.py`. The decoder was run over **every
scan in every `.lcd` file locally available for both IT-TOF accessions**:

| Source | Files | Scans checked | Clean decodes (ok, zero leftover bytes) |
|---|---|---|---|
| MetaboLights MTBLS432 | 15 (all local files) | 102,660 | 102,660 (100%) |
| PRIDE PXD020792 | 5 (all local files) | 6,676 | 6,676 (100%) |
| **Total** | **20** | **109,336** | **109,336 (100%)** |

"Clean" means: starting from the correct prefix/RLE boundary (found by
trying every 2-byte-aligned candidate marker position and requiring the
decode to consume the *entire* remainder of the payload with the
terminator landing exactly on the last word - see `find_prefix_end`),
the decoder consumes every single byte of the payload tail with **zero
leftover bytes and no parse errors**, across scans ranging from ~200
bytes to >25,000 bytes and from ~10 to >500 peaks. This was checked
across all four interleaved channels (`sub_i` 0-3) in every file.

**Canonical worked example** (the scan referenced in earlier
project notes as "297 peaks in 654 bytes" - that peak count was a
mis-assumption based on the now-corrected `u32[3]` header field, see
Section 2; the true, independently-decoded peak count for this scan is
**103**, not 297):

- File: `MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`
- RT entry 0, channel (`sub_i`) 1, `MS Raw Data` offset 1842, scan size
  718 bytes = 64-byte header + 654-byte payload.
- Payload splits into a 194-byte metadata prefix (hex
  `a601 6a02 9600 0000 c201 ffff 0400 fb08 ... 4900`) and a 460-byte
  (230-word) RLE tail.
- RLE tail decodes to exactly **63 runs, 103 total peak values**, ending
  in the terminator word `0x8000` (final tail bytes: `... 01 80 87 00
  a4 2f 00 80`, i.e. one last real run then `00 80` = `0x8000`).
- First run: marker `0x8002` (`02 80`) -> run_length 2, skip word `0107`
  LE = 263 (`07 01`), then 2 raw values `0121`=289 and `0043`=67 (`21 01
  43 00`). Reconstructed: position 263 -> intensity 289, position 264 ->
  intensity 67.
- Zero leftover bytes; `end_idx == nwords` exactly (230/230 words
  consumed).

This structure directly explains the previously-observed byte pattern
("long runs of zero bytes punctuated by short non-zero runs of 1, 3, or
5 bytes"): those short non-zero runs *are* the `marker+skip+values`
groups (a run of `N` intensity values is `2*(N+2)` bytes: 2 for the
marker, 2 for skip, `2*N` for the values - e.g. a 1-value run is 6
bytes, matching the "5-byte-ish" runs seen once you allow for the
adjacent zero byte inside a `skip`/value being > 255), and the "long
zero runs" are exactly where a large `skip` value (a wide gap between
non-zero samples) got zero-padded on the byte grid.

Both previously-rejected hypotheses (plain 7-bit LEB128, and a fixed
record width) were correctly rejected - this is neither; it's a
run-length scheme with a 16-bit in-band marker (high bit set means
"marker/run-length", not a value), which is a different and simpler
mechanism than LEB128 continuation bits.

### 3b. Scan metadata prefix (STILL OPEN)

The bytes before the RLE tail (194 bytes in the great majority of
MTBLS432 scans; ranges up to ~1550 bytes in some PXD020792 scans,
likely because of extra variable-length MS/MS precursor metadata on
those channels) are not decoded. Partial observations, not verified
claims:

- A run of what look like fixed calibration-style coefficients
  (identical across every scan of a given channel in a given file:
  e.g. bytes `fb ff 76 fd 60 f0 06 ff dd ff` interpreted as signed
  `i16` gives `-5, -650, -4000, -249, -35` - plausibly a fixed TOF
  calibration polynomial, not re-derived or confirmed).
- A `u32` field roughly proportional to the eventual peak count with a
  ratio around ~0.43-0.46x (e.g. field value 231 for a 103-peak scan,
  228 for a 102-peak scan, 332 for a 144-peak scan) - close to but not
  exactly matching any simple integer multiple of peak count or run
  count; not resolved.
- Attempted to cross-check against `TTFL Raw Data/Sum TIC Data` (16
  bytes/RT-point, decodes as 4x `u32`, presumably one per channel) to
  see if any prefix field or the RLE-decoded intensity sum matched a
  known TIC value - inconclusive; the `Sum TIC Data` values didn't
  arithmetically match the decoded per-scan intensity sums in any
  straightforward way tried (no scaling factor found). Left open.

### 3c. Channel-index -> m/z calibration (CONFIRMED)

Resolves Sigilweaver/OpenSZRaw#1. The RLE stream's implicit index axis
(position, reconstructed via cumulative `skip` values) is a raw
digitizer/TOF time-bin index, and this session located and verified the
file's own embedded calibration that converts it to physical m/z.
Implemented as `raw::ttfl::Calibration` / `raw::ttfl::parse_calibration`
in `crates/openszraw`; see that module's doc comment for the
implementation-level summary. This section records the full derivation
and evidence.

**Where the calibration lives**: `TTFL Tuning/Tuning Result 00` (and two
byte-identical copies, `01` and `02`) contains two small fixed-offset
tables:

- Up to 9 reference calibrant masses, as little-endian `u32` at byte
  offsets `3022, 3026, 3030, ...` (stride 4), scaled by `1e-4` (a
  fixed-point convention also seen elsewhere in this stream), zero-padded
  after the last real entry.
- A matching count of measured flight times, as little-endian `f64` at
  byte offsets `3150, 3158, 3166, ...` (stride 8).

**Identifying the masses**: the mass ladder's pairwise spacing is an
exact integer multiple of 67.9874 Da in every file checked. 67.9874 Da is
the monoisotopic mass of `HCOONa` (computed here from public atomic mass
constants - C 12.0000, H 1.007825, O 15.994915, Na 22.989770 - not from
any vendor reference), i.e. the mass ladder is a **sodium formate cluster
ion series**, `[Na(HCOONa)n]+`. This is a standard, publicly documented
ESI calibration solution used industry-wide, not proprietary Shimadzu
data - its presence here is expected of any ESI instrument's tune
records.

**Fitting the calibration**: the standard linear TOF flight-time law,
`time = a*sqrt(mz) + b` (basic reflectron/linear TOF physics, not
vendor-specific), fit by least squares against the paired
(mass, time) table, gives a residual at the level of floating-point
round-trip noise. Worked example from
`MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd`'s
`TTFL Tuning/Tuning Result 00`:

| mass (Da) | time  | fit residual |
|-----------|-------|--------------|
| 1589.64   | 21908.68359375 | +0.020 |
| 2949.388  | 29806.38671875 | -0.037 |
| 4309.136  | 36007.29296875 | +0.072 |
| 5668.885  | 41284.74218750 | -0.053 |
| 7028.633  | 45958.96484375 | -0.025 |
| 8388.381  | 50198.98046875 | +0.005 |
| 9748.129  | 54107.10156250 | -0.012 |
| 11107.877 | 57750.94140625 | +0.011 |
| 12467.625 | 61177.76953125 | +0.019 |

giving `a = 547.012885`, `b = 99.101660` (max residual 0.072 out of a
~20,000-61,000 range, i.e. ~1e-6 relative). Inverting gives
`mz = ((index - b) / a)^2`, applied directly to the RLE payload's raw
index with no assumed origin shift.

Checked this way across every locally available IT-TOF `.lcd` file (81
files: 45 MTBLS432, 5 PXD020792, 31 PXD025121) - every file's own mass/time
table fits this model to the same ~1e-6-relative-residual precision, and
there are only **4 distinct `(a, b)` pairs across the whole corpus**
(consistent with a handful of real tuning sessions covering batches of
files, not coincidence or a hardcoded constant): `(547.0129, 99.1017)`
for MTBLS432, `(667.2112, 145.5528)` for PXD025121, and two very close
values `(652.8886, 93.6675)` / `(652.8939, 93.5880)` within PXD020792
(likely a re-tune partway through that batch's acquisition).

**Why this is believed to also calibrate the RLE payload's index axis**
(the part that could not be checked against vendor ground truth, per
this project's clean-room policy - only against independent, internal,
and public-chemistry evidence):

1. **Order of magnitude**: the tuning "time" ladder spans roughly
   20,000-95,000 across the corpus; real scan index ranges reach the same
   order of magnitude for genuine ion signal (see the noise-tail caveat
   below for the much larger index values seen in low-intensity
   background).
2. **Plausible chemistry**: applying the fit to real decoded scan peaks
   yields small-molecule/metabolite-range m/z (tens to low thousands of
   Da) for the large majority of real signal in these LC-MS metabolomics
   runs.
3. **Recurrence of a known calibrant background ion at a stable,
   predicted index position**: searching real scan data (independent of
   any vendor tool) for the theoretical sodium formate cluster masses -
   at the index position `Calibration::mz`'s inverse predicts for each -
   finds them recurring at a **tightly clustered index position (within
   ~7-8 raw index units, sub-0.05 Da) across dozens of scans spanning an
   entire 30-minute run**, in both MTBLS432 and PXD025121 files with
   their own distinct `(a, b)`. In the MTBLS432 file this recurrence is
   also concentrated almost entirely in channels `sub_i` 0/1 and nearly
   absent from `sub_i` 2/3 (e.g. m/z 974.8129: 30/1711 and 44/1711 scans
   respectively in channels 0/1, vs. 0/1711 and 5/1711 in channels 2/3) -
   exactly the pattern expected if `sub_i` 0/1 are positive polarity
   (where a `+1` sodium cluster cation is expected) and 2/3 are negative
   (where it should not appear), independently corroborating the
   channel-pairing noted in `docs/format/06-known-limitations.md` section
   2 as a side effect, though that channel-to-polarity mapping itself
   remains otherwise unconfirmed and out of scope here.
4. A **density-based background check does not discriminate** (searching
   for an arbitrary mass 0.4 Da off the real series finds peaks nearby
   almost as often, ~17-18 average hits per 400 scans either way) - real
   LC-MS metabolomics data is chemically dense enough that "is there a
   peak near this index" alone proves little. The clustering test above
   (point 3) is the one that does discriminate, because it checks
   *positional stability across many independent scans*, not just
   presence.
5. A **mass-defect distribution check across all decoded points was
   uninformative** (flat/uniform, even restricted to each scan's single
   most intense point) - the raw RLE-decoded stream carries a large
   amount of low-level noise/ringing across the whole time-bin axis
   (unrelated to real ion chemistry), which swamped this test. This is
   noted as a dead end, not a red flag against the calibration: see the
   noise-tail caveat below, which independently explains why bulk
   statistics over *all* points are not expected to look clean.

**Known limitation carried over, not resolved by this work**: this
confirms the calibration's functional form and per-file constants to
very high confidence, and confirms it correctly localizes real ion
signal, but does not independently rule out a small, constant index
origin offset between the RLE payload's index convention (cumulative
`skip` from 0 at the start of a scan's own RLE stream) and the tuning
stream's own time convention - see
`docs/format/06-known-limitations.md` section 1.

**Noise-tail caveat**: applying the calibration to the full, unfiltered
RLE index axis (as opposed to only real, high-intensity ion peaks) can
yield very large m/z - e.g. the extreme index of 576,297 documented in
section 3c's earlier revision (`PXD020792/UY02-01-01p400.LCD`) maps to
roughly 780,000 Da under that file's own calibration. This does not
indicate a broken calibration; it indicates that such extreme index
positions are electronic noise, not real ions (no IT-TOF instrument
measures m/z in that range) - filtering/peak-picking such noise is
downstream work this crate does not attempt, matching its existing
policy of exposing the fully-decoded raw signal rather than silently
dropping data.

## Summary of status

- Data Index / 64-byte scan header field layout: **CONFIRMED** (re-derived
  and independently verified against 2 files this session; corrects the
  prior doc's mistaken "cumulative peak counter" reading of `u32[3]`).
- RLE payload structure (marker/skip/values, 16-bit words, `0x8000`
  terminator): **CONFIRMED** - decodes byte-exact with zero leftover
  across 109,336 real scans in all 20 locally available IT-TOF `.lcd`
  files from 2 different accessions (MetaboLights MTBLS432, PRIDE
  PXD020792).
- Scan metadata prefix content: **PARTIAL/open** - boundary reliably
  located, byte layout not decoded.
- Channel-index-to-m/z calibration: **CONFIRMED** - see section 3c above.

Scripts: `re/src/analysis/ttfl_reconfirm.py`,
`re/src/analysis/ttfl_header_scan.py`,
`re/src/analysis/ttfl_payload_dump.py`,
`re/src/analysis/ttfl_marker_scan.py`,
`re/src/analysis/ttfl_rle_decode.py` (the reference decoder),
`re/src/analysis/ttfl_rle_verify.py` (corpus-wide verification),
`re/src/analysis/ttfl_rle_trace.py`, `re/src/analysis/ttfl_prefix_probe.py`.

## Addendum (Phase 4 implementation session): `entry_i` must be read from the subset's own field, and a trailing partial block is real

Section 1 states the subset's `u32[2]` "RT entry index (0-based)...
matches the position of this 64-byte entry in the Data Index." That
equivalence **does not hold on every file**. Verified against
`PXD025121/17.lcd` (a different accession from the MTBLS432 file this
doc's Section 1 was checked against): this file's acquisition has only
2 real interleaved channels per RT point (not 4), so each physical
64-byte block packs *two* RT points' worth of subsets - block 0's four
subsets carry `entry_i` values `[0, 0, 1, 1]`, block 1 carries
`[2, 2, 3, 3]`, and so on. A reader that assumes `entry_i == block
position` (as Section 1's wording invites) silently assigns the wrong
retention time to every other RT point's spectra on this class of file.
`crates/openszraw::raw::ttfl::parse_data_index` reads `entry_i` from
each subset's own bytes rather than from its position, which is correct
on both the 4-real-channel and 2-real-channel cases.

Relatedly, the `Data Index` stream is not always an exact multiple of
64 bytes: 9 files in `PXD025121` have a **trailing partial block** (32
or 48 bytes - 1-3 leftover subsets) when the total real-subset count
isn't a multiple of 4, e.g. `PXD025121/17.lcd` has 657 RT points x 2
real channels = 1314 subsets = 328 full blocks + 1 trailing 32-byte
block (`21024` bytes total, confirmed by reading the tail subsets: real,
in-bounds offsets, `entry_i = 656` = the last RT index). Rejecting
anything not an exact multiple of 64 fails to open all 9 of these real
corpus files; the parser now accepts a trailing block whose size is any
positive multiple of 16.

## Addendum (Phase 4 implementation session): `total_span` magnitude estimate was too low

Section 3c above estimates `total_span` as running "in the few-thousand
to tens-of-thousands range." Rust-side corpus verification this session
found real scans in `PXD020792/UY02-01-01p400.LCD` reaching a
reconstructed time-bin index of **576,297** - well past "tens of
thousands." The RLE decode itself is unaffected (it does not depend on
any assumed upper bound), but `crates/openszraw` does not clamp or
validate the index axis against this doc's estimate for exactly that
reason. See `docs/format/06-known-limitations.md` section 1.
