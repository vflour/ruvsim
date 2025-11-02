use std::thread;
use std::time::Instant;

use ruvsim::{sim_runner, sim_types};
use sim_runner::Runner;
use sim_types::{SimRadix, SimSignalDirection};

fn main() {
    let work_dir =
        std::env::var("HDL_WORK_DIR").unwrap_or_else(|_| env!("HDL_WORK_DIR").to_string());
    let mut runner: Runner = Runner::new("vsim", vec!["-work", &work_dir, "SDRAM_tb"], &work_dir);

    println!("Running SDRAM_tb testbench...");
    thread::sleep(std::time::Duration::from_secs(2));
    let _ = runner
        .wait_until_prompt_startup()
        .expect("Failed to detect prompt");

    let nets = runner
        .get_directed_nets_at_path("/*", SimSignalDirection::Output)
        .expect("Failed to get nets");
    for net in nets[..].iter() {
        println!("Detected net: {}", net.name);
        for driver in net.drivers[..].iter() {
            println!("  Driver: {} ({:?})", driver.source, driver.driver_type);
        }
    }
    for mut net in nets {
        let radix = SimRadix::Binary;
        runner
            .examine_net(&mut net, radix)
            .expect("Failed to examine net");
        println!(
            "  Initial value: {:?}",
            net.value.unwrap_or((radix, "undefined".to_string()))
        );
    }

    // Measure time
    let start_time = Instant::now();
    runner.run_all().expect("Failed to run sim to completion");
    let duration = start_time.elapsed();
    println!("Ran in: {:?}", duration);

    // Check completion banner from SDRAM_tb
    runner
        .get_log_matches(|line: &str| line.contains("SDRAM_tb completed successfully"))
        .expect("Failed to find SDRAM_tb completion log");
    // Sanity: ensure a READ happened (look for the TB read printout)
    let read_lines = runner
        .get_log_matches(|line: &str| line.contains("[TB] Read data:"))
        .unwrap_or_default();
    if read_lines.is_empty() {
        panic!("Did not observe any [TB] Read data lines in transcript");
    }

    let _ = runner.finish();
}
