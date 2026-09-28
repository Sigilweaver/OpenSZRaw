// Python bindings for the Shimadzu LabSolutions reader.

use std::collections::BTreeMap;
use std::sync::{mpsc, Mutex};

use ::openszraw::reader::Reader;
use openmassspec_core::{
    Activation, Analyzer, ChromatogramRecord, CvTerm, MobilityArrayKind, Polarity, PrecursorInfo,
    RunMetadata, ScanMode, SpectrumRecord, SpectrumSource,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

fn to_py_err(e: ::openszraw::Error) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

fn analyzer_str(a: Analyzer) -> &'static str {
    match a {
        Analyzer::ITMS => "itms",
        Analyzer::TQMS => "tqms",
        Analyzer::SQMS => "sqms",
        Analyzer::TOFMS => "tofms",
        Analyzer::FTMS => "ftms",
        Analyzer::Sector => "sector",
    }
}

fn activation_str(a: Activation) -> &'static str {
    match a {
        Activation::HCD => "hcd",
        Activation::MPID => "mpid",
        Activation::ETD => "etd",
        Activation::CID => "cid",
        Activation::ECD => "ecd",
        Activation::IRMPD => "irmpd",
        Activation::PD => "pd",
        Activation::PQD => "pqd",
        Activation::UVPD => "uvpd",
        Activation::SID => "sid",
        Activation::EThcD => "ethcd",
    }
}

fn polarity_str(p: Polarity) -> &'static str {
    match p {
        Polarity::Positive => "positive",
        Polarity::Negative => "negative",
    }
}

fn scan_mode_str(m: ScanMode) -> &'static str {
    match m {
        ScanMode::Centroid => "centroid",
        ScanMode::Profile => "profile",
    }
}

fn mobility_kind_str(k: MobilityArrayKind) -> &'static str {
    match k {
        MobilityArrayKind::InverseReducedVsPerCm2 => "inverse_reduced_k0",
        MobilityArrayKind::DriftTimeMilliseconds => "drift_time_ms",
    }
}

fn cv_term_dict<'py>(py: Python<'py>, term: &CvTerm) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("accession", term.accession)?;
    d.set_item("name", &term.name)?;
    Ok(d)
}

fn precursor_dict<'py>(py: Python<'py>, p: &PrecursorInfo) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("target_mz", p.target_mz)?;
    d.set_item("selected_mz", p.selected_mz)?;
    d.set_item("isolation_width", p.isolation_width)?;
    d.set_item("charge", p.charge)?;
    d.set_item("intensity", p.intensity)?;
    d.set_item("collision_energy", p.collision_energy)?;
    d.set_item("ce_is_nce", p.ce_is_nce)?;
    d.set_item("precursor_native_id", &p.precursor_native_id)?;
    d.set_item("activation", p.activation.map(activation_str))?;
    d.set_item("analyzer", p.analyzer.map(analyzer_str))?;
    d.set_item("ccs", p.ccs)?;
    Ok(d)
}

fn run_info_dict<'py>(py: Python<'py>, m: &RunMetadata) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("source_file_name", &m.source_file_name)?;
    d.set_item(
        "source_file_format",
        cv_term_dict(py, &m.source_file_format)?,
    )?;
    d.set_item("native_id_format", cv_term_dict(py, &m.native_id_format)?)?;
    d.set_item("instrument", cv_term_dict(py, &m.instrument)?)?;
    d.set_item("instrument_serial_number", &m.instrument_serial_number)?;
    d.set_item("software_name", &m.software_name)?;
    d.set_item("software_version", &m.software_version)?;
    d.set_item("acquisition_software_name", &m.acquisition_software_name)?;
    d.set_item(
        "acquisition_software_version",
        &m.acquisition_software_version,
    )?;
    d.set_item("start_timestamp", &m.start_timestamp)?;
    d.set_item(
        "mobility_array_kind",
        m.mobility_array_kind.map(mobility_kind_str),
    )?;
    d.set_item(
        "analyzers",
        m.analyzers
            .iter()
            .copied()
            .map(analyzer_str)
            .collect::<Vec<_>>(),
    )?;
    d.set_item("extra", &m.extra)?;
    Ok(d)
}

