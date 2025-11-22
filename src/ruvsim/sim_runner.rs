use std::{time::Duration};

use super::sim_ctl::breakpoint_ctl::BreakpointCtl; // renamed corrected file
use super::sim_ctl::command_ctl::CommandCtl;
use super::sim_ctl::mem_ctl::MemCtl;
use super::sim_ctl::signal_ctl::SignalCtl;
use super::sim_types::{
    ParsedLine, SimBreakpoint, SimMemory, SimRadix, SimSignal, SimSignalDirection, SimError, SimTimeUnit,
};

/// Builder for creating a `Runner` instance with custom configuration.
///
/// This builder pattern allows for flexible configuration of the simulation runner,
/// including setting the vsim command, arguments, and working directory.
pub struct RunnerBuilder {
    vsim_command: String,
    args: Vec<String>,
    cwd: String,
}

impl RunnerBuilder {
    /// Creates a new `RunnerBuilder` with default values.
    ///
    /// Default values:
    /// - `vsim_command`: "vsim"
    /// - `args`: empty vector
    /// - `cwd`: "." (current directory)
    pub fn new() -> Self {
        Self {
            vsim_command: "vsim".to_string(),
            args: Vec::new(),
            cwd: ".".to_string(),
        }
    }
    
    /// Sets the vsim command to use for the simulation.
    ///
    /// # Arguments
    /// * `cmd` - The path or command for the vsim executable
    pub fn with_vsim_command(mut self, cmd: &str) -> Self {
        self.vsim_command = cmd.to_string();
        self
    }
    
    /// Sets the current working directory for the simulation process.
    ///
    /// # Arguments
    /// * `cwd` - The path to the working directory
    pub fn with_cwd(mut self, cwd: &str) -> Self {
        self.cwd = cwd.to_string();
        self
    }
    
    /// Adds a single argument to the vsim command.
    ///
    /// # Arguments
    /// * `arg` - The argument to add
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }
    
    /// Adds multiple arguments to the vsim command.
    ///
    /// # Arguments
    /// * `more` - A slice of arguments to add
    pub fn with_args(mut self, more: &[&str]) -> Self {
        for a in more {
            self.args.push(a.to_string());
        }
        self
    }
    
    /// Sets the work library for the simulation.
    ///
    /// This adds `-work <lib>` arguments to the vsim command.
    ///
    /// # Arguments
    /// * `lib` - The name of the work library
    pub fn work_lib(mut self, lib: &str) -> Self {
        self.args.push("-work".to_string());
        self.args.push(lib.to_string());
        self
    }
    
    /// Enables access to all signals and variables in the simulation.
    ///
    /// This adds `-voptargs=+acc` to enable full visibility of simulation objects.
    pub fn enable_acc(mut self) -> Self {
        self.args.push("-voptargs=+acc".to_string());
        self
    }
    
    /// Sets the top-level module for the simulation.
    ///
    /// # Arguments
    /// * `top` - The name of the top-level module
    pub fn top(mut self, top: &str) -> Self {
        self.args.push(top.to_string());
        self
    }
    
    /// Enables batch mode for the simulation.
    ///
    /// This adds the `-c` flag to run vsim in command-line (non-GUI) mode.
    pub fn batch_mode(mut self) -> Self {
        self.args.push("-c".to_string());
        self
    }
    
    /// Builds and returns a configured `Runner` instance.
    ///
    /// This consumes the builder and spawns the vsim process with the configured settings.
    pub fn build(self) -> Runner {
        let arg_refs: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
        Runner::new(&self.vsim_command, &arg_refs, &self.cwd)
    }
}

/// Main interface for controlling a ModelSim/Questa simulation.
///
/// The `Runner` provides a high-level API for interacting with a running simulation,
/// including signal examination, breakpoint management, and simulation control.
pub struct Runner {
    ctl: CommandCtl,
    signal_ctl: SignalCtl,
    breakpoint_ctl: BreakpointCtl,
    mem_ctl: MemCtl,
}

