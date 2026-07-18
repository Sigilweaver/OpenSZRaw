//! Acquisition start timestamp, recovered from the CFBF container's own
//! per-entry directory metadata rather than any Shimadzu-specific stream.
//!
//! Every OLE2/CFBF directory entry (storage or stream) carries a `created`
//! and `modified` `FILETIME` field per `[MS-CFB]` section 2.6.4. Unlike the
//! standard `\x05SummaryInformation` property set (checked and ruled out
//! for `.lcd`/`.qgd` in `Sigilweaver/OpenSZRaw#9` - Shimadzu's writer never
//! emits that stream), every real corpus file populates these per-entry
//! `created` fields with a real, non-zero value, and LabSolutions writes
//! nearly all of a run's top-level storages within a fraction of a second
//! of each other at run start. The earliest non-zero `created` value across
//! the whole container is therefore a reliable acquisition-start proxy.
//!
//! Verified against corpus-internal evidence only (no vendor software or
//! output), per `CONTRIBUTING.md`'s clean-room rule: across 9 accessions
//! (`.qgd` and all three `.lcd` families, ~150 files), the earliest
//! per-file timestamp tracks sequential injection order with regular,
//! plausible batch cycle times - e.g. `PXD025121`'s 29 sequentially
//! numbered files land ~66m43s apart with only two exact-double-length
//! gaps (an operator break), and `PXD019638`'s 22 files reconstruct a
//! non-alphabetical, 4-way interleaved injection order (`Br0`, `Br1`,
//! `Br2`, `Br3` cycled in turn) purely from timestamps - a pattern that
//! could not fall out of a naive filename-based heuristic. See
//! `docs/format/06-known-limitations.md` for the full writeup.

use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cfb::CompoundFile;

/// The earliest non-zero CFBF directory-entry creation timestamp across the
/// whole container, formatted as RFC 3339 UTC.
///
/// `FILETIME` is defined as UTC per `[MS-DTYP]` 2.3.3, so no timezone
/// inference is needed. An entry with no creation time set decodes (via the
/// `cfb` crate) to a `SystemTime` at the CFBF epoch (1601-01-01), always
/// before the Unix epoch; every real Shimadzu acquisition in the corpus
/// postdates 1970, so filtering on `SystemTime > UNIX_EPOCH` cleanly
/// distinguishes "unset" from "real" without needing to reconstruct the
/// exact zero-sentinel value (which would require platform-specific
/// `checked_sub` arithmetic the `cfb` crate itself avoids for the same
/// reason - see `cfb::internal::timestamp`).
pub fn earliest_created_timestamp<F: Read + std::io::Seek>(
    comp: &mut CompoundFile<F>,
) -> Option<String> {
    comp.walk()
        .map(|entry| entry.created())
        .filter(|t| *t > UNIX_EPOCH)
        .min()
        .map(system_time_to_rfc3339)
}

/// Convert a proleptic Gregorian civil date to days since 1970-01-01.
///
/// Howard Hinnant's `days_from_civil` algorithm
/// (<https://howardhinnant.github.io/date_algorithms.html#days_from_civil>,
/// public domain calendar arithmetic, independent of any vendor source).
/// Duplicated from `opensxraw::raw::summary_info` (same algorithm, same
/// standalone-crate convention - see also `opentfraw::raw_file_info`) rather
/// than pulled from a shared crate, since none of these vendor readers share
/// a common internal-utilities dependency.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn system_time_to_rfc3339(t: SystemTime) -> String {
    let dur = t.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
    let total_secs = dur.as_secs() as i64;
    let millis = dur.subsec_millis();

    let days = total_secs.div_euclid(86_400);
    let sec_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = sec_of_day / 3600;
    let minute = (sec_of_day % 3600) / 60;
    let second = sec_of_day % 60;

    if millis > 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }
}

#[cfg(test)]
mod tests {
    use super::system_time_to_rfc3339;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn formats_known_instant_without_millis() {
        // 2019-06-09T08:08:12Z, matching the MSV000084197 corpus sample's
        // earliest storage creation time.
        let t = UNIX_EPOCH + Duration::from_secs(1_560_067_692);
        assert_eq!(system_time_to_rfc3339(t), "2019-06-09T08:08:12Z");
    }

    #[test]
    fn formats_known_instant_with_millis() {
        let t = UNIX_EPOCH + Duration::from_millis(1_560_067_692_517);
        assert_eq!(system_time_to_rfc3339(t), "2019-06-09T08:08:12.517Z");
    }
}
