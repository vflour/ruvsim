#![allow(unsafe_op_in_unsafe_fn)]

use std::str::FromStr;
use std::{cell::RefCell, path::Path};

use pyo3::{Bound, exceptions::PyRuntimeError, prelude::*, types::PyModule};
use regex::Regex;

use crate::sim_compiler::{Compiler as RsCompiler, CompilerCommand};
use crate::sim_runner::Runner;
use crate::sim_types::{
    ParsedLine, SimBreakpoint, SimDriver, SimMemory, SimRadix, SimSignal, SimSignalDirection,
    SimTimeUnit,
};

#[pyclass]
#[derive(Clone)]
pub struct PySignal {
    inner: RefCell<SimSignal>,
}

#[pymethods]
impl PySignal {
    #[getter]
    pub fn name(&self) -> String {
        self.inner.borrow().name.clone()
    }
    #[getter]
    pub fn value(&self) -> String {
        self.inner
            .borrow()
            .value
            .clone()
            .unwrap_or((SimRadix::Binary, "X".to_string()))
            .1
    }
    #[getter]
    pub fn radix(&self) -> String {
        match self.inner.borrow().value {
            Some((radix, _)) => format!("{:?}", radix),
            None => "Unknown".to_string(),
        }
    }
    #[getter]
    pub fn drivers(&self) -> Vec<PyDriver> {
        self.inner
            .borrow()
            .drivers
            .clone()
            .into_iter()
            .map(|d| PyDriver { inner: d })
            .collect()
    }
    #[getter]
    pub fn direction(&self) -> String {
        match self.inner.borrow().direction {
            SimSignalDirection::Input => "Input".to_string(),
            SimSignalDirection::Output => "Output".to_string(),
            SimSignalDirection::Inout => "Inout".to_string(),
            SimSignalDirection::Unknown => "Unknown".to_string(),
            SimSignalDirection::None => "".to_string(),
        }
    }
    #[getter]
    pub fn left_bound(&self) -> i32 {
        self.inner.borrow().bounds.left
    }
    #[getter]
    pub fn right_bound(&self) -> i32 {
        self.inner.borrow().bounds.right
    }
    pub fn numeric_value(&self) -> Option<u64> {
        self.inner.borrow().get_numeric_value()
    }
}

#[pyclass]
pub struct PyDriver {
    inner: SimDriver,
}

#[pymethods]
impl PyDriver {
    #[getter]
    pub fn driver_type(&self) -> String {
        format!("{:?}", self.inner.driver_type)
    }
    #[getter]
    pub fn source(&self) -> String {
        self.inner.source.clone()
    }
}

#[pyclass]
pub struct PyBreakpoint {
    inner: SimBreakpoint,
}

#[pymethods]
impl PyBreakpoint {
    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    pub fn file(&self) -> String {
        self.inner.file.to_string_lossy().into_owned()
    }
    #[getter]
    pub fn line_num(&self) -> u32 {
        self.inner.line_num
    }
}

#[pyclass]
pub struct PyMemory {
    inner: SimMemory,
}

#[pymethods]
impl PyMemory {
    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    pub fn left_bound(&self) -> i32 {
        self.inner.size.left
    }
    #[getter]
    pub fn right_bound(&self) -> i32 {
        self.inner.size.right
    }
}

#[pyclass]
pub struct PyParsedLine {
    inner: ParsedLine,
}

#[pymethods]
impl PyParsedLine {
    #[getter]
    pub fn content(&self) -> String {
        self.inner.content.clone()
    }
    #[getter]
    pub fn line_type(&self) -> String {
        format!("{:?}", self.inner.line_type)
    }
}

fn parse_radix(radix: &str) -> PyResult<SimRadix> {
    match radix.to_lowercase().as_str() {
        "binary" | "bin" => Ok(SimRadix::Binary),
        "octal" | "oct" => Ok(SimRadix::Octal),
        "decimal" | "dec" => Ok(SimRadix::Decimal),
        "hex" | "hexadecimal" => Ok(SimRadix::Hexadecimal),
        "unsigned" | "uns" => Ok(SimRadix::Unsigned),
        _ => Err(pyo3::exceptions::PyValueError::new_err("invalid radix")),
    }
}

