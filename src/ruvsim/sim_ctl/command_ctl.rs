use super::super::sim_parser::Parser;
use super::super::sim_types::{LineType, ParsedLine, SimError};

use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};


// Control interface for sending commands to the simulator process
pub struct CommandCtl {
    parser: Parser,
    stdin: ChildStdin,
    child: Child,
    latest_cache: Vec<ParsedLine>, // cache of lines since last prompt
}

impl CommandCtl {
    pub fn spawn(vsim_command: &str, args: &[&str], cwd: &str) -> Result<Self, SimError> {
        // Ensure batch mode (-c) unless already specified in args
        let needs_batch = !args.iter().any(|a| *a == "-c");
        let mut cmd = Command::new(vsim_command);
        if needs_batch {
            cmd.arg("-c");
        }
        let mut child = cmd
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SimError::ProcessError(format!("Failed to start process: {}", e)))?;

        let stdout = child.stdout.take().ok_or("Expected stdout")?;
        let reader = BufReader::new(stdout);
        let parser = Parser::new(reader);
        let stdin = child.stdin.take().ok_or("Expected stdin")?;

        log::info!("Started simulator process with PID {}", child.id());
        let child_status = child.try_wait().map_err(|e| SimError::ProcessError(format!("Failed to check process status: {}", e)))?;
        if  let Some(status) = child_status {
            return Err(SimError::ProcessError(format!(
                "Simulator exited immediately with status {}. CMD: {} {}\n CWD: {}",
                status,
                vsim_command,
                args.join(" "),
                cwd
            )
            .into()));
        }

        Ok(Self {
            parser,
            stdin,
            child,
            latest_cache: Vec::new(),
        })
    }

    pub fn wait_until_prompt_startup(&mut self) -> Result<(), SimError> {
        self.wait_until_prompt(Duration::from_secs(2)).map_err(|e| {
            if self.has_error() {
                return SimError::CommandError("Simulation error during startup.".to_string());
            }
            SimError::TimeoutError(format!("Timeout waiting for prompt during startup. Is the simulator installed and accessible via PATH? Error: {}", e))
        })
    }

    pub fn wait_until_prompt(&mut self, timeout: Duration) -> Result<(), SimError> {
        let at_timeout = Instant::now() + timeout;
        let mut now = Instant::now();
        while at_timeout > now {
            let next_line = self.parser.get_next_prompt()?;
            if next_line.is_some() {
                // rebuild latest_cache from indices
                self.latest_cache.clear();
                for &idx in self.parser.latest_buffer().iter() {
                    self.latest_cache
                        .push(self.parser.parsed_buffer()[idx].clone());
                }
                return Ok(());
            }
            now = Instant::now();
            thread::sleep(Duration::from_millis(10));
        }
        Err(SimError::TimeoutError("Timeout waiting for prompt".to_string()).into())
    }

    pub fn send_command(&mut self, command: &str) -> Result<(), SimError> {
        self.send_command_no_wait(command)?;
        self.wait_until_prompt(Duration::from_millis(500))?;
        if self.has_error() {
            return Err(SimError::CommandError(format!("Simulation error while executing command {}", command)).into());
        }
        Ok(())
    }

    fn has_error(&self) -> bool {
        for line in self.latest_cache.iter() {
            if matches!(line.line_type, LineType::Error) {
                return true;
            }
        }
        false
    }

    pub fn send_command_no_wait(&mut self, command: &str) -> Result<(), SimError> {
        let write_buffer = format!("{}\n", command);
        self.stdin.write_all(write_buffer.as_bytes()).map_err(|e| SimError::IOError(format!("Failed to write to stdin: {}", e)))?;
        self.stdin.flush().map_err(|e| SimError::IOError(format!("Failed to flush stdin: {}", e)))?;
        Ok(())
    }

    pub fn send_and_expect_result(
        &mut self,
        command: &str,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, SimError> {
        self.send_command(command)?;
        self.get_log_matches(predicate)
    }

    pub fn get_log_matches(
        &mut self,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, SimError> {
        let mut matches = Vec::new();
        for line in self.parser.parsed_buffer().iter() {
            if matches!(line.line_type, LineType::Output | LineType::Log)
                && predicate(&line.content)
            {
                matches.push(line);
            }
        }
        Ok(matches)
    }

    pub fn finish(&mut self) -> Result<(), SimError> {

        // Send quit command 
        if let Err(e) = self.send_command_no_wait("quit") {
            log::warn!("Failed to send quit command: {}", e);
        }

        // Give it some time to exit gracefully
        thread::sleep(Duration::from_millis(100));

        // Check if process has exited
        let wait_result = self.child.try_wait().map_err(|e| SimError::ProcessError(format!("Failed to wait for process exit: {}", e)))?;
        // If not, forcibly kill it
        if wait_result.is_none() {
            log::warn!("Process did not exit. Killing process forcibly.");
            self.child.kill().map_err(|e| SimError::ProcessError(format!("Failed to kill process: {}", e)))?;
            self.child.wait().map_err(|e| SimError::ProcessError(format!("Failed to wait for process exit: {}", e)))?;
        }
        Ok(())
    }

    // Parser accessors for higher-level APIs
    pub fn parsed_buffer(&self) -> &Vec<ParsedLine> {
        self.parser.parsed_buffer()
    }
    pub fn latest_buffer(&self) -> &[ParsedLine] {
        &self.latest_cache
    }
}
