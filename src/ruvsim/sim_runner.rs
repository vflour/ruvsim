use std::{error::Error, time::Duration};

use super::sim_ctl::breakpoint_ctl::BreakpointCtl; // renamed corrected file
use super::sim_ctl::command_ctl::CommandCtl;
use super::sim_ctl::mem_ctl::MemCtl;
use super::sim_ctl::signal_ctl::SignalCtl;
use super::sim_types::{
    LineType, ParsedLine, SimBreakpoint, SimMemory, SimRadix, SimSignal, SimSignalDirection,
};

pub struct RunnerBuilder {
    vsim_command: String,
    args: Vec<String>,
    cwd: String,
}

impl RunnerBuilder {
    pub fn new() -> Self {
        Self {
            vsim_command: "vsim".to_string(),
            args: Vec::new(),
            cwd: ".".to_string(),
        }
    }
    pub fn with_vsim_command(mut self, cmd: &str) -> Self {
        self.vsim_command = cmd.to_string();
        self
    }
    pub fn with_cwd(mut self, cwd: &str) -> Self {
        self.cwd = cwd.to_string();
        self
    }
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }
    pub fn with_args(mut self, more: &[&str]) -> Self {
        for a in more {
            self.args.push(a.to_string());
        }
        self
    }
    pub fn work_lib(mut self, lib: &str) -> Self {
        self.args.push("-work".to_string());
        self.args.push(lib.to_string());
        self
    }
    pub fn enable_acc(mut self) -> Self {
        self.args.push("-voptargs=+acc".to_string());
        self
    }
    pub fn top(mut self, top: &str) -> Self {
        self.args.push(top.to_string());
        self
    }
    pub fn batch_mode(mut self) -> Self {
        self.args.push("-c".to_string());
        self
    }
    pub fn build(self) -> Runner {
        let arg_refs: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
        Runner::new(&self.vsim_command, &arg_refs, &self.cwd)
    }
}

pub struct Runner {
    ctl: CommandCtl,
    signal_ctl: SignalCtl,
    breakpoint_ctl: BreakpointCtl,
    mem_ctl: MemCtl,
}

impl Runner {
    pub fn new(vsim_command: &str, args: &[&str], cwd: &str) -> Self {
        let ctl = CommandCtl::spawn(vsim_command, args, cwd)
            .expect("Failed to spawn program. Is it installed?");
        Runner {
            ctl,
            signal_ctl: SignalCtl::new(),
            breakpoint_ctl: BreakpointCtl::new(),
            mem_ctl: MemCtl::new(),
        }
    }
    pub fn wait_until_prompt_startup(&mut self) -> Result<(), Box<dyn Error>> {
        self.ctl.wait_until_prompt_startup()
    }

    pub fn wait_until_prompt(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        self.ctl.wait_until_prompt(timeout)
    }

    pub fn send_command(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        self.ctl.send_command(command)
    }

    pub fn send_command_no_wait(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        self.ctl.send_command_no_wait(command)
    }

    pub fn run_all(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command_no_wait("run -all")
    }

    pub fn run_next(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("run -next")
    }

    pub fn run_for(&mut self, ns: u64) -> Result<(), Box<dyn Error>> {
        self.send_command(&format!("run {}ns", ns))
    }