fn parse_direction(dir: &str) -> PyResult<SimSignalDirection> {
    match dir.to_lowercase().as_str() {
        "in" | "input" => Ok(SimSignalDirection::Input),
        "out" | "output" => Ok(SimSignalDirection::Output),
        "inout" => Ok(SimSignalDirection::Inout),
        "" => Ok(SimSignalDirection::None),
        _ => Err(pyo3::exceptions::PyValueError::new_err("invalid direction")),
    }
}

#[pyclass]
pub struct PyRunner {
    inner: std::cell::RefCell<Runner>,
}

#[pymethods]
impl PyRunner {
    #[new]
    fn new(vsim_command: String, args: Vec<String>, cwd: String) -> PyResult<Self> {
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let runner = Runner::new(&vsim_command, &arg_refs, &cwd);
        Ok(Self {
            inner: std::cell::RefCell::new(runner),
        })
    }

    fn wait_until_prompt_ms(&self, timeout_ms: u64) -> PyResult<()> {
        use std::time::Duration;
        self.inner
            .borrow_mut()
            .wait_until_prompt(Duration::from_millis(timeout_ms))
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn wait_until_prompt_startup(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .wait_until_prompt_startup()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn send_command(&self, command: String) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .send_command(&command)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn send_command_no_wait(&self, command: String) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .send_command_no_wait(&command)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn run_for(&self, time: u64, unit: String) -> PyResult<()> {
        let sim_unit = SimTimeUnit::from_str(&unit).map_err(|e| PyRuntimeError::new_err(e))?;
        self.inner
            .borrow_mut()
            .run_for(time, sim_unit)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn run_all(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .run_all()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn run_next(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .run_next()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn run_continue(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .run_continue()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn finish(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .finish()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn get_time(&self) -> PyResult<String> {
        self.inner
            .borrow_mut()
            .get_time()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn get_status(&self) -> PyResult<String> {
        self.inner
            .borrow_mut()
            .get_status()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    #[pyo3(signature = (path, direction=None))]
    fn get_signal(&self, path: &str, direction: Option<String>) -> PyResult<Option<PySignal>> {
        Ok(self.get_signals(Some(path), direction)?.first().cloned())
    }

    #[pyo3(signature = (path=None, direction=None))]
    fn get_signals(
        &self,
        path: Option<&str>,
        direction: Option<String>,
    ) -> PyResult<Vec<PySignal>> {
        let p = path.unwrap_or_else(|| "");
        let dir = match direction {
            Some(d) => Some(parse_direction(&d)?),
            None => None,
        };

        self.inner
            .borrow_mut()
            .get_signals(&p, dir)
            .map(|signals| {
                signals
                    .into_iter()
                    .map(|n| PySignal {
                        inner: RefCell::new(n),
                    })
                    .collect()
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn examine_signal(&self, signal: &PySignal, radix: String) -> PyResult<()> {
        let mut sig = signal.inner.borrow_mut();
        let rdx = parse_radix(&radix)?;
        self.inner
            .borrow_mut()
            .examine_signal(&mut sig, rdx)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn examine_signals_batch(
        &self,
        signals: Vec<Bound<'_, PySignal>>,
        radix: String,
    ) -> PyResult<()> {
        let rdx = parse_radix(&radix)?;
        let mut inner_signals: Vec<SimSignal> = signals
            .iter()
            .map(|s| s.borrow().inner.borrow().clone())
            .collect();
        self.inner
            .borrow_mut()
            .examine_signals_batch(&mut inner_signals, rdx)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        for (py_sig, inner_sig) in signals.iter().zip(inner_signals.iter()) {
            py_sig.borrow().inner.borrow_mut().value = inner_sig.value.clone();
        }
        Ok(())
    }

    fn examine_signal_int_64(&self, signal: &PySignal) -> PyResult<u64> {
        let mut sig = signal.inner.borrow_mut();
        self.inner
            .borrow_mut()
            .examine_signal(&mut sig, SimRadix::Binary)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        sig.get_numeric_value::<u64>()
            .ok_or_else(|| PyRuntimeError::new_err("Unable to convert signal to numeric value"))
    }

    fn examine_signal_int_128(&self, signal: &PySignal) -> PyResult<u128> {
        let mut sig = signal.inner.borrow_mut();
        self.inner
            .borrow_mut()
            .examine_signal(&mut sig, SimRadix::Binary)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        sig.get_numeric_value::<u128>()
            .ok_or_else(|| PyRuntimeError::new_err("Unable to convert signal to numeric value"))
    }

    fn force_signal(&self, signal: &PySignal, value: String, radix: String) -> PyResult<()> {
        let mut sig = signal.inner.borrow_mut();
        let rdx = parse_radix(&radix)?;
        self.inner
            .borrow_mut()
            .force_signal(&mut sig, &value, rdx)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn force_signals_batch(
        &self,
        signals: Vec<Bound<'_, PySignal>>,
        values: Vec<String>,
        radices: Vec<String>,
    ) -> PyResult<()> {
        if signals.len() != values.len() || signals.len() != radices.len() {
            return Err(PyRuntimeError::new_err(
                "signals, values, and radices must have the same length",
            ));
        }
        let inner_signals: Vec<SimSignal> = signals
            .iter()
            .map(|s| s.borrow().inner.borrow().clone())
            .collect();
        let parsed_values: PyResult<Vec<(SimRadix, &str)>> = radices
            .iter()
            .zip(values.iter())
            .map(|(radix_str, val)| parse_radix(radix_str).map(|rdx| (rdx, val.as_str())))
            .collect();
        let parsed_values = parsed_values?;
        self.inner
            .borrow_mut()
            .force_signals_batch(&inner_signals, &parsed_values)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn force_signal_update(&self, signal: &PySignal) -> PyResult<()> {
        let sig = signal.inner.borrow();
        self.inner
            .borrow_mut()
            .force_signal_update(&sig)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn restart(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .restart()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn reset(&self) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .reset()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn create_breakpoint(&self, filename: String, line_num: u32) -> PyResult<PyBreakpoint> {
        self.inner
            .borrow_mut()
            .create_breakpoint(&filename, line_num)
            .map(|bp| PyBreakpoint { inner: bp })
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn list_breakpoints(&self) -> PyResult<Vec<PyBreakpoint>> {
        self.inner
            .borrow_mut()
            .list_breakpoints()
            .map(|bps| {
                bps.into_iter()
                    .map(|bp| PyBreakpoint { inner: bp })
                    .collect()
            })
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
    fn delete_breakpoint(&self, file: String, line_num: u32) -> PyResult<()> {
        self.inner
            .borrow_mut()
            .delete_breakpoint(&file, line_num)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn get_log_matches_contains(&self, needle: String) -> PyResult<Vec<String>> {
        let contents: Vec<String> = {
            let mut runner = self.inner.borrow_mut();
            let matches = runner
                .get_log_matches(|line| line.contains(&needle))
                .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
            matches.into_iter().map(|pl| pl.content.clone()).collect()
        };
        Ok(contents)
    }
    fn send_and_expect_contains(&self, command: String, needle: String) -> PyResult<Vec<String>> {
        let contents: Vec<String> = {
            let mut runner = self.inner.borrow_mut();
            let matches = runner
                .send_and_expect_result(&command, |line| line.contains(&needle))
                .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
            matches.into_iter().map(|pl| pl.content.clone()).collect()
        };
        Ok(contents)
    }
    fn get_log_matches_regex(&self, pattern: String) -> PyResult<Vec<String>> {
        let re = Regex::new(&pattern).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("invalid regex: {}", e))
        })?;
        let contents: Vec<String> = {
            let mut runner = self.inner.borrow_mut();
            let matches = runner
                .get_log_matches(|line| re.is_match(line))
                .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
            matches.into_iter().map(|pl| pl.content.clone()).collect()
        };
        Ok(contents)
    }
    fn send_and_expect_regex(&self, command: String, pattern: String) -> PyResult<Vec<String>> {
        let re = Regex::new(&pattern).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("invalid regex: {}", e))
        })?;
        let contents: Vec<String> = {
            let mut runner = self.inner.borrow_mut();
            let matches = runner
                .send_and_expect_result(&command, |line| re.is_match(line))
                .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
            matches.into_iter().map(|pl| pl.content.clone()).collect()
        };
        Ok(contents)
    }

    fn parsed_buffer(&self) -> PyResult<Vec<PyParsedLine>> {
        Ok(self
            .inner
            .borrow()
            .parsed_buffer()
            .iter()
            .cloned()
            .map(|pl| PyParsedLine { inner: pl })
            .collect())
    }
    fn latest_buffer(&self) -> PyResult<Vec<PyParsedLine>> {
        Ok(self
            .inner
            .borrow()
            .latest_buffer()
            .iter()
            .cloned()
            .map(|pl| PyParsedLine { inner: pl.clone() })
            .collect())
    }

    fn list_mems(&self) -> PyResult<Vec<PyMemory>> {
        self.inner
            .borrow_mut()
            .list_mems()
            .map(|mems| mems.into_iter().map(|m| PyMemory { inner: m }).collect())
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
}

#[pyclass]
pub struct PyCompiler {
    inner: RefCell<Option<RsCompiler>>,
    is_vlog: bool,
}

impl PyCompiler {
    fn update(&self, f: impl FnOnce(RsCompiler) -> RsCompiler) -> PyResult<()> {
        let mut slot = self.inner.borrow_mut();
        if let Some(compiler) = slot.take() {
            *slot = Some(f(compiler));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Compiler already used"))
        }
    }
}

#[pymethods]
impl PyCompiler {
    #[new]
    fn new(work_dir: String, modelsim_path: String, command: String) -> PyResult<Self> {
        let cmd = match command.to_lowercase().as_str() {
            "vlog" => CompilerCommand::Vlog,
            "vcom" => CompilerCommand::Vcom,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "command must be 'vlog' or 'vcom'",
                ));
            }
        };
        let is_vlog = matches!(cmd, CompilerCommand::Vlog);
        let compiler = RsCompiler::new(&work_dir, &modelsim_path, cmd);
        Ok(Self {
            inner: RefCell::new(Some(compiler)),
            is_vlog,
        })
    }
    fn enable_system_verilog(&self) -> PyResult<()> {
        if !self.is_vlog {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "SystemVerilog only supported with 'vlog'",
            ));
        }
        self.update(|c| c.enable_system_verilog())
    }
    fn add_arg(&self, arg: String) -> PyResult<()> {
        self.update(|c| c.add_arg(&arg))
    }
    fn add_optimization(&self, level: u8) -> PyResult<()> {
        self.update(|c| c.add_optimization(level))
    }
    fn set_current_dir(&self, dir: String) -> PyResult<()> {
        self.update(|c| c.set_current_dir(&dir))
    }
    fn add_dependency(&self, dep: String) -> PyResult<()> {
        if !Path::new(&dep).exists() {
            return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
                "Dependency file does not exist: {}",
                dep
            )));
        }
        self.update(|c| c.add_dependency(&dep))
    }
    fn add_dependencies(&self, deps: Vec<String>) -> PyResult<()> {
        for dep in &deps {
            if !Path::new(dep).exists() {
                return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
                    "Dependency file does not exist: {}",
                    dep
                )));
            }
        }
        self.update(|c| c.add_dependencies(&deps))
    }
    fn add_work_library(&self, lib: String) -> PyResult<()> {
        self.update(|c| c.add_work_library(&lib))
    }
    fn set_work(&self, lib: String) -> PyResult<()> {
        self.update(|c| c.set_work(&lib))
    }
    fn run(&self) -> PyResult<()> {
        let mut slot = self.inner.borrow_mut();
        let Some(compiler) = slot.take() else {
            return Err(PyRuntimeError::new_err("Compiler already used"));
        };
        compiler
            .run()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
}

#[pyfunction]
fn version() -> String {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

#[pymodule]
fn ruvsim(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction_bound!(version, m)?)?;
    m.add_class::<PyRunner>()?;
    m.add_class::<PyCompiler>()?;
    m.add_class::<PySignal>()?;
    m.add_class::<PyDriver>()?;
    m.add_class::<PyBreakpoint>()?;
    m.add_class::<PyMemory>()?;
    m.add_class::<PyParsedLine>()?;
    Ok(())
}
