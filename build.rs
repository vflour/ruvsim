use std::fs;
use std::path::{Path, PathBuf};

fn get_hdl_dependencies() -> Vec<PathBuf> {
    // Get list of .sv files in /test/hdl
    let hdl_dir = Path::new("test/hdl");

    let sv_files = fs::read_dir(hdl_dir)
        .expect("Failed to read HDL directory")
        .filter_map(|entry| {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sv") && path.is_file() {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>();

    sv_files
}

fn main() {
    let hdl_files = get_hdl_dependencies();
    for file in hdl_files {
        println!("cargo::rerun-if-changed={}", file.display());
    }
    // Run cmake from CMakeLists.txt in project root
    let out = cmake::Config::new(".").build();

    // Compute the Questa work library directory created by CMake
    // CMake uses ${CMAKE_CURRENT_BINARY_DIR}/build and names the library 'work_SDRAM_tb'
    let work_dir = out.join("build").join("build").join("work_SDRAM_tb");

    // Expose the path to Rust code at compile time
    println!("cargo:rustc-env=HDL_WORK_DIR={}", work_dir.display());
}
