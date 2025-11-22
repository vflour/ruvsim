# ruvsim (Rust-VSim)

A Rust library with Python bindings for controlling ModelSim/Questa simulators programmatically. Provides compilation, simulation control, signal inspection, breakpoint management, and memory access capabilities.

Currently only supports Verilog/SystemVerilog at the moment.

## Rust API 
The Rust API provides direct access to ModelSim/Questa functionality through a type-safe interface.

### Features

#### Compiler
Compile HDL source files (Verilog/SystemVerilog and VHDL), executes both vlib and vlog internally.

```rust
use ruvsim::sim_compiler::{Compiler, CompilerCommand};

let compiler = Compiler::new("build/work_dir", "/path/to/modelsim/bin", CompilerCommand::Vlog)
    .enable_system_verilog()
    .set_work("work_lib")
    .add_dependency("hdl/design.sv")
    .add_dependency("hdl/testbench.sv")
    .add_optimization(5);

compiler.run().expect("Compilation failed");
```

Additionally, you can leave the path to modelsim empty if it's already in your PATH.

```rust
let compiler = Compiler::new("build/work_dir", "", CompilerCommand::Vlog)
    .enable_system_verilog()
    .set_work("work_lib")
    .add_dependency("hdl/design.sv")
    .add_dependency("hdl/testbench.sv")
    .add_optimization(5);

```

#### Runner
Controls the simulation runner; it's essentially a wrapper for the vsim command line interface. 

```rust
use ruvsim::sim_runner::{Runner, RunnerBuilder};

// Create and configure a runner
let mut runner = RunnerBuilder::new()
    .with_vsim_command("vsim")
    .with_cwd("build/work_dir")
    .work_lib("work_lib")
    .top("testbench")
    .enable_acc()
    .batch_mode()
    .build();

runner.wait_until_prompt_startup().expect("Failed to start");

// Control simulation
runner.run_for(100).expect("Run failed");  // Run for 100ns
runner.run_next().expect("Run failed");     // Run to next event
runner.run_all().expect("Run failed");      // Run to completion

// Query simulation state
let time = runner.get_time().expect("Failed to get time");
let status = runner.get_status().expect("Failed to get status");
```

#### Signal Inspection
Read and modify signal values:

```rust
use ruvsim::sim_types::{SimRadix, SimSignalDirection};

// Get signals by path pattern
let signals = runner.get_signals("/top/module/*", Some(SimSignalDirection::Output))
    .expect("Failed to get signals");

// Examine a specific signal
let mut signal = runner.get_signal("/top/module/data", None)
    .expect("Signal not found");
runner.examine_signal(&mut signal, SimRadix::Hexadecimal)
    .expect("Failed to examine");

println!("Signal value: {:?}", signal.value);

// Force a signal to a new value
runner.force_signal(&signal, "ff", SimRadix::Hexadecimal)
    .expect("Failed to force signal");
```

#### Breakpoints
Set and manage simulation breakpoints:

```rust
// Create a breakpoint at a source line
let bp = runner.create_breakpoint("testbench.sv", 42)
    .expect("Failed to create breakpoint");

// Continue to next breakpoint
runner.run_continue().expect("Failed to continue");

// List all breakpoints
let breakpoints = runner.list_breakpoints()
    .expect("Failed to list breakpoints");

// Delete a breakpoint
runner.delete_breakpoint("testbench.sv", 42)
    .expect("Failed to delete");
```

#### Log Parsing
Search and filter simulation output:

```rust
// Find log lines matching a predicate
let matches = runner.get_log_matches(|line| line.contains("ERROR"))
    .expect("Failed to search logs");

// Get all parsed output
let buffer = runner.parsed_buffer();
for line in buffer {
    println!("{:?}: {}", line.line_type, line.content);
}
```

## Python Bindings

Python bindings (Python 3.9+) provide the same functionality with a Pythonic interface.

### Installation

```bash
pip install maturin
maturin develop --release
```

Or build a wheel:
```bash
maturin build --release
pip install target/wheels/ruvsim-*.whl
```

### Usage

#### Compilation

```python
from ruvsim import PyCompiler

# Create and configure compiler
compiler = PyCompiler("build/work_dir", "/path/to/modelsim/bin", "vlog")
compiler.enable_system_verilog()
compiler.set_work("work_lib")
compiler.add_dependencies(["hdl/design.sv", "hdl/testbench.sv"])
compiler.run()
```

#### Simulation Control

```python
from ruvsim import PyRunner

# Create runner
runner = PyRunner(
    "vsim",
    ["-work", "work_lib", "-voptargs=+acc", "testbench", "-c"],
    "build/work_dir"
)

runner.wait_until_prompt_startup()

# Run simulation
runner.send_command("log -r /*")  # Log all signals
runner.run_for(100)               # Run for 100ns
runner.run_next()                 # Run to next event
runner.run_all()                  # Run to completion

# Query state
time = runner.get_time()
status = runner.get_status()
print(f"Time: {time}, Status: {status}")
```

#### Signal Access

```python
# Get signals
nets = runner.get_nets("/top/module/*", direction="Output")

# Examine individual signal
signal = runner.get_net("/top/module/data")
runner.examine_net(signal, "hex")
print(f"Value: {signal.value}, Radix: {signal.radix}")

# Batch examine for performance
signals = runner.get_nets("/top/module/*")
runner.examine_nets_batch(signals, "binary")
for sig in signals:
    print(f"{sig.name}: {sig.value}")

# Force signal values
runner.force_net(signal, "ff", "hex")

# Batch force for performance
runner.force_nets_batch(signals, ["01", "10", "11"], ["bin", "bin", "bin"])

# Get numeric value (for integer signals)
value = signal.numeric_value()  # Returns Python int or None
```

#### Breakpoints

