"""
Type stubs for ruvsim - A Python interface to ModelSim/QuestaSim simulator
"""

from typing import Optional, List

class PySignal:
    """Represents a signal in the simulation"""
    
    @property
    def name(self) -> str:
        """The name/path of the signal"""
        ...
    
    @property
    def value(self) -> str:
        """The current value of the signal as a string"""
        ...
    
    @property
    def radix(self) -> str:
        """The radix used to display the value (Binary, Octal, Decimal, Hexadecimal, Unsigned, or Unknown)"""
        ...
    
    @property
    def drivers(self) -> List['PyDriver']:
        """List of drivers for this signal"""
        ...
    
    @property
    def direction(self) -> str:
        """The direction of the signal (Input, Output, Inout, or Unknown)"""
        ...
    
    @property
    def left_bound(self) -> Optional[int]:
        """The left bound of the signal range (in bits), if applicable"""
        ...
    
    @property
    def right_bound(self) -> Optional[int]:
        """The right bound of the signal range (in bits), if applicable"""
        ...
    
    def numeric_value(self) -> Optional[int]:
        """
        Get the numeric value of the signal as an integer.
        Returns None if the value contains 'x' or 'z' (unknown/high-impedance).
        """
        ...

class PyDriver:
    """Represents a driver of a signal"""
    
    @property
    def driver_type(self) -> str:
        """The type of driver (Object, Assign, Force, or Other)"""
        ...
    
    @property
    def source(self) -> str:
        """The source of the driver"""
        ...

class PyBreakpoint:
    """Represents a simulation breakpoint"""
    
    @property
    def name(self) -> str:
        """The name of the breakpoint"""
        ...
    
    @property
    def file(self) -> str:
        """The file path where the breakpoint is set"""
        ...
    
    @property
    def line_num(self) -> int:
        """The line number where the breakpoint is set"""
        ...

