use std::{thread, time::Duration};

use ruvsim::{sim_compiler::{Compiler, CompilerCommand}, sim_runner::Runner};


const SOURCE_DIR: &str = "examples/dummy/";
const BUILD_DIR: &str = "build/dummy";
const WORK_LIB : &str = "work_dummy";

fn get_source_files_from_dir(dir: &str, extensions: &[&str]) -> Vec<String> {
    let mut source_files = Vec::new();

    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap()) {
                    source_files.push(path.display().to_string());
                }
            }
        }
    }
    source_files
}

fn compile_testbench() {
    // Get all sources
    let source_directories  = ["examples/hdl", SOURCE_DIR];
    let mut sources = Vec::new();
    for dir in &source_directories {
        let src_files = get_source_files_from_dir(dir, &["v", "sv"]);
        for f in src_files {
            sources.push(f);
        }
    }

    // Assumes you can access modelsim via PATH
    // This initializes the compiler
    let mut compiler = Compiler::new(BUILD_DIR, "", CompilerCommand::Vlog).set_work(WORK_LIB);
    for f in sources {
        compiler = compiler.add_dependency(&f);
    }
    compiler.run().expect("Failed to compile testbench");
}

fn run_sim() {

    // Allow simulator to start
    let mut runner: Runner = Runner::new(
        "vsim",
        &vec!["-work", WORK_LIB, "-voptargs=+acc", "Dummy_tb", "-c"],
        BUILD_DIR,
    );
    thread::sleep(Duration::from_millis(500));
    runner.wait_until_prompt_startup().expect("No prompt");

    // Log everything
    runner.send_command("log -r /*").ok();
    runner.run_all().expect("Failed to run all");

    // run_all finishes the simulation,
    runner.get_log_matches( |line| line.starts_with("Starting testbench")).expect("Failed to find log line");
    runner.get_log_matches( |line| line.starts_with("# 123")).expect("Failed to find log line");
    runner.get_log_matches( |line| line.starts_with("a = 10")).expect("Failed to find log line");
    runner.get_log_matches( |line| line.starts_with("a = 20")).expect("Failed to find log line");

    println!("Dummy_tb ran successfully.");

}

fn main() {
    compile_testbench();
    run_sim();
}