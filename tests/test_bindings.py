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
