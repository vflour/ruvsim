use std::error::Error;

use super::super::sim_runner::Runner;
use super::super::sim_types::{SimDriver, SimRadix, SimSignal, SimSignalDirection};
use super::command_ctl::CommandCtl;

pub struct SignalCtl {
    signals: Vec<SimSignal>,
}

impl SignalCtl {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
        }
    }

    pub fn signals(&self) -> &Vec<SimSignal> {
        &self.signals
    }
    pub fn signals_mut(&mut self) -> &mut Vec<SimSignal> {
        &mut self.signals
    }

    pub fn populate_all(
        &mut self,
        ctl: &mut CommandCtl,
        top_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        let all = self.get_signals_with_ctl(ctl, top_path, None)?;
        self.signals = all;
        Ok(())
    }

    pub fn get_signals_with_ctl(
        &mut self,
        ctl: &mut CommandCtl,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<Vec<SimSignal>, Box<dyn Error>> {
        let mut nets: Vec<SimSignal> = Vec::new();
        let directions: Vec<SimSignalDirection> = match net_direction {
            Some(d) => vec![d],
            None => vec![
                SimSignalDirection::Input,
                SimSignalDirection::Output,
                SimSignalDirection::Inout,
            ],
        };
        for dir in directions {
            let paths = self.get_signal_paths(ctl, path, dir)?;
            for net_path in paths {
                let net_data = self.get_signal(ctl, &net_path, dir)?;
                nets.push(net_data);
            }
        }
        Ok(nets)
    }

    pub fn get_signal_paths(
        &mut self,
        ctl: &mut CommandCtl,
        path: &str,
        net_direction: SimSignalDirection,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let direction_arg = match net_direction {
            SimSignalDirection::Input => "in",
            SimSignalDirection::Output => "out",
            SimSignalDirection::Inout => "inout",
            _ => return Err("Invalid direction for get_signal_paths".into()),
        };
        let output = ctl.send_and_expect_result(
            &format!("puts \"NETS: [find nets -{} {}]\"", direction_arg, path),
            |line| line.starts_with("NETS:"),
        )?;
        let line = output.last().unwrap().content.trim_start_matches("NETS:");
        Ok(line.split_whitespace().map(|s| s.to_string()).collect())
    }

    pub fn get_signal(
        &mut self,
        ctl: &mut CommandCtl,
        path: &str,
        net_direction: SimSignalDirection,
    ) -> Result<SimSignal, Box<dyn Error>> {
        let drivers_output = ctl.send_and_expect_result(
            &format!(
                "puts \"DRIVERS: [string map {{\\n ;}} [drivers {}]]\"",
                path
            ),
            |line| line.starts_with("DRIVERS: "),
        )?;
        let drivers_line = drivers_output
            .last()
            .unwrap()
            .content
            .trim_start_matches("DRIVERS: ");
        let drivers = SimDriver::from_multiple_drivers_output(drivers_line, path);
        let net_output = ctl.send_and_expect_result(
            &format!("puts \"NET_DATA: [describe -binary {}]\"", path),
            |line| line.starts_with("NET_DATA: "),
        )?;
        let net_line = net_output
            .last()
            .unwrap()
            .content
            .trim_start_matches("NET_DATA: ");
        Ok(SimSignal::from_describe_output(
            path,
            net_direction,
            drivers,
            net_line,
        ))
    }

    pub fn find(&self, name: &str) -> Option<&SimSignal> {
        self.signals.iter().find(|s| s.name == name)
    }
    pub fn find_mut(&mut self, name: &str) -> Option<&mut SimSignal> {
        self.signals.iter_mut().find(|s| s.name == name)
    }

    pub fn examine(
        &mut self,
        runner: &mut Runner,
        name: &str,
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        let sig = self.find_mut(name).ok_or("Signal not found")?;
        runner.examine_net(sig, radix)
    }

    pub fn examine_batch(
        &mut self,
        runner: &mut Runner,
        names: &[&str],
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        // Collect indices first
        let mut indices: Vec<usize> = Vec::new();
        for n in names {
            if let Some(pos) = self.signals.iter().position(|s| s.name == *n) {
                indices.push(pos);
            } else {
                return Err(format!("Signal '{}' not found", n).into());
            }
        }
        let mut temp: Vec<SimSignal> = indices.iter().map(|&i| self.signals[i].clone()).collect();
        runner.examine_nets_batch(&mut temp, radix)?;
        for (idx, updated) in indices.into_iter().zip(temp.into_iter()) {
            self.signals[idx].value = updated.value;
        }
        Ok(())
    }

    pub fn force_signal(
        &mut self,
        ctl: &mut CommandCtl,
        signal: &SimSignal,
        value: &str,
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        let value_prefix = match radix {
            SimRadix::Binary => "'b",
            SimRadix::Octal => "'o",
            SimRadix::Decimal => "'d",
            SimRadix::Hexadecimal => "'h",
            SimRadix::Unsigned => "'u",
            SimRadix::Unknown => "",
        };
        ctl.send_command(&format!("force {} {}{}", signal.name, value_prefix, value))
    }

    pub fn force_signals_batch(
        &mut self,
        ctl: &mut CommandCtl,
        signals: &[SimSignal],
        values: &[(SimRadix, &str)],
    ) -> Result<(), Box<dyn Error>> {
        if signals.is_empty() || signals.len() != values.len() {
            return Err("Signals and values must have the same length and be non-empty".into());
        }
        let mut tcl_commands = Vec::new();
        for (signal, (radix, value)) in signals.iter().zip(values.iter()) {
            let value_prefix = match radix {
                SimRadix::Binary => "'b",
                SimRadix::Octal => "'o",
                SimRadix::Decimal => "'d",
                SimRadix::Hexadecimal => "'h",
                SimRadix::Unsigned => "'u",
                SimRadix::Unknown => "",
            };
            tcl_commands.push(format!("force {} {}{}", signal.name, value_prefix, value));
        }
        let combined_command = tcl_commands.join("; ");
        ctl.send_command(&combined_command)
    }

    pub fn force_signal_update(
        &mut self,
        ctl: &mut CommandCtl,
        signal: &SimSignal,
    ) -> Result<(), Box<dyn Error>> {
        let value = signal
            .value
            .as_ref()
            .ok_or("Signal has no value to force")?;
        self.force_signal(ctl, signal, value.1.as_str(), value.0)
    }

    // Legacy by-name wrappers delegate to new methods
    pub fn force(
        &mut self,
        ctl: &mut CommandCtl,
        name: &str,
        value: &str,
        radix: SimRadix,
    ) -> Result<(), Box<dyn Error>> {
        let idx = self
            .signals
            .iter()
            .position(|s| s.name == name)
            .ok_or("Signal not found")?;
        let sig_clone = self.signals[idx].clone();
        self.force_signal(ctl, &sig_clone, value, radix)
    }
    pub fn force_batch(
        &mut self,
        ctl: &mut CommandCtl,
        names: &[&str],
        vals: &[(SimRadix, &str)],
    ) -> Result<(), Box<dyn Error>> {
        if names.len() != vals.len() {
            return Err("Names and values length mismatch".into());
        }
        let mut sigs: Vec<SimSignal> = Vec::new();
        for n in names {
            let idx = self
                .signals
                .iter()
                .position(|s| s.name == *n)
                .ok_or(format!("Signal '{}' not found", n))?;
            sigs.push(self.signals[idx].clone());
        }
        self.force_signals_batch(ctl, &sigs, vals)
    }
}
