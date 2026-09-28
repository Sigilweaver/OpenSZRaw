"""Real-file smoke test for the Shimadzu Python field surface."""

from __future__ import annotations

import os
from pathlib import Path

import openszraw
import pytest


def test_decoded_fields():
    path = Path(os.environ.get("OPENSZRAW_TEST_FILE", "corpus/PXD034978_49_27a__8122021_11.qgd"))
    if not path.is_file():
        pytest.skip("set OPENSZRAW_TEST_FILE to a QGD or LCD file")
    reader = openszraw.RawReader(str(path))
    run = reader.run_info()
    assert run["source_file_name"] == path.name
    assert run["extra"]["openszraw.variant"] == reader.variant
    assert reader.variant in {"qgd", "qtfl", "ttfl", "singlequad"}
    assert reader.calibration() is None or isinstance(reader.calibration()["a"], float)
    assert reader.scan_index()
    first = reader.read_spectrum(0)
    streamed = next(openszraw.iter_spectra(str(path)))
    assert first.native_id == streamed.native_id
    assert len(first.mz) == len(first.intensity)
    for chrom in reader.read_chromatograms():
        assert len(chrom["time_sec"]) == len(chrom["intensity"])