impl Runner {
    /// Creates a new `Runner` instance and spawns the vsim process.
    ///
    /// # Arguments
    /// * `vsim_command` - The vsim executable path or command
    /// * `args` - Command-line arguments for vsim
    /// * `cwd` - Working directory for the simulation process
    ///
    /// # Panics
    /// Panics if the vsim process fails to spawn.
    pub fn new(vsim_command: &str, args: &[&str], cwd: &str) -> Self {
        let ctl = CommandCtl::spawn(vsim_command, args, cwd)
            .expect("Failed to spawn program.");
        Runner {
            ctl,
            signal_ctl: SignalCtl::new(),
            breakpoint_ctl: BreakpointCtl::new(),
            mem_ctl: MemCtl::new(),
        }
    }
    
    /// Waits for the simulation prompt to appear during startup.
    ///
    /// This should be called after creating a new `Runner` to ensure the simulation
    /// is ready to accept commands.
    ///
    /// # Returns
    /// `Ok(())` if the prompt appears, or a `SimError` if there's a timeout or error.
    pub fn wait_until_prompt_startup(&mut self) -> Result<(), SimError> {
        self.ctl.wait_until_prompt_startup()
    }

    /// Waits for the simulation prompt to appear with a specified timeout.
    ///
    /// # Arguments
    /// * `timeout` - Maximum duration to wait for the prompt
    ///
    /// # Returns
    /// `Ok(())` if the prompt appears within the timeout, or a `SimError` otherwise.
    pub fn wait_until_prompt(&mut self, timeout: Duration) -> Result<(), SimError> {
        self.ctl.wait_until_prompt(timeout)
    }

    /// Sends a TCL command to the simulation and waits for completion.
    ///
    /// # Arguments
    /// * `command` - The TCL command to execute
    ///
    /// # Returns
    /// `Ok(())` if the command executes successfully, or a `SimError` otherwise.
    pub fn send_command(&mut self, command: &str) -> Result<(), SimError> {
        self.ctl.send_command(command)
    }

    /// Sends a TCL command to the simulation without waiting for completion.
    ///
    /// This is useful for commands that may run indefinitely (like `run -all`).
    ///
    /// # Arguments
    /// * `command` - The TCL command to execute
    ///
    /// # Returns
    /// `Ok(())` if the command is sent successfully, or a `SimError` otherwise.
    pub fn send_command_no_wait(&mut self, command: &str) -> Result<(), SimError> {
        self.ctl.send_command_no_wait(command)
    }

    /// Runs the simulation until completion or until a breakpoint is hit.
    ///
    /// This executes the `run -all` command without waiting for completion.
    ///
    /// # Returns
    /// `Ok(())` if the command is sent successfully, or a `SimError` otherwise.
    pub fn run_all(&mut self) -> Result<(), SimError> {
        self.send_command_no_wait("run -all")
    }

    /// Advances the simulation to the next event.
    ///
    /// This executes the `run -next` command and waits for completion.
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` otherwise.
    pub fn run_next(&mut self) -> Result<(), SimError> {
        self.send_command("run -next")
    }

    /// Runs the simulation for a specified duration for a given time unit.
    ///
    /// # Arguments
    /// * `duration` - The number of time units to simulate
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` otherwise.
    pub fn run_for(&mut self, duration: u64, timescale: SimTimeUnit) -> Result<(), SimError> {
        self.send_command(&format!("run {}{}", duration, timescale))
    }