```python
# Create breakpoint
bp = runner.create_breakpoint("testbench.sv", 42)
print(f"Breakpoint: {bp.name} at {bp.file}:{bp.line_num}")

# Continue to breakpoint
runner.run_continue()

# List and delete
breakpoints = runner.list_breakpoints()
runner.delete_breakpoint("testbench.sv", 42)
```

#### Log Searching

```python
# Search by substring
matches = runner.get_log_matches_contains("ERROR")
for match in matches:
    print(match)

# Search by regex
matches = runner.get_log_matches_regex(r"^# Time: \d+")

# Send command and expect output
results = runner.send_and_expect_contains("status", "running")
results = runner.send_and_expect_regex("examine signal", r"32'h[0-9a-f]+")
```

#### Memory Access

```python
# List memory regions
mems = runner.list_mems()
for mem in mems:
    print(f"{mem.name}: [{mem.left_bound}:{mem.right_bound}]")
```

#### Direct Commands

```python
# Send arbitrary TCL commands
runner.send_command("restart -f")
runner.send_command_no_wait("run -all")  # Non-blocking

# Parse output buffer
parsed = runner.parsed_buffer()
for line in parsed:
    print(f"{line.line_type}: {line.content}")

# Get only latest output
latest = runner.latest_buffer()
```

### Python API Classes

- **`PyRunner`**: Main simulation controller
- **`PyCompiler`**: HDL compilation interface
- **`PySignal`**: Signal object with properties (name, value, radix, direction, bounds, drivers)
- **`PyDriver`**: Signal driver information
- **`PyBreakpoint`**: Breakpoint representation
- **`PyMemory`**: Memory region information
- **`PyParsedLine`**: Parsed simulator output line

### Example: Complete Workflow

```python
import os
from ruvsim import PyCompiler, PyRunner

# Setup directories
build_dir = "build/my_design"
os.makedirs(build_dir, exist_ok=True)

# Compile
compiler = PyCompiler(build_dir, "", "vlog")
compiler.enable_system_verilog()
compiler.set_work("work_my_design")
compiler.add_dependencies(["hdl/design.sv", "hdl/tb.sv"])
compiler.run()

# Run simulation
runner = PyRunner("vsim", 
    ["-work", "work_my_design", "-voptargs=+acc", "tb", "-c"],
    build_dir)
runner.wait_until_prompt_startup()

# Test scenario
runner.send_command("log -r /*")
runner.run_for(1000)

# Check results
matches = runner.get_log_matches_contains("TEST PASSED")
if matches:
    print("✓ Test passed!")
else:
    print("✗ Test failed!")

runner.finish()
```

## RuvSim REST API
Overview
- Axum-based REST API to compile HDL deps, start a ModelSim/Questa session, inspect nets, run simulation steps, and query logs.
- OpenAPI docs served at /docs and /api-docs/openapi.json.

Environment
- RUVSIM_API_ADDR: Bind address (default 0.0.0.0:8080)
- RUVSIM_WORK_DIR: Required. Base directory for compilation and simulator CWD.
- RUVSIM_MODELSIM_PATH: Optional. Path to ModelSim/Questa bin folder for vlib/vlog/vcom.
- RUVSIM_VSIM_BIN: Optional. Simulator executable name (default "vsim").
- RUVSIM_SESSION_TTL_SECS: Optional. Idle eviction TTL in seconds (default 600).

Path policy
- The API rejects absolute dependency paths.
- deps must be relative to RUVSIM_WORK_DIR. The server resolves each dep as "${RUVSIM_WORK_DIR}/${dep}".

Key endpoints
- GET /health
- GET /sessions
- POST /sessions
  Body: { "work_lib": "work_SDRAM_tb", "top": "SDRAM_tb", "deps": ["hdl/SDRAM_tb.sv", "hdl/SDRAM.sv", "hdl/Dummy.sv", "hdl/Dummy_tb.sv"] }
  Returns: { "id": "<uuid>" }
- GET /sessions/{id}/nets?path=/*&direction=All|Input|Output|Inout&examine=true&radix=binary
- POST /sessions/{id}/run  Body: { "mode": "all" } | { "mode": "next" } | { "mode": "for", "ns": 100 }
- GET /sessions/{id}/logs?contains=NETS
- POST /sessions/{id}/examine  Body: { "path": "/top/u0/signal", "radix": "hex" }
- POST /sessions/{id}/cmd  Body: { "command": "run -next", "expect_contains": "VSIM" }
- DELETE /sessions/{id}

Notes
- Compilation uses vlog (SystemVerilog enabled). vcom is not exposed yet.
- work library is created under RUVSIM_WORK_DIR if not present.
- Errors are returned as { "message": "..." } with suitable HTTP status codes.

## Examples

The `examples/` directory contains several complete working examples demonstrating different features of ruvsim. Each example includes both Rust and Python implementations where applicable.

### Dummy - Basic Simulation
**Location:** `examples/dummy/`

**Run:**
```bash
# Rust
cargo run --example dummy

# Python
python examples/dummy/main.py
```

### Memory Access
**Location:** `examples/mem/`

**Run:**
```bash
cargo run --example mem
```

### Breakpoints
**Location:** `examples/breakpoints/`

**Run:**
```bash
cargo run --example breakpoints
```

### SERV MCU - Advanced Integration
**Location:** `examples/serv_mcu/`

**Run:**
```bash
# Rust
cargo run --example serv_mcu

# Python (requires: pip install bronzebeard)
python examples/serv_mcu/main.py
```

### Running Examples

All examples assume ModelSim/Questa is in your system PATH. If not, set the path:

```bash
# For Rust examples
export PATH="/path/to/modelsim/bin:$PATH"
cargo run --example dummy

# For Python examples
python examples/dummy/main.py
```

