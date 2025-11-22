"""Type stubs for ruvsim - Rust-based ModelSim/QuestaSim interface"""

from typing import Optional, Literal

def version() -> str:
    """Return the version of the ruvsim package."""
    ...

class PyDriver:
    """Represents a signal driver in the simulation."""
    
    @property
    def driver_type(self) -> str:
        """The type of the driver (e.g., 'Wire', 'Reg')."""
        ...
    
    @property
    def source(self) -> str:
        """The source location of the driver."""
        ...

class PySignal:
    """Represents a signal in the simulation."""
    
    @property
    def name(self) -> str:
        """The name of the signal."""
        ...
    
    @property
    def value(self) -> str:
        """The current value of the signal as a string."""
        ...
    
    @property
    def radix(self) -> str:
        """The radix/base of the signal value (e.g., 'Binary', 'Hexadecimal')."""
        ...
    
    @property
    def drivers(self) -> list[PyDriver]:
        """List of drivers for this signal."""
        ...
    
    @property
    def direction(self) -> str:
        """The direction of the signal ('Input', 'Output', 'Inout', 'Unknown')."""
        ...
    
    @property
    def left_bound(self) -> int:
        """The left bound of the signal range."""
        ...
    
    @property
    def right_bound(self) -> int:
        """The right bound of the signal range."""
        ...
    
    def numeric_value(self) -> Optional[int]:
        """
        Get the numeric value of the signal.
        
        Returns:
            The signal value as an integer, or None if conversion fails.
        """
        ...

class PyBreakpoint:
    """Represents a breakpoint in the simulation."""
    
    @property
    def name(self) -> str:
        """The name/identifier of the breakpoint."""
        ...
    
    @property
    def file(self) -> str:
        """The file where the breakpoint is set."""
        ...
    
    @property
    def line_num(self) -> int:
        """The line number where the breakpoint is set."""
        ...

class PyMemory:
    """Represents a memory block in the simulation."""
    
    @property
    def name(self) -> str:
        """The name of the memory block."""
        ...
    
    @property
    def left_bound(self) -> int:
        """The left bound of the memory address range."""
        ...
    
    @property
    def right_bound(self) -> int:
        """The right bound of the memory address range."""
        ...

class PyParsedLine:
    """Represents a parsed line from the simulator output."""
    
    @property
    def content(self) -> str:
        """The content of the parsed line."""
        ...
    
    @property
    def line_type(self) -> str:
        """The type of the line (e.g., 'Info', 'Warning', 'Error')."""
        ...

