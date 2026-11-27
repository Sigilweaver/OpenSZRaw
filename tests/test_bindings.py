"""Real-file smoke test for the Shimadzu Python field surface."""

from __future__ import annotations

import os
from pathlib import Path

import openszraw
import pytest


def test_decoded_fields():
    path = Path(os.environ.get("OPENSZRAW_TEST_FILE", "corpus/PXD034978_49_27a__8122021_11.qgd"))
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but test file is missing: {path}")
        pytest.skip("set OPENSZRAW_TEST_FILE to a QGD or LCD file")
    reader = openszraw.RawReader(str(path))
    run = reader.run_info()
    assert run["source_file_name"] == path.name
    assert run["extra"]["openszraw.variant"] == reader.variant
    assert reader.variant in {"qgd", "qtfl", "ttfl", "singlequad"}
    assert reader.calibration() is None or isinstance(reader.calibration()["a"], float)
    assert reader.scan_index()
    assert reader.skipped_scans >= 0
    with pytest.raises(IndexError):
        reader.read_spectrum(reader.scan_count)
    first = reader.read_spectrum(0)
    streamed = next(openszraw.iter_spectra(str(path)))
    assert first.native_id == streamed.native_id
    assert len(first.mz) == len(first.intensity)
    for chrom in reader.read_chromatograms():
        assert len(chrom["time_sec"]) == len(chrom["intensity"])
    lenient = openszraw.RawReader(str(path), skip_errors=True)
    assert lenient.scan_count + lenient.skipped_scans >= reader.scan_count
    assert next(openszraw.iter_spectra(str(path), skip_errors=True)).native_id == first.native_id


CORE4_PRECURSOR_KEYS = {
    "target_mz",
    "selected_mz",
    "isolation_width",
    "charge",
    "intensity",
    "collision_energies",
    "precursor_native_id",
    "activation",
    "analyzer",
    "ccs",
}


def test_core4_record_fields():
    """Real-record check of the Core 4 field surface (MRM fixture: MS2 with a precursor)."""
    path = Path(os.environ.get("OPENSZRAW_TEST_FILE", "corpus/PXD034978_49_27a__8122021_11.qgd"))
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but test file is missing: {path}")
        pytest.skip("set OPENSZRAW_TEST_FILE to a QGD or LCD file")
    reader = openszraw.RawReader(str(path))
    spectrum = next(
        (s for s in (reader.read_spectrum(i) for i in range(reader.scan_count)) if s.precursor),
        None,
    )
    if spectrum is None:
        pytest.skip("test file has no spectrum with a precursor")
    # Retention time is a float, or None when the file has none.
    assert isinstance(spectrum.retention_time_sec, float)
    # Analyzer and scan mode are Core's labels ("tqms", "sqms", "tof", "centroid", ...).
    assert spectrum.analyzer in {"itms", "tqms", "sqms", "tof", "ftms", "sector"}
    assert spectrum.scan_mode in {"centroid", "profile"}
    precursor = spectrum.precursor
    assert set(precursor) == CORE4_PRECURSOR_KEYS
    assert "collision_energy" not in precursor and "ce_is_nce" not in precursor
    # The reader does not decode collision energy, so the list is empty (not a guessed value).
    assert precursor["collision_energies"] == []
    assert precursor["target_mz"] is not None
    assert precursor["analyzer"] is None or precursor["analyzer"] in {"tqms", "sqms", "tof"}


def _fixture_path() -> Path:
    path = Path(os.environ.get("OPENSZRAW_TEST_FILE", "corpus/PXD034978_49_27a__8122021_11.qgd"))
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but test file is missing: {path}")
        pytest.skip("set OPENSZRAW_TEST_FILE to a QGD or LCD file")
    return path


def test_spectrum_warnings_field():
    """`warnings` lists what the reader flagged on a spectrum, as code and message."""
    reader = openszraw.RawReader(str(_fixture_path()))
    for i in range(min(reader.scan_count, 50)):
        for warning in reader.read_spectrum(i).warnings:
            assert set(warning) == {"code", "message"}
            assert warning["code"] and warning["message"]
    # A file with no undecodable scans skips nothing, and the stream says so.
    stream = openszraw.iter_spectra(str(_fixture_path()), skip_errors=True)
    assert stream.skipped_scans == 0
    assert sum(1 for _ in stream) == reader.scan_count


def test_skip_errors_counts_and_warns():
    """Scans left out under `skip_errors=True` are counted and reported, never silent.

    Needs a file with undecodable scans (for example an MTBLS14820 QTOF `.lcd`).
    """
    value = os.environ.get("OPENSZRAW_TEST_SKIP_FILE")
    if not value or not Path(value).is_file():
        pytest.skip("set OPENSZRAW_TEST_SKIP_FILE to a file with undecodable scans")
    path = str(value)
    with pytest.raises(RuntimeError):
        list(openszraw.iter_spectra(path))
    with pytest.warns(RuntimeWarning, match="could not be decoded"):
        reader = openszraw.RawReader(path, skip_errors=True)
    assert reader.skipped_scans > 0
    stream = openszraw.iter_spectra(path, skip_errors=True)
    with pytest.warns(RuntimeWarning, match="could not be decoded"):
        kept = sum(1 for _ in stream)
    assert stream.skipped_scans == reader.skipped_scans
    assert kept == reader.scan_count
