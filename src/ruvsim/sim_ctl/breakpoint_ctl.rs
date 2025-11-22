use std::path::PathBuf;

use super::super::sim_types::{LineType, ParsedLine, SimBreakpoint, SimError};
use super::command_ctl::CommandCtl;

pub struct BreakpointCtl {
    breakpoints: Vec<SimBreakpoint>,
    _enabled: bool,
}

impl BreakpointCtl {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            _enabled: true,
        }
    }
    pub fn breakpoints(&self) -> &Vec<SimBreakpoint> {
        &self.breakpoints
    }

    pub fn create(
        &mut self,
        ctl: &mut CommandCtl,
        filename: &str,
        line_num: u32,
    ) -> Result<SimBreakpoint, SimError> {
        let bp = SimBreakpoint {
            name: format!("{} {}", filename, line_num),
            file: PathBuf::from(filename),
            line_num,
        };
        ctl.send_command(&format!("bp {} {}", filename, line_num))?;
        ctl.send_and_expect_result("bp", |line| line.contains(&bp.name))?;
        // Issue list to refresh
        ctl.send_command("bp")?;
        self.refresh(ctl.latest_buffer());
        Ok(bp)
    }

    pub fn list(&mut self, ctl: &mut CommandCtl) -> Result<Vec<SimBreakpoint>, SimError> {
        ctl.send_command("bp")?;
        self.refresh(ctl.latest_buffer());
        Ok(self.breakpoints.clone())
    }

    pub fn delete(
        &mut self,
        ctl: &mut CommandCtl,
        file: &str,
        line_num: u32,
    ) -> Result<(), SimError> {
        ctl.send_command(&format!("bd {} {}", file, line_num))?; // NOTE: may need 'bp -delete'
        ctl.send_command("bp")?; // refresh listing
        self.refresh(ctl.latest_buffer());
        if self
            .breakpoints
            .iter()
            .any(|bp| bp.file.to_string_lossy() == file && bp.line_num == line_num)
        {
            return Err("Breakpoint still present after delete".into());
        }
        Ok(())
    }

    fn refresh(&mut self, latest: &[ParsedLine]) {
        let mut seen = std::collections::HashSet::new();
        let mut bps: Vec<SimBreakpoint> = Vec::new();
        for line in latest.iter() {
            if !matches!(line.line_type, LineType::Output | LineType::Log) {
                continue;
            }
            let raw = line.content.trim_start_matches('#').trim();
            if !raw.starts_with("bp ") {
                continue;
            }
            let mut parts = raw.split_whitespace();
            parts.next(); // bp
            let file = match parts.next() {
                Some(f) => f,
                None => continue,
            };
            let ln = match parts.next().and_then(|s| s.parse::<u32>().ok()) {
                Some(n) => n,
                None => continue,
            };
            if !seen.insert((file.to_string(), ln)) {
                continue;
            }
            bps.push(SimBreakpoint {
                name: format!("{} {}", file, ln),
                file: PathBuf::from(file),
                line_num: ln,
            });
        }
        self.breakpoints = bps;
    }

    pub fn toggle_enabled(&mut self, _ctl: &mut CommandCtl) -> Result<(), SimError> {
        self._enabled = !self._enabled;
        if self._enabled {
            _ctl.send_command("enablebp")?;
        } else {
            _ctl.send_command("disablebp")?;
        }
        Ok(())
    }
}