fn chromatograms_to_py<'py>(
    py: Python<'py>,
    records: impl IntoIterator<Item = ChromatogramRecord>,
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for c in records {
        let d = PyDict::new(py);
        d.set_item("index", c.index)?;
        d.set_item("id", c.id)?;
        d.set_item(
            "chromatogram_type",
            c.chromatogram_type
                .as_ref()
                .map(|term| cv_term_dict(py, term))
                .transpose()?,
        )?;
        d.set_item("precursor_mz", c.precursor_mz)?;
        d.set_item("product_mz", c.product_mz)?;
        d.set_item("time_sec", c.time_sec)?;
        d.set_item("intensity", c.intensity)?;
        out.append(d)?;
    }
    Ok(out)
}

/// A complete vendor-neutral spectrum record. Peak arrays remain Python lists
/// for compatibility with the original native binding.
#[pyclass]
pub struct Spectrum {
    rec: SpectrumRecord,
}

#[pymethods]
impl Spectrum {
    #[getter]
    fn index(&self) -> usize {
        self.rec.index
    }
    #[getter]
    fn scan_number(&self) -> u32 {
        self.rec.scan_number
    }
    #[getter]
    fn native_id(&self) -> &str {
        &self.rec.native_id
    }
    #[getter]
    fn ms_level(&self) -> u32 {
        self.rec.ms_level
    }
    #[getter]
    fn polarity(&self) -> Option<&'static str> {
        self.rec.polarity.map(polarity_str)
    }
    #[getter]
    fn scan_mode(&self) -> Option<&'static str> {
        self.rec.scan_mode.map(scan_mode_str)
    }
    #[getter]
    fn analyzer(&self) -> Option<&'static str> {
        self.rec.analyzer.map(analyzer_str)
    }
    #[getter]
    fn acquisition_event_id(&self) -> Option<u32> {
        self.rec.acquisition_event_id
    }
    #[getter]
    fn filter(&self) -> Option<&str> {
        self.rec.filter.as_deref()
    }
    #[getter]
    fn retention_time_sec(&self) -> f64 {
        self.rec.retention_time_sec
    }
    #[getter]
    fn total_ion_current(&self) -> f64 {
        self.rec.effective_tic()
    }
    #[getter]
    fn reported_total_ion_current(&self) -> Option<f64> {
        self.rec.total_ion_current
    }
    #[getter]
    fn base_peak_mz(&self) -> Option<f64> {
        self.rec.effective_base_peak().map(|p| p.0)
    }
    #[getter]
    fn reported_base_peak_mz(&self) -> Option<f64> {
        self.rec.base_peak_mz
    }
    #[getter]
    fn base_peak_intensity(&self) -> Option<f64> {
        self.rec.effective_base_peak().map(|p| p.1)
    }
    #[getter]
    fn reported_base_peak_intensity(&self) -> Option<f64> {
        self.rec.base_peak_intensity
    }
    #[getter]
    fn low_mz(&self) -> Option<f64> {
        self.rec.low_mz
    }
    #[getter]
    fn high_mz(&self) -> Option<f64> {
        self.rec.high_mz
    }
    #[getter]
    fn ion_injection_time_ms(&self) -> Option<f64> {
        self.rec.ion_injection_time_ms
    }
    #[getter]
    fn inv_mobility(&self) -> Option<f64> {
        self.rec.inv_mobility
    }
    #[getter]
    fn faims_cv(&self) -> Option<f64> {
        self.rec.faims_cv
    }
    #[getter]
    fn precursor<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.rec
            .precursor
            .as_ref()
            .map(|p| precursor_dict(py, p))
            .transpose()
    }
    #[getter]
    fn mz(&self) -> Vec<f64> {
        self.rec.mz.clone()
    }
    #[getter]
    fn intensity(&self) -> Vec<f32> {
        self.rec.intensity.clone()
    }
    #[getter]
    fn inv_mobility_per_peak(&self) -> Option<Vec<f32>> {
        self.rec.inv_mobility_per_peak.clone()
    }
    #[getter]
    fn extra(&self) -> BTreeMap<String, String> {
        self.rec.extra.clone()
    }

    fn __len__(&self) -> usize {
        self.rec.mz.len()
    }
    fn __repr__(&self) -> String {
        format!(
            "Spectrum({} peaks, RT {:.2}s)",
            self.rec.mz.len(),
            self.rec.retention_time_sec,
        )
    }
}

#[pyclass]
pub struct RawReader {
    path: String,
    reader: Reader,
    spectra: Vec<SpectrumRecord>,
}

