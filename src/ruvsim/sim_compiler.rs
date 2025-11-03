use std::{
    error::Error,
    path::Path,
    process::{Command},
};

use std::io::Error as IOError;

pub enum CompilerCommand {
    Vlog,
    Vcom,
}

pub struct Compiler {
    work_dir: String,
    work_name: String,
    current_dir: String,
    modelsim_path: String,
    compiler_command: CompilerCommand,
    compiler_args: Vec<String>,
}

impl Compiler {
    pub fn new(work_dir: &str, modelsim_path: &str, compiler_command: CompilerCommand) -> Self {
        // Create directory if it doesn't exist
        if !Path::new(work_dir).exists() {
            std::fs::create_dir_all(work_dir)
                .expect("Failed to create work directory for compiler");
        }

        Compiler {
            work_dir: work_dir.to_string(),
            modelsim_path: modelsim_path.to_string(),
            compiler_command,
            compiler_args: Vec::new(),
            work_name: "work".to_string(),
            current_dir: ".".to_string(),
        }
    }

    pub fn enable_system_verilog(mut self) -> Self {
        // Check compiler
        if let CompilerCommand::Vcom = self.compiler_command {
            panic!("SystemVerilog is not supported for VHDL compilation (vcom)");
        }
        self.compiler_args.push("-sv".to_string());
        self
    }

    pub fn add_arg(mut self, arg: &str) -> Self {
        self.compiler_args.push(arg.to_string());
        self
    }
    pub fn add_optimization(mut self, level: u8) -> Self {
        match level {
            0 => self.compiler_args.push("-O0".to_string()),
            1 => self.compiler_args.push("-O1".to_string()),
            4 => self.compiler_args.push("-O4".to_string()),
            5 => self.compiler_args.push("-O5".to_string()),
            _ => panic!("Invalid optimization level: {}", level),
        }
        self
    }
    pub fn set_current_dir(mut self, dir: &str) -> Self {
        self.current_dir = dir.to_string();
        self
    }

    pub fn add_dependency(mut self, dep: &str) -> Self {
        // Check if file exists
        if !Path::new(dep).exists() {
            panic!("Dependency file {} does not exist", dep);
        }

        self.compiler_args.push(dep.to_string());
        self
    }

    pub fn add_dependencies(mut self, deps: &[String]) -> Self {
        for dep in deps {
            self = self.add_dependency(dep);
        }
        self
    }

    pub fn add_work_library(mut self, lib: &str) -> Self {
        self.compiler_args.push("-work".to_string());
        self.compiler_args.push(lib.to_string());
        self
    }

    pub fn set_work(mut self, lib: &str) -> Self {
        self.work_name = lib.to_string();
        self
    }

    fn create_library_command(&self, lib: &str) -> Result<(), Box<dyn Error>> {
        // Create library
        let ms_path = Path::new(&self.modelsim_path);
        let vlib_bin = Path::join(ms_path, "vlib");

        let child = Command::new(vlib_bin.to_str().unwrap())
            .current_dir(&self.work_dir)
            .arg(lib)
            .spawn()?;

        // Wait to complete
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(format!(
                "Failed to create library {}: {}",
                lib,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    pub fn run(self) -> Result<(), IOError> {
        // First, create the work library if it doesn't exist
        if !Path::join(Path::new(&self.work_dir), &self.work_name).exists() {
            log::info!(
                "Work library {} does not exist. Creating...",
                &self.work_name
            );
            // Create work_dir if it doesnt exist
            if !Path::new(&self.work_dir).exists() {
                log::info!("Creating work directory: {}", &self.work_dir);
                std::fs::create_dir_all(&self.work_dir)
                    .expect("Failed to create work directory for compiler");
            }

            // Create work library
            self.create_library_command(&self.work_name)
                .expect("Failed to create work library");
        }

        // Then, run the compilation command
        let command_str = match self.compiler_command {
            CompilerCommand::Vlog => "vlog",
            CompilerCommand::Vcom => "vcom",
        };

        let work_name = &self.work_name;
        let work_dir = &self.work_dir;
        let compiler_args = &self.compiler_args;
        let current_dir = &self.current_dir;

        let compiler_path = Path::join(Path::new(&self.modelsim_path), command_str);
        let mut command = Command::new(compiler_path.to_str().unwrap());

        command.current_dir(current_dir);
        command.arg("-work");
        command.arg(Path::join(Path::new(work_dir), work_name));

        for arg in compiler_args {
            command.arg(arg);
        }

        log::info!("Running compiler command: {:?}", command,);
        let mut child = command.spawn()?;
        let status = child.wait()?;
        if !status.success() {
            return Err(IOError::new(
                std::io::ErrorKind::Other,
                format!(
                    "Compilation failed with exit code: {}",
                    status.code().unwrap_or(-1)
                ),
            ));
        }
        Ok(())
    }
}