    pub fn send_and_expect_result(
        &mut self,
        command: &str,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, Box<dyn Error>> {
        self.ctl.send_and_expect_result(command, predicate)
    }

    pub fn get_log_matches(
        &mut self,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, Box<dyn Error>> {
        self.ctl.get_log_matches(predicate)
    }

    pub fn finish(&mut self) -> Result<(), Box<dyn Error>> {
        self.ctl.finish()
    }

    pub fn get_nets(
        &mut self,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<Vec<SimSignal>, Box<dyn Error>> {
        let ctl = &mut self.ctl; // separate mutable borrow
        self.signal_ctl
            .get_signals_with_ctl(ctl, path, net_direction)
    }

    pub fn get_net(
        &mut self,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<SimSignal, Box<dyn Error>> {
        let ctl = &mut self.ctl;
        self.signal_ctl
            .get_signals_with_ctl(ctl, path, net_direction)?
            .get(0)
            .cloned()
            .ok_or_else(|| format!("No signal found at path: {}", path).into())
    }

    pub fn examine_net(
        &mut self,
        signal: &mut SimSignal,
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        let radix_arg = match radix {
            SimRadix::Binary => "-binary",
            SimRadix::Octal => "-octal",
            SimRadix::Decimal => "-decimal",
            SimRadix::Hexadecimal => "-hexadecimal",
            SimRadix::Unsigned => "-unsigned",
            SimRadix::Unknown => "",
        };
        let output = self.send_and_expect_result(
            &format!("puts \"EXAMINE: [examine {} {}]\"", radix_arg, signal.name),
            |line| line.starts_with("EXAMINE: "),
        )?;
        let examined_line = output
            .last()
            .unwrap()
            .content
            .trim_start_matches("EXAMINE: ");
        signal.set(radix, examined_line);
        Ok(())
    }

    pub fn examine_nets_batch(
        &mut self,
        signals: &mut [SimSignal],
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        if signals.is_empty() {
            return Ok(());
        }
        let radix_arg = match radix {
            SimRadix::Binary => "-binary",
            SimRadix::Octal => "-octal",
            SimRadix::Decimal => "-decimal",
            SimRadix::Hexadecimal => "-hexadecimal",
            SimRadix::Unsigned => "-unsigned",
            SimRadix::Unknown => "",
        };
        let mut tcl_commands = Vec::new();
        for signal in signals.iter() {
            tcl_commands.push(format!(
                "puts \"EXAMINE_BATCH:{}:[examine {} {}]\"",
                signal.name, radix_arg, signal.name
            ));
        }
        let combined_command = tcl_commands.join("; ");
        self.send_command(&combined_command)?;
        let mut results = std::collections::HashMap::new();
        for line in self.ctl.parsed_buffer().iter() {
            if matches!(line.line_type, LineType::Output | LineType::Log)
                && line.content.starts_with("EXAMINE_BATCH:")
            {
                let parts: Vec<&str> = line
                    .content
                    .trim_start_matches("EXAMINE_BATCH:")
                    .splitn(2, ':')
                    .collect();
                if parts.len() == 2 {
                    results.insert(parts[0].to_string(), parts[1].to_string());
                }
            }
        }
        for signal in signals.iter_mut() {
            if let Some(value) = results.get(&signal.name) {
                signal.set(radix, value);
            }
        }
        Ok(())
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("restart -f")
    }
    pub fn reset(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("reset -f")
    }

    pub fn breakpoint_ctl(&self) -> &BreakpointCtl {
        &self.breakpoint_ctl
    }
    pub fn breakpoint_ctl_mut(&mut self) -> &mut BreakpointCtl {
        &mut self.breakpoint_ctl
    }

    pub fn create_breakpoint(
        &mut self,
        filename: &str,
        line_num: u32,
    ) -> Result<SimBreakpoint, Box<dyn Error>> {
        self.breakpoint_ctl
            .create(&mut self.ctl, filename, line_num)
    }

    pub fn run_continue(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("run -continue")
    }

    pub fn force_signal(
        &mut self,
        signal: &SimSignal,
        value: &str,
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        self.signal_ctl
            .force_signal(&mut self.ctl, signal, value, radix)
    }
    pub fn force_signals_batch(
        &mut self,
        signals: &[SimSignal],
        values: &[(SimRadix, &str)],
    ) -> Result<(), Box<dyn Error>> {
        self.signal_ctl
            .force_signals_batch(&mut self.ctl, signals, values)
    }
    pub fn force_signal_update(&mut self, signal: &SimSignal) -> Result<(), Box<dyn Error>> {
        self.signal_ctl.force_signal_update(&mut self.ctl, signal)
    }

    pub fn get_time(&mut self) -> Result<String, Box<dyn Error>> {
        let output =
            self.send_and_expect_result("puts \"TIME: $now\"", |line| line.starts_with("TIME:"))?;
        Ok(output
            .last()
            .unwrap()
            .content
            .trim_start_matches("TIME:")
            .to_string())
    }

    pub fn get_status(&mut self) -> Result<String, Box<dyn Error>> {
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

    pub fn list_breakpoints(&mut self) -> Result<Vec<SimBreakpoint>, Box<dyn Error>> {
        self.breakpoint_ctl.list(&mut self.ctl)
    }

    pub fn delete_breakpoint(&mut self, file: &str, line_num: u32) -> Result<(), Box<dyn Error>> {
        self.breakpoint_ctl.delete(&mut self.ctl, file, line_num)
    }

    pub fn signal_ctl(&self) -> &SignalCtl {
        &self.signal_ctl
    }
    pub fn signal_ctl_mut(&mut self) -> &mut SignalCtl {
        &mut self.signal_ctl
    }

    pub fn parsed_buffer(&self) -> &Vec<ParsedLine> {
        self.ctl.parsed_buffer()
    }
    pub fn latest_buffer(&self) -> &[ParsedLine] {
        self.ctl.latest_buffer()
    }

    pub fn list_mems(&mut self) -> Result<Vec<SimMemory>, Box<dyn Error>> {
        self.mem_ctl.list(&mut self.ctl)
    }
}
