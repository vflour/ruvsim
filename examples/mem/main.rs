use ruvsim::{
    sim_compiler::{Compiler, CompilerCommand},
    sim_runner::Runner,
    sim_types::{SimRadix, SimTimeUnit},
};

// Example memory testbench runner template.
// Fill in TB_TOP with your testbench top module name and add any
// specific simulator commands in run_sim() where marked.

const SOURCE_DIR: &str = "examples/mem/"; // Directory containing Mem_tb.sv and related files
const HDL_DIRS: [&str; 1] = ["examples/hdl"]; // Add more HDL source dirs as needed
const BUILD_DIR: &str = "build/mem"; // Output build directory
const WORK_LIB: &str = "work_mem"; // Work library name
const TB_TOP: &str = "Mem_tb"; // Change if your top-level changes
const SIM_EXEC: &str = "vsim"; // Assumes ModelSim/Questa is in PATH

fn get_source_files_from_dir(dir: &str, extensions: &[&str]) -> Vec<String> {
    let mut source_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str) {
                            source_files.push(path.display().to_string());
                        }
                    }
                }
            }
        }
    }
    source_files
}

fn compile_testbench() {
    // Collect HDL sources
    let mut sources: Vec<String> = Vec::new();
    for dir in HDL_DIRS.iter().chain(std::iter::once(&SOURCE_DIR)) {
        let mut files = get_source_files_from_dir(dir, &["v", "sv"]);
        sources.append(&mut files);
    }

    // Initialize compiler
    let mut compiler = Compiler::new(BUILD_DIR, "", CompilerCommand::Vlog).set_work(WORK_LIB);
    for src in sources {
        compiler = compiler.add_dependency(&src);
    }
    compiler.run().expect("Compilation failed");

    // Copy mem.txt to build directory
    let mem_src = format!("{}/mem.txt", SOURCE_DIR);
    let mem_dst = format!("{}/mem.txt", BUILD_DIR);
    std::fs::copy(&mem_src, &mem_dst).expect("Failed to copy mem.txt");
}

fn run_sim() {
    // Basic runner setup; add flags or change top module as needed
    let mut runner = Runner::new(
        SIM_EXEC,
        &vec!["-work", WORK_LIB, "-voptargs=+acc", TB_TOP, "-c"],
        BUILD_DIR,
    );

    // Wait for simulator prompt (optional early exit handling)
    runner
        .wait_until_prompt_startup()
        .expect("Simulator did not start");

    // List all memories
    let mems = runner.list_mems().expect("Failed to list memories");
    println!("Memories in the simulation:");
    for mem in mems {
        println!(
            "    Memory: {}, Size: [{}:{}]",
            mem.name, mem.size.left, mem.size.right
        );
    }
    // Set the signal 'a' to 42
    let a_in = runner
        .get_signal("/Mem_tb/a", Option::None)
        .expect("Failed to get signal 'a'");
    runner
        .force_signal(&a_in, "42", SimRadix::Decimal)
        .expect("Failed to set signal 'a' to 42");

    // Run simulation
    runner.run_for(1000, SimTimeUnit::Ns).expect("Run failed");

    // View data output
    let mut q_out = runner
        .get_signal("/Mem_tb/q", Option::None)
        .expect("Failed to get signal 'q'");
    runner
        .examine_signal(&mut q_out, SimRadix::Decimal)
        .expect("Failed to examine signal 'q'");

    println!(
        "Output signal q after running simulation: {:?}",
        q_out.get_numeric_value::<u64>()
    );

    runner.finish().expect("Failed to finish simulation");
}

fn main() {
    compile_testbench();
    run_sim();
}