class PyRunner:
    """
    Main interface for controlling the ModelSim/QuestaSim simulator.
    
    This class provides methods to control simulation execution, examine and
    manipulate signals, manage breakpoints, and interact with the simulator.
    """
    
    def __init__(self, vsim_command: str, args: list[str], cwd: str) -> None:
        """
        Initialize a new simulator runner.
        
        Args:
            vsim_command: Path to the vsim executable
            args: Command-line arguments to pass to vsim
            cwd: Current working directory for the simulator
        """
        ...
    
    def wait_until_prompt_ms(self, timeout_ms: int) -> None:
        """
        Wait for the simulator prompt with a timeout.
        
        Args:
            timeout_ms: Timeout in milliseconds
            
        Raises:
            RuntimeError: If timeout is reached or other error occurs
        """
        ...
    
    def wait_until_prompt_startup(self) -> None:
        """
        Wait for the initial simulator startup prompt.
        
        Raises:
            RuntimeError: If an error occurs during startup
        """
        ...
    
    def send_command(self, command: str) -> None:
        """
        Send a command to the simulator and wait for completion.
        
        Args:
            command: The TCL command to send
            
        Raises:
            RuntimeError: If command execution fails
        """
        ...
    
    def send_command_no_wait(self, command: str) -> None:
        """
        Send a command to the simulator without waiting for completion.
        
        Args:
            command: The TCL command to send
            
        Raises:
            RuntimeError: If command sending fails
        """
        ...
    
    def run_for(self, time: int, unit: str) -> None:
        """
        Run the simulation for a specified time period.
        
        Args:
            time: The amount of time to run
            unit: Time unit ('fs', 'ps', 'ns', 'us', 'ms', 's')
            
        Raises:
            RuntimeError: If simulation run fails
        """
        ...
    
    def run_all(self) -> None:
        """
        Run the simulation until completion.
        
        Raises:
            RuntimeError: If simulation run fails
        """
        ...
    
    def run_next(self) -> None:
        """
        Step to the next simulation event.
        
        Raises:
            RuntimeError: If simulation step fails
        """
        ...
    
    def run_continue(self) -> None:
        """
        Continue simulation execution (used after hitting a breakpoint).
        
        Raises:
            RuntimeError: If simulation continue fails
        """
        ...
    
    def finish(self) -> None:
        """
        Finish the simulation.
        
        Raises:
            RuntimeError: If finish command fails
        """
        ...
    
    def get_time(self) -> str:
        """
        Get the current simulation time.
        
        Returns:
            The current simulation time as a string
            
        Raises:
            RuntimeError: If time query fails
        """
        ...
    
    def get_status(self) -> str:
        """
        Get the current simulation status.
        
        Returns:
            The simulation status string
            
        Raises:
            RuntimeError: If status query fails
        """
        ...
    
    def get_signal(self, path: str, direction: Optional[str] = None) -> Optional[PySignal]:
        """
        Get a single signal by path.
        
        Args:
            path: The hierarchical path to the signal
            direction: Optional direction filter ('in', 'out', 'inout')
            
        Returns:
            The signal if found, None otherwise
            
        Raises:
            RuntimeError: If signal query fails
        """
        ...
    
    def get_signals(self, path: Optional[str] = None, direction: Optional[str] = None) -> list[PySignal]:
        """
        Get multiple signals matching a path pattern.
        
        Args:
            path: Optional hierarchical path pattern (default: '*' for all signals)
            direction: Optional direction filter ('in', 'out', 'inout')
            
        Returns:
            List of matching signals
            
        Raises:
            RuntimeError: If signal query fails
        """
        ...
    
    def examine_signal(self, signal: PySignal, radix: str) -> None:
        """
        Examine a signal and update its value in the specified radix.
        
        Args:
            signal: The signal to examine
            radix: Display radix ('binary'/'bin', 'octal'/'oct', 'decimal'/'dec', 
                   'hex'/'hexadecimal', 'unsigned'/'uns')
            
        Raises:
            RuntimeError: If examine fails
            ValueError: If radix is invalid
        """
        ...
    
    def examine_signals_batch(self, signals: list[PySignal], radix: str) -> None:
        """
        Examine multiple signals in batch for better performance.
        
        Args:
            signals: List of signals to examine
            radix: Display radix for all signals
            
        Raises:
            RuntimeError: If examine fails
            ValueError: If radix is invalid
        """
        ...
    
    def examine_signal_int_64(self, signal: PySignal) -> int:
        """
        Examine a signal and return its value as a 64-bit integer.
        
        Args:
            signal: The signal to examine
            
        Returns:
            The signal value as an integer
            
        Raises:
            RuntimeError: If examine fails or value cannot be converted
        """
        ...
    
    def examine_signal_int_128(self, signal: PySignal) -> int:
        """
        Examine a signal and return its value as a 128-bit integer.
        
        Args:
            signal: The signal to examine
            
        Returns:
            The signal value as an integer
            
        Raises:
            RuntimeError: If examine fails or value cannot be converted
        """
        ...
    
    def force_signal(self, signal: PySignal, value: str, radix: str) -> None:
        """
        Force a signal to a specific value.
        
        Args:
            signal: The signal to force
            value: The value to force (as a string in the specified radix)
            radix: The radix of the value
            
        Raises:
            RuntimeError: If force fails
            ValueError: If radix is invalid
        """
        ...
    
    def force_signals_batch(self, signals: list[PySignal], values: list[str], radices: list[str]) -> None:
        """
        Force multiple signals in batch for better performance.
        
        Args:
            signals: List of signals to force
            values: List of values (one per signal)
            radices: List of radices (one per signal)
            
        Raises:
            RuntimeError: If lists have different lengths or force fails
            ValueError: If any radix is invalid
        """
        ...
    
    def force_signal_update(self, signal: PySignal) -> None:
        """
        Update a forced signal (propagate the forced value).
        
        Args:
            signal: The signal to update
            
        Raises:
            RuntimeError: If update fails
        """
        ...
    
    def restart(self) -> None:
        """
        Restart the simulation.
        
        Raises:
            RuntimeError: If restart fails
        """
        ...
    
    def reset(self) -> None:
        """
        Reset the simulation.
        
        Raises:
            RuntimeError: If reset fails
        """
        ...
    
    def create_breakpoint(self, filename: str, line_num: int) -> PyBreakpoint:
        """
        Create a breakpoint at a specific file and line.
        
        Args:
            filename: The source file name
            line_num: The line number
            
        Returns:
            The created breakpoint
            
        Raises:
            RuntimeError: If breakpoint creation fails
        """
        ...
    
    def list_breakpoints(self) -> list[PyBreakpoint]:
        """
        List all breakpoints.
        
        Returns:
            List of all breakpoints
            
        Raises:
            RuntimeError: If listing fails
        """
        ...
    
    def delete_breakpoint(self, file: str, line_num: int) -> None:
        """
        Delete a breakpoint.
        
        Args:
            file: The source file name
            line_num: The line number
            
        Raises:
            RuntimeError: If deletion fails
        """
        ...
    
    def get_log_matches_contains(self, needle: str) -> list[str]:
        """
        Get log lines containing a specific substring.
        
        Args:
            needle: The substring to search for
            
        Returns:
            List of matching log lines
            
        Raises:
            RuntimeError: If search fails
        """
        ...
    
    def send_and_expect_contains(self, command: str, needle: str) -> list[str]:
        """
        Send a command and return output lines containing a specific substring.
        
        Args:
            command: The command to send
            needle: The substring to search for in output
            
        Returns:
            List of matching output lines
            
        Raises:
            RuntimeError: If command fails
        """
        ...
    
    def get_log_matches_regex(self, pattern: str) -> list[str]:
        """
        Get log lines matching a regular expression.
        
        Args:
            pattern: The regex pattern to match
            
        Returns:
            List of matching log lines
            
        Raises:
            RuntimeError: If search fails
            ValueError: If regex pattern is invalid
        """
        ...
    
    def send_and_expect_regex(self, command: str, pattern: str) -> list[str]:
        """
        Send a command and return output lines matching a regular expression.
        
        Args:
            command: The command to send
            pattern: The regex pattern to match in output
            
        Returns:
            List of matching output lines
            
        Raises:
            RuntimeError: If command fails
            ValueError: If regex pattern is invalid
        """
        ...
    
    def parsed_buffer(self) -> list[PyParsedLine]:
        """
        Get all parsed lines from the simulator output buffer.
        
        Returns:
            List of all parsed lines
        """
        ...
    
    def latest_buffer(self) -> list[PyParsedLine]:
        """
        Get the latest parsed lines from the simulator output buffer.
        
        Returns:
            List of latest parsed lines
        """
        ...
    
    def list_mems(self) -> list[PyMemory]:
        """
        List all memory blocks in the simulation.
        
        Returns:
            List of memory blocks
            
        Raises:
            RuntimeError: If listing fails
        """
        ...

