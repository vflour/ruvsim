
use super::sim_parser::Parser;

use std::{
    error::Error, io::{BufReader, Read, Write}, path::PathBuf, process::{Child, ChildStderr, ChildStdin, Command, Stdio}, thread, time::{Duration, Instant}
};

use super::sim_types::{LineType, ParsedLine, SimDriver, SimRadix, SimSignal, SimSignalDirection, SimBreakpoint};

pub struct Runner {
    parser: Parser,
    stdin: ChildStdin,
    stderr: ChildStderr,
    child: Child,
}

impl Runner {
    pub fn new(vsim_command: &str, args: &Vec<&str>, cwd: &str) -> Self {
        let mut child = Command::new(vsim_command)
            .arg("-c")
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn program. Is it installed?");

        let stdout = child.stdout.take().expect("Expected stdout");
        let reader = BufReader::new(stdout);
        let parser = Parser::new(reader);

        let stdin = child.stdin.take().expect("Expected stdin");
        let stderr = child.stderr.take().expect("Expected stderr");

        println!("Started simulator process with PID {}", child.id());
        let wait_status = child.try_wait();
        match wait_status {
            Err(e) => panic!("Error checking simulator process status: {}", e),
            Ok(Some(status)) => panic!(
                "Simulator exited immediately with status {}. CMD: {} {}\n CWD: {}",
                status,
                vsim_command,
                args.join(" "),
                cwd
            ),
            _ => { /* Still running as expected */ }
        }

        Runner {
            parser: parser,
            child: child,
            stdin: stdin,
            stderr: stderr,
        }
    }
    pub fn wait_until_prompt_startup(&mut self) -> Result<(), Box<dyn Error>> {
        let result = self.wait_until_prompt(Duration::from_secs(2));
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(format!(
                    "Timeout waiting for prompt during startup. Is the simulator installed and accessible via PATH? Error: {}",
                    e
                )
                .into())
            }
        }
    }

    pub fn wait_until_prompt(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        let at_timeout = Instant::now() + timeout;

        let mut now = Instant::now();
        while at_timeout > now {
            let next_line = self.parser.get_next_prompt()?;
            if next_line.is_some() {
                return Ok(());
            }
            now = Instant::now();
            thread::sleep(Duration::from_millis(10));
        }
        return Err("Timeout waiting for prompt".into());
    }

    pub fn send_command(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        self.send_command_no_wait(command)?;
        self.wait_until_prompt(Duration::from_millis(10))?;
        Ok(())
    }

    pub fn send_command_no_wait(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        let mut stdin = &self.stdin;
        stdin.write_all(command.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn run_all(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command_no_wait("run -all")?;
        Ok(())
    }

    pub fn run_next(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("run -next")?;
        Ok(())
    }

    pub fn run_for(&mut self, ns: u64) -> Result<(), Box<dyn Error>> {
        self.send_command(&format!("run {}ns", ns))?;
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
            if matches!(line.line_type, LineType::Output | LineType::Log) {
                if predicate(&line.content) {
                    matches.push(line);
                }
            }
        }
        Ok(matches)
    }

    pub fn finish(&mut self) -> Result<(), Box<dyn Error>> {

        // Tell MS to terminate
        let mut stdin = &self.stdin;
        stdin.write_all(b"quit\n")?;
        let _ = stdin.flush();

        // Wait a moment for process to exit
        thread::sleep(Duration::from_millis(100));
        let process_finished = self.child.try_wait();
        if process_finished.is_err() {
            println!("Killing process");
            let _ = self.child.kill()?;
        }
        // Report any errors
        let mut stderr_reader = BufReader::new(&mut self.stderr);
        let mut buf: String = String::new();
        let lines = stderr_reader.read_to_string(&mut buf)?;
        if lines > 0 {
            println!("Simulator stderr output:\n{}", buf);
        }

        Ok(())
    }

    pub fn get_directed_nets_at_path(
        &mut self,
        path: &str,
        net_direction: SimSignalDirection,
    ) -> Result<Vec<SimSignal>, Box<dyn Error>> {
        // Get all nets as owned Strings so we don't keep borrowing self
        let paths = self.get_net_paths(path, net_direction)?;
        let mut nets: Vec<SimSignal> = Vec::new();
        for net_path in paths {
            let net_data = self.get_net_data(&net_path, net_direction)?;
            nets.push(net_data);
        }
        Ok(nets)
    }

    pub fn get_all_nets_at_path(&mut self, path: &str) -> Result<Vec<SimSignal>, Box<dyn Error>> {
        let in_nets = self.get_directed_nets_at_path(path, SimSignalDirection::Input)?;
        let out_nets = self.get_directed_nets_at_path(path, SimSignalDirection::Output)?;
        let inout_nets = self.get_directed_nets_at_path(path, SimSignalDirection::Inout)?;

        let mut all_nets = in_nets;
        all_nets.extend(out_nets);
        all_nets.extend(inout_nets);
        Ok(all_nets)
    }

    fn get_net_paths(
        &mut self,
        path: &str,
        net_direction: SimSignalDirection,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let direction_arg = match net_direction {
            SimSignalDirection::Input => "in",
            SimSignalDirection::Output => "out",
            SimSignalDirection::Inout => "inout",
            _ => {
                return Err("Invalid direction for get_net_paths".into());
            }
        };

        let output = self.send_and_expect_result(
            &format!("puts \"NETS: [find nets -{} {}]\"", direction_arg, path),
            |line| line.starts_with("NETS:"),
        )?;
        // Ideally, you want the latest output only
        let line = output.last().unwrap().content.trim_start_matches("NETS:");

        let net_paths = line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        Ok(net_paths)
    }

    fn get_net_data(
        &mut self,
        path: &str,
        net_direction: SimSignalDirection,
    ) -> Result<SimSignal, Box<dyn Error>> {
        // First, get drivers
        let drivers_output = self.send_and_expect_result(
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

        let net_output = self.send_and_expect_result(
            &format!("puts \"NET_DATA: [describe -binary {}]\"", path),
            |line| line.starts_with("NET_DATA: "),
        )?;
        let net_line = net_output
            .last()
            .unwrap()
            .content
            .trim_start_matches("NET_DATA: ");
        let signal = SimSignal::from_describe_output(path, net_direction, drivers, net_line);
        Ok(signal)
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

    pub fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("restart -f")?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("reset -f")?;
        Ok(())
    }

    pub fn create_breakpoint(&mut self, filename: &str, line_num: u32) -> Result<SimBreakpoint, Box<dyn Error>> {
        let breakpoint = SimBreakpoint{
            name: format!("{} {}", filename, line_num),
            file: PathBuf::from(filename),
            line_num,
        };
        self.send_command(&format!("bp {} {}", filename, line_num))?;
        self.send_and_expect_result("bp", |line| line.contains(&breakpoint.name))?;
        
        Ok(breakpoint)
    }

    pub fn run_continue(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_command("run -continue")?;
        Ok(())
    }

    // pub fn
}