    /// Sends a command and returns parsed output lines matching a predicate.
    ///
    /// # Arguments
    /// * `command` - The TCL command to execute
    /// * `predicate` - A function that returns true for lines to include in the result
    ///
    /// # Returns
    /// A vector of parsed lines matching the predicate, or a `SimError` on failure.
    pub fn send_and_expect_result(
        &mut self,
        command: &str,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, SimError> {
        self.ctl.send_and_expect_result(command, predicate)
    }

    /// Retrieves log lines from the buffer that match a predicate.
    ///
    /// # Arguments
    /// * `predicate` - A function that returns true for lines to include in the result
    ///
    /// # Returns
    /// A vector of parsed lines matching the predicate, or a `SimError` on failure.
    pub fn get_log_matches(
        &mut self,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, SimError> {
        self.ctl.get_log_matches(predicate)
    }

    /// Terminates the simulation process gracefully.
    ///
    /// # Returns
    /// `Ok(())` if the simulation terminates successfully, or a `SimError` otherwise.
    pub fn finish(&mut self) -> Result<(), SimError> {
        self.ctl.finish()
    }

    /// Retrieves all signals at the specified hierarchical path.
    ///
    /// # Arguments
    /// * `path` - The hierarchical path to the signals (e.g., "/top/module")
    /// * `net_direction` - Optional filter for signal direction (input, output, inout)
    ///
    /// # Returns
    /// A vector of signals at the specified path, or a `SimError` on failure.
    pub fn get_signals(
        &mut self,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<Vec<SimSignal>, SimError> {
        let ctl = &mut self.ctl; // separate mutable borrow
        self.signal_ctl
            .get_signals_with_ctl(ctl, path, net_direction)
    }

    /// Retrieves a single signal at the specified hierarchical path.
    ///
    /// # Arguments
    /// * `path` - The hierarchical path to the signal
    /// * `net_direction` - Optional filter for signal direction (input, output, inout)
    ///
    /// # Returns
    /// The signal at the specified path, or a `SimError` if not found or on failure.
    pub fn get_signal(
        &mut self,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<SimSignal, SimError> {
        let ctl = &mut self.ctl;
        self.signal_ctl
            .get_signals_with_ctl(ctl, path, net_direction)?
            .get(0)
            .cloned()
            .ok_or_else(|| SimError::CommandError(format!("No signal found at path: {}", path).into()))
    }

    /// Examines (reads) the current value of a signal in the specified radix.
    ///
    /// The signal's value is updated in place.
    ///
    /// # Arguments
    /// * `signal` - The signal to examine (will be updated with the current value)
    /// * `radix` - The number format to use (binary, hex, decimal, etc.)
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn examine_signal(
        &mut self,
        signal: &mut SimSignal,
        radix: SimRadix,
    ) -> Result<(), SimError> {
        self.signal_ctl.examine(&mut self.ctl, signal, radix)
    }

    /// Examines multiple signals in a single batch operation for efficiency.
    ///
    /// All signals' values are updated in place.
    ///
    /// # Arguments
    /// * `signals` - A mutable slice of signals to examine
    /// * `radix` - The number format to use for all signals
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn examine_signals_batch(
        &mut self,
        signals: &mut [SimSignal],
        radix: SimRadix,
    ) -> Result<(), SimError> {
        self.signal_ctl
            .examine_batch(&mut self.ctl, signals, radix)
    }

    /// Restarts the simulation from time zero.
    ///
    /// This executes the `restart -f` command.
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn restart(&mut self) -> Result<(), SimError> {
        self.send_command("restart -f")
    }
    
    /// Resets the simulation state.
    ///
    /// This executes the `reset -f` command.
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn reset(&mut self) -> Result<(), SimError> {
        self.send_command("reset -f")
    }

    /// Returns an immutable reference to the breakpoint controller.
    ///
    /// # Returns
    /// A reference to the `BreakpointCtl` instance.
    pub fn breakpoint_ctl(&self) -> &BreakpointCtl {
        &self.breakpoint_ctl
    }
    
    /// Returns a mutable reference to the breakpoint controller.
    ///
    /// # Returns
    /// A mutable reference to the `BreakpointCtl` instance.
    pub fn breakpoint_ctl_mut(&mut self) -> &mut BreakpointCtl {
        &mut self.breakpoint_ctl
    }

    /// Creates a breakpoint at the specified file and line number.
    ///
    /// # Arguments
    /// * `filename` - The source file containing the breakpoint location
    /// * `line_num` - The line number for the breakpoint
    ///
    /// # Returns
    /// The created `SimBreakpoint`, or a `SimError` on failure.
    pub fn create_breakpoint(
        &mut self,
        filename: &str,
        line_num: u32,
    ) -> Result<SimBreakpoint, SimError> {
        self.breakpoint_ctl
            .create(&mut self.ctl, filename, line_num)
    }

    /// Continues simulation execution after hitting a breakpoint.
    ///
    /// This executes the `run -continue` command.
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn run_continue(&mut self) -> Result<(), SimError> {
        self.send_command("run -continue")
    }

    /// Forces a signal to a specific value.
    ///
    /// # Arguments
    /// * `signal` - The signal to force
    /// * `value` - The value to force (as a string in the specified radix)
    /// * `radix` - The number format of the value
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn force_signal(
        &mut self,
        signal: &SimSignal,
        value: &str,
        radix: SimRadix,
    ) -> Result<(), SimError> {
        self.signal_ctl
            .force_signal(&mut self.ctl, signal, value, radix)
    }
    
    /// Forces multiple signals to specific values in a single batch operation.
    ///
    /// # Arguments
    /// * `signals` - A slice of signals to force
    /// * `values` - A slice of (radix, value) tuples corresponding to each signal
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn force_signals_batch(
        &mut self,
        signals: &[SimSignal],
        values: &[(SimRadix, &str)],
    ) -> Result<(), SimError> {
        self.signal_ctl
            .force_signals_batch(&mut self.ctl, signals, values)
    }
    
    /// Updates a forced signal with its current stored value.
    ///
    /// # Arguments
    /// * `signal` - The signal to update with its stored value
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn force_signal_update(&mut self, signal: &SimSignal) -> Result<(), SimError> {
        self.signal_ctl.force_signal_update(&mut self.ctl, signal)
    }

    /// Retrieves the current simulation time.
    ///
    /// # Returns
    /// The current simulation time as a string, or a `SimError` on failure.
    pub fn get_time(&mut self) -> Result<String, SimError> {
        let output =
            self.send_and_expect_result("puts \"TIME: $now\"", |line| line.starts_with("TIME:"))?;
        Ok(output
            .last()
            .unwrap()
            .content
            .trim_start_matches("TIME:")
            .to_string())
    }

    /// Retrieves the current simulation run status.
    ///
    /// # Returns
    /// The simulation status (e.g., "running", "break", "finish") as a string,
    /// or a `SimError` on failure.
    pub fn get_status(&mut self) -> Result<String, SimError> {
        let output = self.send_and_expect_result("puts \"STATUS: [runStatus]\"", |line| {
            line.starts_with("STATUS:")
        })?;
        Ok(output
            .last()
            .unwrap()
            .content
            .trim_start_matches("STATUS:")
            .to_string())
    }

    /// Lists all currently set breakpoints in the simulation.
    ///
    /// # Returns
    /// A vector of all breakpoints, or a `SimError` on failure.
    pub fn list_breakpoints(&mut self) -> Result<Vec<SimBreakpoint>, SimError> {
        self.breakpoint_ctl.list(&mut self.ctl)
    }

    /// Deletes a breakpoint at the specified file and line number.
    ///
    /// # Arguments
    /// * `file` - The source file containing the breakpoint
    /// * `line_num` - The line number of the breakpoint to delete
    ///
    /// # Returns
    /// `Ok(())` if successful, or a `SimError` on failure.
    pub fn delete_breakpoint(&mut self, file: &str, line_num: u32) -> Result<(), SimError> {
        self.breakpoint_ctl.delete(&mut self.ctl, file, line_num)
    }

    /// Returns the complete parsed output buffer from the simulation.
    ///
    /// # Returns
    /// A reference to the vector of all parsed lines from the simulation output.
    pub fn parsed_buffer(&self) -> &Vec<ParsedLine> {
        self.ctl.parsed_buffer()
    }
    
    /// Returns the most recent parsed output lines from the simulation.
    ///
    /// # Returns
    /// A slice of the latest parsed lines from the simulation output.
    pub fn latest_buffer(&self) -> &[ParsedLine] {
        self.ctl.latest_buffer()
    }

    /// Lists all memory objects in the simulation.
    ///
    /// # Returns
    /// A vector of all memory objects, or a `SimError` on failure.
    pub fn list_mems(&mut self) -> Result<Vec<SimMemory>, SimError> {
        self.mem_ctl.list(&mut self.ctl)
    }
}