class PyRunner:
    """
    Main simulation runner interface.
    Controls a ModelSim/QuestaSim simulation session.
    """
    
    def __init__(self, vsim_command: str, args: List[str], cwd: str) -> None:
        """
        Create a new simulation runner.
        
        Args:
            vsim_command: Path to the vsim executable
            args: Command-line arguments to pass to vsim
            cwd: Working directory for the simulation
        """
        ...
    
    def wait_until_prompt_ms(self, timeout_ms: int) -> None:
        """
        Wait until a prompt appears in the simulation, with a timeout in milliseconds.
        
        Args:
            timeout_ms: Timeout in milliseconds
        
        Raises:
            RuntimeError: If timeout occurs or other error
        """
        ...
    
    def send_command(self, command: str) -> None:
        """
        Send a TCL command to the simulator.
        
        Args:
            command: The TCL command to send
        
        Raises:
            RuntimeError: If command fails
        """
        ...
    
    def run_for(self, ns: int) -> None:
        """
        Run the simulation for a specified number of nanoseconds.
        
        Args:
            ns: Number of nanoseconds to run
        
        Raises:
            RuntimeError: If simulation fails
        """
        ...
    
    def get_directed_nets_at_path(self, path: str, direction: str) -> List[PySignal]:
        """
        Get all nets at a given path with a specific direction.
        
        Args:
            path: The hierarchical path in the design
            direction: Direction filter ("in"/"input", "out"/"output", or "inout")
        
        Returns:
            List of PySignal objects matching the criteria
        
        Raises:
            RuntimeError: If query fails
            ValueError: If direction is invalid
        """
        ...
    
    def get_all_nets_at_path(self, path: str) -> List[PySignal]:
        """
        Get all nets at a given path, regardless of direction.
        
        Args:
            path: The hierarchical path in the design
        
        Returns:
            List of PySignal objects
        
        Raises:
            RuntimeError: If query fails
        """
        ...
    
    def examine_net(self, signal: PySignal, radix: str) -> None:
        """
        Examine a signal and update its value using the specified radix.
        
        Args:
            signal: The signal to examine
            radix: The radix to use ("binary"/"bin", "octal"/"oct", "decimal"/"dec", 
                   "hex"/"hexadecimal", or "unsigned"/"uns")
        
        Raises:
            RuntimeError: If examination fails
            ValueError: If radix is invalid
        """
        ...

    def examine_nets_batch(self, signals: List[PySignal], radix: str) -> None:
        """
        Examine multiple signals in a single batch operation for improved performance.
        
        This method is significantly faster than calling examine_net() individually 
        for each signal, as it sends all examine commands at once and parses the 
        results in a single operation.
        
        Args:
            signals: List of signals to examine
            radix: The radix to use for all signals ("binary"/"bin", "octal"/"oct", 
                   "decimal"/"dec", "hex"/"hexadecimal", or "unsigned"/"uns")
        
        Raises:
            RuntimeError: If examination fails
            ValueError: If radix is invalid
        
        Example:
            >>> signals = runner.get_all_nets_at_path("/top/*")
            >>> runner.examine_nets_batch(signals, "binary")
            >>> print(signals[0].value)
        """
        ...

    def examine_net_int(self, signal: PySignal) -> int:
        """
        Examine a signal and return its numeric value as an integer.
        
        Args:
            signal: The signal to examine
        Returns:
            The numeric value as an integer
        Raises:
            RuntimeError: If examination fails
        """
        ...

    def force_net(self, signal: PySignal, value: str, radix: str) -> None:
        """
        Force a signal to a specific value in the simulation.
        
        Args:
            signal_name: The hierarchical path of the signal to force
            value: The value to force (e.g., "1", "0", "16#FF", "2#1010")
            radix: The radix of the value ("binary"/"bin", "octal"/"oct", "decimal"/"dec", 
                   "hex"/"hexadecimal", or "unsigned"/"uns")
        
        Raises:
            RuntimeError: If force command fails
        """
        ...

    def force_nets_batch(self, signals: List[PySignal], values: List[str], radices: List[str]) -> None:
        """
        Force multiple signals to specific values in a single batch operation.
        
        This method is significantly faster than calling force_net() individually 
        for each signal, as it sends all force commands at once.
        
        Args:
            signals: List of signals to force
            values: List of values to force (same length as signals)
            radices: List of radices for each value (same length as signals)
                     Each radix can be "binary"/"bin", "octal"/"oct", "decimal"/"dec", 
                     "hex"/"hexadecimal", or "unsigned"/"uns"
        
        Raises:
            RuntimeError: If force command fails or if lists have different lengths
            ValueError: If any radix is invalid
        
        Example:
            >>> signals = runner.get_all_nets_at_path("/top/clk")
            >>> runner.force_nets_batch(signals[:3], ["1", "0", "1"], ["binary", "binary", "binary"])
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
        Create a breakpoint in the simulation.
        
        Args:
            filename: The file containing the breakpoint
            line_num: The line number for the breakpoint
        
        Returns:
            PyBreakpoint object representing the created breakpoint
        
        Raises:
            RuntimeError: If breakpoint creation fails
        """
        ...

    def run_continue(self) -> None:
        """
        Continue running the simulation (typically after hitting a breakpoint).
        
        Raises:
            RuntimeError: If continue fails
        """
        ...
    
    def get_log_matches_contains(self, needle: str) -> List[str]:
        """
        Get log lines that contain a specific substring.
        
        Args:
            needle: The substring to search for
        
        Returns:
            List of matching log lines
        
        Raises:
            RuntimeError: If search fails
        """
        ...
    
    def send_and_expect_contains(self, command: str, needle: str) -> List[str]:
        """
        Send a command and return output lines containing a specific substring.
        
        Args:
            command: The command to send
            needle: The substring to search for in the output
        
        Returns:
            List of matching output lines
        
        Raises:
            RuntimeError: If command fails
        """
        ...
    
    def get_log_matches_regex(self, pattern: str) -> List[str]:
        """
        Get log lines that match a regular expression pattern.
        
        Args:
            pattern: The regex pattern to match
        
        Returns:
            List of matching log lines
        
        Raises:
            RuntimeError: If search fails
            ValueError: If regex pattern is invalid
        """
        ...
    
    def send_and_expect_regex(self, command: str, pattern: str) -> List[str]:
        """
        Send a command and return output lines matching a regular expression.
        
        Args:
            command: The command to send
            pattern: The regex pattern to match in the output
        
        Returns:
            List of matching output lines
        
        Raises:
            RuntimeError: If command fails
            ValueError: If regex pattern is invalid
        """
        ...
    
    def get_nets(self, path: Optional[str] = None) -> List[PySignal]:
        """
        Get all nets at a given path (defaults to all nets with "*").
        
        Args:
            path: The hierarchical path in the design (default: "*")
        
        Returns:
            List of PySignal objects
        
        Raises:
            RuntimeError: If query fails
        """
        ...
    
    def wait_until_prompt_startup(self) -> None:
        """
        Wait for the initial prompt during simulator startup.
        
        Raises:
            RuntimeError: If timeout or error occurs
        """
        ...
    
    def run_all(self) -> None:
        """
        Run the simulation to completion.
        
        Raises:
            RuntimeError: If simulation fails
        """
        ...
    
    def run_next(self) -> None:
        """
        Execute the next statement in the simulation (step).
        
        Raises:
            RuntimeError: If step fails
        """
        ...
    
    def finish(self) -> None:
        """
        Finish the simulation session.
        
        Raises:
            RuntimeError: If finish fails
        """
        ...