class PyCompiler:
    """
    Interface for compiling HDL code with ModelSim/QuestaSim compilers (vlog/vcom).
    
    This class provides a builder-style interface for configuring and running
    the HDL compiler.
    """
    
    def __init__(self, work_dir: str, modelsim_path: str, command: Literal["vlog", "vcom"]) -> None:
        """
        Initialize a new compiler.
        
        Args:
            work_dir: Working directory for compilation
            modelsim_path: Path to ModelSim/QuestaSim installation
            command: Compiler command ('vlog' for Verilog/SystemVerilog, 'vcom' for VHDL)
            
        Raises:
            ValueError: If command is not 'vlog' or 'vcom'
        """
        ...
    
    def enable_system_verilog(self) -> None:
        """
        Enable SystemVerilog support (only valid for vlog).
        
        Raises:
            ValueError: If called on a vcom compiler
        """
        ...
    
    def add_arg(self, arg: str) -> None:
        """
        Add a custom argument to the compiler command.
        
        Args:
            arg: The argument to add
            
        Raises:
            RuntimeError: If compiler has already been run
        """
        ...
    
    def add_optimization(self, level: int) -> None:
        """
        Set the optimization level.
        
        Args:
            level: Optimization level (O0, O3, O4, O5)
            
        Raises:
            RuntimeError: If compiler has already been run
        """
        ...
    
    def set_current_dir(self, dir: str) -> None:
        """
        Set the current directory for compilation.
        
        Args:
            dir: The directory path
            
        Raises:
            RuntimeError: If compiler has already been run
        """
        ...
    
    def add_dependency(self, dep: str) -> None:
        """
        Add a single HDL source file dependency.
        
        Args:
            dep: Path to the dependency file
            
        Raises:
            RuntimeError: If compiler has already been run
            FileNotFoundError: If dependency file does not exist
        """
        ...
    
    def add_dependencies(self, deps: list[str]) -> None:
        """
        Add multiple HDL source file dependencies.
        
        Args:
            deps: List of paths to dependency files
            
        Raises:
            RuntimeError: If compiler has already been run
            FileNotFoundError: If any dependency file does not exist
        """
        ...
    
    def add_work_library(self, lib: str) -> None:
        """
        Add a work library to search path.
        
        Args:
            lib: Library name
            
        Raises:
            RuntimeError: If compiler has already been run
        """
        ...
    
    def set_work(self, lib: str) -> None:
        """
        Set the target work library.
        
        Args:
            lib: Library name
            
        Raises:
            RuntimeError: If compiler has already been run
        """
        ...
    
    def run(self) -> None:
        """
        Execute the compilation.
        
        This consumes the compiler object - it cannot be reused after calling run().
        
        Raises:
            RuntimeError: If compilation fails or compiler has already been run
        """
        ...
