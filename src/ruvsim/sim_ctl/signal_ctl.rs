use super::super::sim_types::{
    LineType, SimDriver, SimError, SimRadix, SimSignal, SimSignalDirection,
};
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

    pub fn populate_all(&mut self, ctl: &mut CommandCtl, top_path: &str) -> Result<(), SimError> {
        let all = self.get_signals_with_ctl(ctl, top_path, None)?;
        self.signals = all;
        Ok(())
    }

    pub fn get_signals_with_ctl(
        &mut self,
        ctl: &mut CommandCtl,
        path: &str,
        net_direction: Option<SimSignalDirection>,
    ) -> Result<Vec<SimSignal>, SimError> {
        let mut nets: Vec<SimSignal> = Vec::new();
        let directions: Vec<SimSignalDirection> = match net_direction {
            Some(d) => vec![d],
            None => vec![SimSignalDirection::None],
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
    ) -> Result<Vec<String>, SimError> {
        let direction_arg = match net_direction {
            SimSignalDirection::Input => "-in",
            SimSignalDirection::Output => "-out",
            SimSignalDirection::Inout => "-inout",
            SimSignalDirection::None => "",
            _ => return Err("Invalid direction for get_signal_paths".into()),
        };
        let output = ctl.send_and_expect_result(
            &format!("puts \"NETS: [find nets {} {}]\"", direction_arg, path),
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
    ) -> Result<SimSignal, SimError> {
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
        ctl: &mut CommandCtl,
        signal: &mut SimSignal,
        radix: SimRadix,
    ) -> Result<(), SimError> {
        let radix_arg = match radix {
            SimRadix::Binary => "-binary",
            SimRadix::Octal => "-octal",
            SimRadix::Decimal => "-decimal",
            SimRadix::Hexadecimal => "-hexadecimal",
            SimRadix::Unsigned => "-unsigned",
            SimRadix::Unknown => "",
        };
        let output = ctl.send_and_expect_result(
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

    pub fn examine_batch(
        &mut self,
        ctl: &mut CommandCtl,
        signals: &mut [SimSignal],
        radix: SimRadix,
    ) -> Result<(), SimError> {
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
        ctl.send_command(&combined_command)?;
        let mut results = std::collections::HashMap::new();
        for line in ctl.parsed_buffer().iter() {
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

    pub fn force_signal(
        &mut self,
        ctl: &mut CommandCtl,
        signal: &SimSignal,
        value: &str,
        radix: SimRadix,
    ) -> Result<(), SimError> {
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
    ) -> Result<(), SimError> {
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
    ) -> Result<(), SimError> {
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
    ) -> Result<(), SimError> {
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
    ) -> Result<(), SimError> {
        if names.len() != vals.len() {
            return Err("Names and values length mismatch".into());
        }
        let mut sigs: Vec<SimSignal> = Vec::new();
        for n in names {
            let idx = self
                .signals
                .iter()
                .position(|s| s.name == *n)
                .ok_or(SimError::CommandError(format!("Signal '{}' not found", n)))?;
            sigs.push(self.signals[idx].clone());
        }
        self.force_signals_batch(ctl, &sigs, vals)
    }
}
