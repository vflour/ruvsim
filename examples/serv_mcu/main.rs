use std::{fs::File, io::Write, path::Path, thread, time::Duration};
use ruvsim::{sim_compiler, sim_runner, sim_types::SimTimeUnit};
use sim_compiler::{Compiler, CompilerCommand};
use sim_runner::Runner;

const SOURCE_DIR: &str = "examples/serv_mcu/";
const BUILD_DIR: &str = "build/serv_mcu";
const WORK_LIB : &str = "work_serv_mcu";

fn init_build_dir() {
    std::fs::create_dir_all(BUILD_DIR).expect("Failed to create build directory");
}

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

fn assemble_mem_image() {
    
    let asm_file = Path::new(SOURCE_DIR).join("serv_mcu.asm");
    let bin_file = Path::new(BUILD_DIR).join("mem.bin");

    // Compile binary first
    let mut bronzebeard_ch = std::process::Command::new("bronzebeard")
        .arg(asm_file.to_str().unwrap())
        .arg("-o")
        .arg(bin_file.to_str().unwrap())
        .spawn()
        .expect("Failed to run bronzebeard. Run pip install bronzebeard if not installed.");

    let status = bronzebeard_ch.wait().expect("Failed to wait on bronzebeard");
    if !status.success() {
        panic!("Bronzebeard assembler failed");
    }

    let text_file = Path::new(BUILD_DIR).join("mem.txt");
    // Read binary, export to text
    let blob = std::fs::read(bin_file.to_str().unwrap()).expect("Failed to read mem.bin");
    let chunks = blob.chunks(4);

    let mut f = File::create(text_file).expect("Failed to create mem.txt");
    for w in chunks {
        let word = w.iter().rev().fold(0, |acc, &b | (acc << 8) | b as u32);
        writeln!(f, "{:08x}", word).expect("Failed to write to mem.txt");
    }

    drop(f);

}


fn compile_testbench() {
    let source_path = Path::new(SOURCE_DIR);
    let serv_path = source_path.join("serv");
    let source_directories  = ["examples/hdl", SOURCE_DIR, serv_path.to_str().unwrap()];

    let mut sources = Vec::new();
    for dir in &source_directories {
        let serv_files = get_source_files_from_dir(dir, &["v", "sv"]);
        for f in serv_files {
            sources.push(f);
        }
    }

    // Assumes you can access modelsim via PATH
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
        &vec!["-work", WORK_LIB, "-voptargs=+acc", "serv_sdram_tb", "-c"],
        BUILD_DIR,
    );
    thread::sleep(Duration::from_millis(500));
    runner.wait_until_prompt_startup().expect("No prompt");

    // Log everything
    runner.send_command("log -r /*").ok();
    // Short run to load memory & release reset
    runner.run_for(1000, SimTimeUnit::Ns).ok();
    // Examine first few instruction words and data bus address
    if let Ok(lines) = runner.send_and_expect_result("examine /serv_sdram_tb/i_ibus_rdt", |l| l.starts_with("# 32'h")) {
        if let Some(line) = lines.last() { println!("Current i_ibus_rdt: {}", line.content); }
    }
    // Run further to reach string address
    runner.run_for(200_000, SimTimeUnit::Ns).ok();

    runner.finish().expect("Finish failed");
    println!("serv_sdram_tb ran successfully.");
}


fn main() {
    init_build_dir();
    assemble_mem_image();
    compile_testbench();
    run_sim();
}
