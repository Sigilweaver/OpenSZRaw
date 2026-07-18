//! Conformance tests against real corpus fixtures, one per on-disk
//! variant. The corpus lives out of tree (`/workspaces/Projects/Data/SZRaw`)
//! and is not checked into this repo, so these tests skip cleanly (rather
//! than failing the build) when it is absent, e.g. on CI.

use openmassspec_core::conformance::assert_source_invariants;
use openmassspec_core::SpectrumSource;
use openszraw::reader::Reader;
use std::path::{Path, PathBuf};

fn qgd_profile_fixture() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SZRaw/PXD019638/L-B2-Br0-1.qgd")
}

fn qgd_mrm_fixture() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SZRaw/PXD034978/49_27a__8122021_11.qgd")
}

fn qtfl_fixture() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SZRaw/MSV000084197/20190607_NM16.lcd")
}

fn ttfl_fixture() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SZRaw/PXD020792/UY02-01-01p400.LCD")
}

fn open_or_skip(path: &Path) -> Option<Reader> {
    if !path.exists() {
        eprintln!("skip: corpus not present at {}", path.display());
        return None;
    }
    Some(Reader::open(path).unwrap_or_else(|e| panic!("Reader::open({path:?}) failed: {e}")))
}

#[test]
fn qgd_profile_conformance() {
    let Some(mut reader) = open_or_skip(&qgd_profile_fixture()) else {
        return;
    };
    let n = assert_source_invariants(&mut reader).expect("conformance invariants failed");
    assert!(n > 0, "expected at least one spectrum");
    println!("qgd profile: {n} spectra");
}

#[test]
fn qgd_profile_peaks_are_plausible() {
    let Some(mut reader) = open_or_skip(&qgd_profile_fixture()) else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());
    assert!(spectra.iter().all(|s| s.ms_level == 1));

    let with_peaks: Vec<_> = spectra.iter().filter(|s| !s.mz.is_empty()).collect();
    assert!(!with_peaks.is_empty(), "expected at least one spectrum with peaks");

    // GC-MS EI full-scan m/z is typically in the tens-to-low-hundreds Da
    // range (this reference file's first scan example in
    // docs/format/02-gcms-qgd-scans.md starts at m/z 100.0); allow up to
    // 2000 Da to cover wider-range methods without being a vacuous check.
    for s in &with_peaks {
        for &mz in &s.mz {
            assert!(
                (1.0..=2000.0).contains(&mz),
                "implausible GC-MS m/z {mz} in spectrum {}",
                s.native_id
            );
        }
    }

    // Retention time must be non-decreasing across the whole run (single
    // linear GC acquisition, no interleaving).
    let mut last_rt = f64::MIN;
    for s in &spectra {
        assert!(
            s.retention_time_sec + 1e-6 >= last_rt,
            "RT regressed: {} -> {}",
            last_rt,
            s.retention_time_sec
        );
        last_rt = s.retention_time_sec;
    }
}

#[test]
fn qgd_mrm_conformance() {
    let Some(mut reader) = open_or_skip(&qgd_mrm_fixture()) else {
        return;
    };
    let n = assert_source_invariants(&mut reader).expect("conformance invariants failed");
    assert!(n > 0, "expected at least one spectrum");
    println!("qgd mrm: {n} spectra");
}

#[test]
fn qgd_mrm_transitions_are_plausible() {
    let Some(mut reader) = open_or_skip(&qgd_mrm_fixture()) else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());
    assert!(spectra.iter().all(|s| s.ms_level == 2));
    assert!(spectra.iter().all(|s| s.mz.len() == 1 && s.intensity.len() == 1));

    for s in &spectra {
        let precursor = s
            .precursor
            .as_ref()
            .unwrap_or_else(|| panic!("MRM spectrum {} missing precursor", s.native_id));
        let precursor_mz = precursor.selected_mz.expect("selected_mz set for MRM");
        assert!(
            (1.0..=2000.0).contains(&precursor_mz),
            "implausible precursor m/z {precursor_mz}"
        );
        let product_mz = s.mz[0];
        assert!(
            (1.0..=2000.0).contains(&product_mz),
            "implausible product m/z {product_mz}"
        );
    }
}

#[test]
fn qtfl_conformance() {
    let Some(mut reader) = open_or_skip(&qtfl_fixture()) else {
        return;
    };
    let n = assert_source_invariants(&mut reader).expect("conformance invariants failed");
    assert!(n > 0, "expected at least one spectrum");
    println!("qtfl: {n} spectra");
}

#[test]
fn qtfl_centroid_mz_is_plausible_and_has_ms2() {
    let Some(mut reader) = open_or_skip(&qtfl_fixture()) else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());

    let has_ms1 = spectra.iter().any(|s| s.ms_level == 1);
    let has_ms2 = spectra.iter().any(|s| s.ms_level == 2);
    assert!(has_ms1, "expected at least one MS1 (event_id==1) scan");
    assert!(has_ms2, "expected at least one MS2 (event_id>1) scan - see docs/format/05 addendum");

    let with_peaks: Vec<_> = spectra.iter().filter(|s| !s.mz.is_empty()).collect();
    assert!(!with_peaks.is_empty());
    for s in &with_peaks {
        for &mz in &s.mz {
            assert!(
                (50.0..=5000.0).contains(&mz),
                "implausible QTOF m/z {mz} in spectrum {}",
                s.native_id
            );
        }
    }

    // Every MS2 spectrum must carry a precursor reference (even though we
    // do not decode the real precursor m/z this session - see
    // docs/format/06-known-limitations.md).
    for s in spectra.iter().filter(|s| s.ms_level == 2) {
        assert!(
            s.precursor
                .as_ref()
                .is_some_and(|p| p.precursor_native_id.is_some()),
            "MS2 spectrum {} missing precursor_native_id",
            s.native_id
        );
    }
}

#[test]
fn ttfl_conformance() {
    let Some(mut reader) = open_or_skip(&ttfl_fixture()) else {
        return;
    };
    let n = assert_source_invariants(&mut reader).expect("conformance invariants failed");
    assert!(n > 0, "expected at least one spectrum");
    println!("ttfl: {n} spectra");
}

#[test]
fn ttfl_index_axis_is_uncalibrated_but_bounded() {
    let Some(mut reader) = open_or_skip(&ttfl_fixture()) else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());
    assert!(spectra.iter().all(|s| s.ms_level == 1));

    let with_peaks: Vec<_> = spectra.iter().filter(|s| !s.mz.is_empty()).collect();
    assert!(!with_peaks.is_empty());

    // The "mz" field here is a raw digitizer/time-bin index (see
    // docs/format/06-known-limitations.md), not m/z - it should still be
    // non-negative and bounded. docs/format/03-lcd-ttfl-msdata.md
    // characterizes the range as "few-thousand to tens-of-thousands,"
    // but this session found real scans in this exact fixture reaching
    // ~576,000 (see docs/format/06-known-limitations.md addendum), so
    // this bound is deliberately generous rather than reproducing the
    // doc's undersold estimate.
    for s in &with_peaks {
        for &idx in &s.mz {
            assert!(idx >= 0.0, "negative time-bin index {idx}");
            assert!(idx < 2_000_000.0, "implausibly large time-bin index {idx}");
        }
    }
}