#[pymethods]
impl RawReader {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let mut reader = Reader::open(path).map_err(to_py_err)?;
        let spectra = reader.iter_spectra().collect();
        Ok(Self {
            path: path.to_string(),
            reader,
            spectra,
        })
    }

    #[getter]
    fn scan_count(&self) -> usize {
        self.spectra.len()
    }

    fn read_spectrum(&self, scan_index: usize) -> PyResult<Spectrum> {
        self.spectra
            .get(scan_index)
            .cloned()
            .map(|rec| Spectrum { rec })
            .ok_or_else(|| PyRuntimeError::new_err(format!("scan {scan_index} out of range")))
    }

    fn run_info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        run_info_dict(py, &self.reader.run_metadata())
    }

    /// Detected Shimadzu on-disk variant.
    #[getter]
    fn variant(&self) -> String {
        format!("{:?}", self.reader.variant()).to_ascii_lowercase()
    }

    /// TTFL index-to-m/z calibration, when this is a calibrated IT-TOF file.
    fn calibration<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.reader
            .ttfl_calibration()
            .map(|cal| {
                let out = PyDict::new(py);
                out.set_item("a", cal.a)?;
                out.set_item("b", cal.b)?;
                Ok(out)
            })
            .transpose()
    }

    /// Physical scan or channel-subset index metadata.
    fn scan_index<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let out = PyList::empty(py);
        for item in self.reader.scan_index() {
            let row = PyDict::new(py);
            row.set_item("offset", item.offset)?;
            row.set_item("end_offset", item.end_offset)?;
            row.set_item("retention_time_ms", item.retention_time_ms)?;
            row.set_item("cycle_index", item.cycle_index)?;
            row.set_item("event_id", item.event_id)?;
            row.set_item("entry_index", item.entry_index)?;
            row.set_item("channel_index", item.channel_index)?;
            out.append(row)?;
        }
        Ok(out)
    }

    fn read_chromatograms<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        chromatograms_to_py(py, self.reader.iter_chromatograms())
    }

    fn __repr__(&self) -> String {
        format!("RawReader('{}', {} scans)", self.path, self.spectra.len())
    }
}

enum StreamMsg {
    Record(Box<SpectrumRecord>),
    Done(Result<(), String>),
}

#[pyclass]
struct SpectrumIter {
    rx: Mutex<mpsc::Receiver<StreamMsg>>,
    _handle: std::thread::JoinHandle<()>,
    finished: bool,
}

#[pymethods]
impl SpectrumIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Spectrum>> {
        if slf.finished {
            return Ok(None);
        }
        let rx = &slf.rx;
        let message = py.detach(|| {
            rx.lock()
                .map_err(|_| "iterator mutex poisoned".to_string())?
                .recv()
                .map_err(|e| e.to_string())
        });
        match message {
            Ok(StreamMsg::Record(rec)) => Ok(Some(Spectrum { rec: *rec })),
            Ok(StreamMsg::Done(Ok(()))) => {
                slf.finished = true;
                Ok(None)
            }
            Ok(StreamMsg::Done(Err(e))) | Err(e) => {
                slf.finished = true;
                Err(PyRuntimeError::new_err(e))
            }
        }
    }
}

/// Decode spectra one at a time without buffering the whole acquisition.
#[pyfunction]
fn iter_spectra(py: Python<'_>, path: String) -> PyResult<Py<SpectrumIter>> {
    let (tx, rx) = mpsc::sync_channel(1);
    let handle = std::thread::spawn(move || match Reader::open(&path) {
        Ok(mut reader) => {
            for rec in reader.iter_spectra() {
                if tx.send(StreamMsg::Record(Box::new(rec))).is_err() {
                    return;
                }
            }
            let _ = tx.send(StreamMsg::Done(Ok(())));
        }
        Err(e) => {
            let _ = tx.send(StreamMsg::Done(Err(e.to_string())));
        }
    });
    Py::new(
        py,
        SpectrumIter {
            rx: Mutex::new(rx),
            _handle: handle,
            finished: false,
        },
    )
}

#[pymodule]
fn openszraw(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RawReader>()?;
    m.add_class::<Spectrum>()?;
    m.add_class::<SpectrumIter>()?;
    m.add_function(wrap_pyfunction!(iter_spectra, m)?)?;
    Ok(())
}
