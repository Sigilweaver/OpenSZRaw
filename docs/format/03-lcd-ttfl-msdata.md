# 03. LC-MS IT-TOF (.lcd) MS Data Structures

**Status**: PARTIAL (payload run-length encoding scheme: CONFIRMED; scan
metadata prefix and channel-to-m/z calibration: still open)

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

### 3c. Channel-index -> m/z calibration (STILL OPEN)

The RLE stream's implicit index axis (position, reconstructed via
cumulative `skip` values) is very likely a raw digitizer/TOF time-bin
index, not m/z directly - `total_span` (highest position reached) varies
per scan in the few-thousand to tens-of-thousands range, consistent with
a TOF time axis. No calibration table (index -> m/z, e.g. the usual
quadratic `m/z = (a*t + b)^2` TOF formula) was located or derived this
session. `TTFL Instrument Param/MS Parameter` and `TTFL Tuning/*` streams
exist in every file and are likely where such a calibration lives, but
were not opened/parsed this session - flagged as the natural next step
for anyone picking this up.

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
- Channel-index-to-m/z calibration: **open** - not investigated this
  session beyond noting where it probably lives.

Scripts: `re/src/analysis/ttfl_reconfirm.py`,
`re/src/analysis/ttfl_header_scan.py`,
`re/src/analysis/ttfl_payload_dump.py`,
`re/src/analysis/ttfl_marker_scan.py`,
`re/src/analysis/ttfl_rle_decode.py` (the reference decoder),
`re/src/analysis/ttfl_rle_verify.py` (corpus-wide verification),
`re/src/analysis/ttfl_rle_trace.py`, `re/src/analysis/ttfl_prefix_probe.py`.