class PyCompiler:
    """
    Compiler interface for ModelSim/QuestaSim.
    Supports both vlog (Verilog/SystemVerilog) and vcom (VHDL) commands.
    """
    
    def __init__(self, work_dir: str, modelsim_path: str, command: str) -> None:
        """
        Create a new compiler instance.
        
        Args:
            work_dir: Working directory for compilation
            modelsim_path: Path to ModelSim installation
            command: Compiler command to use ("vlog" or "vcom")
        
        Raises:
            ValueError: If command is not "vlog" or "vcom"
        """
        ...
    
    def enable_system_verilog(self) -> None:
        """
        Enable SystemVerilog support (vlog only).
        
        Raises:
            ValueError: If called on a vcom compiler
            RuntimeError: If compiler already used
        """
        ...
    
    def add_arg(self, arg: str) -> None:
        """
        Add a custom argument to the compiler command.
        
        Args:
            arg: The argument to add
        
        Raises:
            RuntimeError: If compiler already used
        """
        ...
    
    def add_optimization(self, level: int) -> None:
        """
        Set optimization level for compilation.
        
        Args:
            level: Optimization level (0-3 typically)
        
        Raises:
            RuntimeError: If compiler already used
        """
        ...
    
    def set_current_dir(self, dir: str) -> None:
        """
        Set the current directory for compilation.
        
        Args:
            dir: Directory path
        
        Raises:
            RuntimeError: If compiler already used
        """
        ...
    
    def add_dependency(self, dep: str) -> None:
        """
        Add a single source file dependency.
        
        Args:
            dep: Path to the dependency file
        
        Raises:
            FileNotFoundError: If dependency file doesn't exist
            RuntimeError: If compiler already used
        """
        ...
    
    def add_dependencies(self, deps: List[str]) -> None:
        """
        Add multiple source file dependencies.
        
        Args:
            deps: List of paths to dependency files
        
        Raises:
            FileNotFoundError: If any dependency file doesn't exist
            RuntimeError: If compiler already used
        """
        ...
    
    def add_work_library(self, lib: str) -> None:
        """
        Add a work library to the compilation.
        
        Args:
            lib: Library name
        
        Raises:
            RuntimeError: If compiler already used
        """
        ...
    
    def set_work(self, lib: str) -> None:
        """
        Set the work library for compilation.
        
        Args:
            lib: Library name
        
        Raises:
            RuntimeError: If compiler already used
        """
        ...
    
    def run(self) -> None:
        """
        Execute the compilation.
        Note: This consumes the compiler instance and it cannot be reused.
        
        Raises:
            RuntimeError: If compilation fails or compiler already used
        """
        ...

def version() -> str:
    """
    Get the version of the ruvsim package.
    
    Returns:
        Version string
    """
    ...
