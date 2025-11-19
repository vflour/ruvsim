use super::super::sim_parser::Parser;
use super::super::sim_types::{LineType, ParsedLine};

use std::error::Error;
use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct CommandCtl {
    parser: Parser,
    stdin: ChildStdin,
    stderr: ChildStderr,
    child: Child,
    latest_cache: Vec<ParsedLine>, // cache of lines since last prompt
}

impl CommandCtl {
    pub fn spawn(vsim_command: &str, args: &[&str], cwd: &str) -> Result<Self, Box<dyn Error>> {
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
            .spawn()?;

        let stdout = child.stdout.take().ok_or("Expected stdout")?;
        let reader = BufReader::new(stdout);
        let parser = Parser::new(reader);
        let stdin = child.stdin.take().ok_or("Expected stdin")?;
        let stderr = child.stderr.take().ok_or("Expected stderr")?;

        println!("Started simulator process with PID {}", child.id());
        if let Some(status) = child.try_wait()? {
            return Err(format!(
                "Simulator exited immediately with status {}. CMD: {} {}\n CWD: {}",
                status,
                vsim_command,
                args.join(" "),
                cwd
            )
            .into());
        }

        Ok(Self {
            parser,
            stdin,
            stderr,
            child,
            latest_cache: Vec::new(),
        })
    }

    pub fn wait_until_prompt_startup(&mut self) -> Result<(), Box<dyn Error>> {
        self.wait_until_prompt(Duration::from_secs(2)).map_err(|e| {
            format!("Timeout waiting for prompt during startup. Is the simulator installed and accessible via PATH? Error: {}", e).into()
        })
    }

    pub fn wait_until_prompt(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
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
        Err("Timeout waiting for prompt".into())
    }

    pub fn send_command(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        self.send_command_no_wait(command)?;
        self.wait_until_prompt(Duration::from_millis(500))?;
        if self.has_error() {
            return Err("Simulator reported an error".into());
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

    pub fn send_command_no_wait(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        self.stdin.write_all(command.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    pub fn send_and_expect_result(
        &mut self,
        command: &str,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, Box<dyn Error>> {
        self.send_command(command)?;
        self.get_log_matches(predicate)
    }

    pub fn get_log_matches(
        &mut self,
        predicate: impl Fn(&str) -> bool,
    ) -> Result<Vec<&ParsedLine>, Box<dyn Error>> {
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

    pub fn finish(&mut self) -> Result<(), Box<dyn Error>> {
        self.stdin.write_all(b"quit\n")?;
        let _ = self.stdin.flush();

        let mut stderr_reader = BufReader::new(&mut self.stderr);
        let mut buf: String = String::new();
        let lines = stderr_reader.read_to_string(&mut buf)?;
        if lines > 0 {
            println!("Simulator stderr output:\n{}", buf);
        }

        thread::sleep(Duration::from_millis(100));
        if self.child.try_wait()?.is_none() {
            println!("Killing process");
            self.child.kill()?;
            self.child.wait()?;
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
