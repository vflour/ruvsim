use std::{thread, time::Duration};

use ruvsim::sim_compiler::{Compiler, CompilerCommand};
use ruvsim::sim_runner::Runner;

const SOURCE_DIR: &str = "examples/breakpoints/";
const BUILD_DIR: &str = "build/breakpoints";
const WORK_LIB: &str = "work_break";

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

fn compile_tb() {
    let mut compiler = Compiler::new(BUILD_DIR, "", CompilerCommand::Vlog).set_work(WORK_LIB);
    for f in get_source_files_from_dir(SOURCE_DIR, &["sv"]) {
        compiler = compiler.add_dependency(&f);
    }
    compiler.run().expect("Compile failed");
}

fn demo_breakpoints() {
    let mut runner = Runner::new(
        "vsim",
        &vec!["-work", WORK_LIB, "-voptargs=+acc", "Break_tb", "-c"],
        BUILD_DIR,
    );
    thread::sleep(Duration::from_millis(300));
    runner.wait_until_prompt_startup().expect("No vsim prompt");

    // Create breakpoints at two source lines (adjust if needed based on line numbers)
    // We expect Break_tb.sv line ~13 (inside repeat loop) and line ~19 (before finish)
    let bp1 = runner
        .create_breakpoint("Break_tb.sv", 10)
        .expect("bp1 failed");
    let bp2 = runner
        .create_breakpoint("Break_tb.sv", 15)
        .expect("bp2 failed");

    println!("Created breakpoints: {} and {}", bp1.name, bp2.name);

    // List breakpoints
    let list = runner.list_breakpoints().expect("list failed");
    println!("Listing breakpoints ({}):", list.len());
    for bp in &list {
        println!("  {}:{}", bp.file.display(), bp.line_num);
    }

    // Run to first breakpoint
    runner.run_continue().expect("run to first bp failed");

    // Stack trace
    let out = runner
        .send_and_expect_result("puts \"TRACEBACK: [tb]\"", |output| {
            output.starts_with("TRACEBACK:")
        })
        .expect("stack trace failed");
    println!("Stack trace at first breakpoint:\n{}", out[0].content);

    // Run status
    println!(
        "Status after run_continue: {}",
        runner.get_status().unwrap_or("<unknown>".to_string())
    );
    println!(
        "Time after first stop: {}",
        runner.get_time().unwrap_or("<unknown>".to_string())
    );

    // Delete first breakpoint
    runner
        .delete_breakpoint("Break_tb.sv", 10)
        .expect("delete bp1 failed");
    println!("Deleted breakpoint at line 10");

    // List again
    let list_after = runner.list_breakpoints().expect("list after delete failed");
    println!("Remaining breakpoints ({}):", list_after.len());
    for bp in &list_after {
        println!("  {}:{}", bp.file.display(), bp.line_num);
    }

    // Continue until second breakpoint
    runner.run_continue().expect("run to second bp failed");
    println!(
        "Status after second stop: {}",
        runner.get_status().unwrap_or("<unknown>".to_string())
    );
    println!(
        "Time after second stop: {}",
        runner.get_time().unwrap_or("<unknown>".to_string())
    );

    // Clean up second breakpoint
    runner.delete_breakpoint("Break_tb.sv", 15).ok();
    println!("Deleted breakpoint at line 15");

    // Finish simulation
    runner.run_all().ok();
    println!("Breakpoint demo finished.");
}

fn main() {
    compile_tb();
    demo_breakpoints();
}
