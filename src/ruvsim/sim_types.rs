// Output lines
use num_traits::Num;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

// Error types for simulator interaction
#[derive(Debug, Clone)]
pub enum SimError {
    ProcessError(String),
    TimeoutError(String),
    ParseError(String),
    CommandError(String),
    IOError(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::ProcessError(msg) => write!(f, "Process Error: {}", msg),
            SimError::TimeoutError(msg) => write!(f, "Timeout Error: {}", msg),
            SimError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            SimError::CommandError(msg) => write!(f, "Command Error: {}", msg),
            SimError::IOError(msg) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl From<&str> for SimError {
    fn from(s: &str) -> Self {
        SimError::ParseError(s.to_string())
    }
}

impl std::error::Error for SimError {}

// Regexes for each LineType:
fn get_line_type_from_regex(line: &str) -> LineType {
    let prompt_re = regex::Regex::new(r"^(VSIM|Questa|ModelSim)( \d+)?>$").unwrap();
    let exit_re = regex::Regex::new(r"^# End time: (.*), Elapsed time: (.*)$").unwrap();
    let error_re = regex::Regex::new(r"^# (\*\* Error:|wrong # args: should be).*$").unwrap();
    let tcl_comment_re = regex::Regex::new(r"^#.*$").unwrap();

    if prompt_re.is_match(line) {
        LineType::Prompt
    } else if exit_re.is_match(line) {
        LineType::Exit
    } else if error_re.is_match(line) {
        LineType::Error
    } else if tcl_comment_re.is_match(line) {
        LineType::Log
    } else {
        LineType::Output
    }
}

#[derive(Debug, Clone)]
pub enum PromptType {
    VSim,
    Questa,
    ModelSim,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum LineType {
    Unknown,
    Log,
    Output,
    Prompt,
    Exit,
    Error,
}

impl FromStr for LineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Log" => Ok(LineType::Log),
            "Prompt" => Ok(LineType::Prompt),
            "Output" => Ok(LineType::Output),
            "Unknown" => Ok(LineType::Unknown),
            "Error" => Ok(LineType::Error),
            "Exit" => Ok(LineType::Exit),
            _ => Err(format!("Unknown LineType: {}", s)),
        }
    }
}

fn get_prompt_type_from_line(line: &str) -> PromptType {
    if line.starts_with("VSIM") {
        PromptType::VSim
    } else if line.starts_with("Questa") {
        PromptType::Questa
    } else if line.starts_with("ModelSim") {
        PromptType::ModelSim
    } else {
        PromptType::Unknown
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedPrompt {
    pub line: String,
    pub suffix: String,
}

impl ParsedPrompt {
    fn new(line: &str) -> Self {
        let _prompt_type = get_prompt_type_from_line(line);
        let id = {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[1].trim_end_matches('>').parse::<u32>() {
                    Some(num)
                } else {
                    None
                }
            } else {
                None
            }
        };

        ParsedPrompt {
            line: line.to_string(),
            suffix: id.map_or(String::new(), |id| id.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParsedLine {
    pub content: String,
    pub line_type: LineType,
    pub prompt_info: Option<ParsedPrompt>,
}

impl ParsedLine {
    pub fn new(line: &str) -> Self {
        let line_type = get_line_type_from_regex(line);
        let mut prompt_info = None;
        if let LineType::Prompt = line_type {
            prompt_info = Some(ParsedPrompt::new(line));
        }

        ParsedLine {
            line_type,
            content: line.to_string(),
            prompt_info,
        }
    }
}

// VSim Object Types
// Assume Verilog/SV only

// -----------------------
// Simulation run control/lifecycle types
// -----------------------

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum SimTimeUnit {
    Fs,
    Ps,
    Ns,
    Us,
    Ms,
    S,
}

impl fmt::Display for SimTimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimTimeUnit::Fs => write!(f, "fs"),
            SimTimeUnit::Ps => write!(f, "ps"),
            SimTimeUnit::Ns => write!(f, "ns"),
            SimTimeUnit::Us => write!(f, "us"),
            SimTimeUnit::Ms => write!(f, "ms"),
            SimTimeUnit::S => write!(f, "s"),
        }
    }
}

impl FromStr for SimTimeUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fs" => Ok(SimTimeUnit::Fs),
            "ps" => Ok(SimTimeUnit::Ps),
            "ns" => Ok(SimTimeUnit::Ns),
            "us" => Ok(SimTimeUnit::Us),
            "ms" => Ok(SimTimeUnit::Ms),
            "s" => Ok(SimTimeUnit::S),
            _ => Err(format!("Unknown SimTimeUnit: {}", s)),
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct SimTime {
    pub value: u64,
    pub unit: SimTimeUnit,
}

impl SimTime {
    #[allow(dead_code)]
    pub fn new(value: u64, unit: SimTimeUnit) -> Self {
        Self { value, unit }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimBreakpoint {
    pub name: String,
    pub file: PathBuf,
    pub line_num: u32,
}

#[allow(non_camel_case_types)]
pub enum SimObjectType {
    V__Function,
    V__ModuleInstance,
    V__NamedFork,
    V__NamedBegin,
    V__Net,
    V__Task,
    V__Register,
    V__Variable,
    V__Class,
    SV__Package,
    SV__Program,
    SV__Interface,
    SV__Array,
    SV__Directive,
    SV__Property,
    SV__Sequence,
}

#[derive(Copy, Clone)]
pub enum SimSignalDirection {
    Input,
    Output,
    Inout,
    None,
    Unknown,
}

#[derive(Copy, Clone)]
pub enum SimSignalType {
    Wire,
    Reg,
    Logic,
    Integer,
    Bit,
    Real,
    Event,
    String,
    Other,
}

#[allow(dead_code)]
pub struct SimObject {
    name: String,
    object_type: SimObjectType,
}

#[derive(Copy, Clone)]
pub enum SimDriverType {
    Object,
    Assign,
    Force,
    Other,
}

#[derive(Debug, Copy, Clone)]
pub enum SimRadix {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
    Unsigned,
    Unknown,
}

impl std::fmt::Debug for SimDriverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimDriverType::Object => write!(f, "Object"),
            SimDriverType::Assign => write!(f, "Assign"),
            SimDriverType::Force => write!(f, "Force"),
            SimDriverType::Other => write!(f, "Other"),
        }
    }
}

#[derive(Clone)]
pub struct SimDriver {
    pub driver_type: SimDriverType,
    pub source: String,
}

impl SimDriver {
    pub fn from_multiple_drivers_output(output: &str, path: &str) -> Vec<SimDriver> {
        let mut drivers: Vec<SimDriver> = Vec::new();

        // Since each entry is separated by ';'
        let lines: Vec<&str> = output
            .trim_start_matches(&format!("Drivers for {}:", path))
            .trim()
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for line in lines {
            if line.contains(&format!("{} is not a signal, net, or port", path)) {
                continue;
            }
            let driver = SimDriver::from_drivers_output(line.trim());
            drivers.push(driver);
        }
        drivers
    }
    pub fn from_drivers_output(output: &str) -> Self {
        let drivers_re =
            regex::Regex::new(r"\s*(\w+)\s*: (Driver|Net) (.+)").expect("Failed to compile regex");

        if let Some(captures) = drivers_re.captures(output) {
            let driver_type_str = captures.get(2).unwrap().as_str();
            let driver_type = match driver_type_str.trim() {
                "Driver" => SimDriverType::Object,
                "Net" => SimDriverType::Assign,
                _ => SimDriverType::Other,
            };
            let source = captures.get(3).unwrap().as_str().to_string();

            SimDriver {
                driver_type,
                source,
            }
        } else {
            SimDriver {
                driver_type: SimDriverType::Other,
                source: String::new(),
            }
        }
    }
}

#[derive(Clone)]
pub struct SimMemory {
    pub name: String,
    pub size: SimSignalBounds,
    pub width: usize,
}

impl SimMemory {
    pub fn from_mem_list_output(output: &str) -> Option<Self> {
        // Example line:
        // # Verilog: /Mem_tb/mem [0:63] x 8 w
        let mem_re = regex::Regex::new(r"# Verilog:\s+(\S+)\s+\[(\d+:\d+)\]\s+x\s+(\d+)\s+w")
            .expect("Failed to compile regex");

        if let Some(captures) = mem_re.captures(output) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let bounds_str = captures.get(2).unwrap().as_str();
            let width_str = captures.get(3).unwrap().as_str();

            if let Some(bounds) = SimSignalBounds::from_str(bounds_str) {
                if let Ok(width) = width_str.parse::<usize>() {
                    return Some(SimMemory {
                        name,
                        size: bounds,
                        width,
                    });
                }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct SimSignalBounds {
    pub left: i32,
    pub right: i32,
}

impl SimSignalBounds {
    pub fn from_str(bounds_str: &str) -> Option<Self> {
        let parts: Vec<&str> = bounds_str
            .trim_matches(&['[', ']'][..])
            .split(':')
            .collect();
        if parts.len() == 2 {
            if let (Ok(left), Ok(right)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                return Some(SimSignalBounds { left, right });
            }
        }
        None
    }

    pub fn width(&self) -> usize {
        (self.left - self.right + 1) as usize
    }
}

#[derive(Clone)]
pub struct SimSignal {
    pub name: String,
    pub signal_type: SimSignalType,
    pub direction: SimSignalDirection,
    pub bounds: SimSignalBounds,
    pub drivers: Vec<SimDriver>,
    pub value: Option<(SimRadix, String)>,
}

impl SimSignal {
    pub fn from_describe_output(
        path: &str,
        direction: SimSignalDirection,
        drivers: Vec<SimDriver>,
        output: &str,
    ) -> Self {
        let examine_re = regex::Regex::new(r"(Register|Wire|Net|bit|Logic) ?(\[\d+:\d+\])?")
            .expect("Failed to compile regex");

        let mut signal_type = SimSignalType::Other;
        let mut bounds: SimSignalBounds = SimSignalBounds { left: 0, right: 0 };

        if let Some(captures) = examine_re.captures(output) {
            let type_str = captures.get(1).unwrap().as_str();
            signal_type = match type_str {
                "Register" => SimSignalType::Reg,
                "Wire" => SimSignalType::Wire,
                "Net" => SimSignalType::Other,
                "bit" => SimSignalType::Bit,
                "Logic" => SimSignalType::Logic,
                _ => SimSignalType::Other,
            };
            if let Some(bounds_match) = captures.get(2) {
                if let Some(b) = SimSignalBounds::from_str(bounds_match.as_str()) {
                    bounds = b;
                }
            }
        }
        SimSignal {
            name: path.to_string(),
            signal_type,
            direction,
            bounds,
            drivers,
            value: None,
        }
    }

    pub fn set(&mut self, radix: SimRadix, value: &str) {
        self.value = Some((radix, value.to_string()));
    }

    fn clean_value_str(&self) -> Option<String> {
        if let Some((_, val_str)) = &self.value {
            // Remove any prefixes like "0'b", "0'd", etc.
            let clean_str = match val_str.splitn(2, '\'').nth(1) {
                Some(after_quote) => {
                    // drop a single base char if present (b/o/d/x, case-insensitive), then lowercase
                    let rest = after_quote
                        .trim_start_matches(|c: char| {
                            matches!(c, 'b' | 'B' | 'o' | 'O' | 'x' | 'X' | 'd' | 'D')
                        })
                        .to_lowercase();

                    // This gets tricky, because some values will contain 'x' or 'z' for unknown/high-impedance
                    // Return None in these cases
                    if rest.contains('x') || rest.contains('z') {
                        return None;
                    }

                    rest
                }
                None => val_str.to_lowercase(),
            };
            Some(clean_str)
        } else {
            None
        }
    }

    pub fn get_numeric_value<T>(&self) -> Option<T>
    where
        T: Num,
    {
        // Remove any prefixes like "0'b", "0'd", etc.
        if let Some(clean_str) = self.clean_value_str() {
            let (radix, _val_str) = self.value.as_ref().unwrap();
            // Determine radix based on original string
            match radix {
                SimRadix::Binary => return T::from_str_radix(&clean_str, 2).ok(),
                SimRadix::Octal => return T::from_str_radix(&clean_str, 8).ok(),
                SimRadix::Decimal => return T::from_str_radix(&clean_str, 10).ok(),
                SimRadix::Hexadecimal => return T::from_str_radix(&clean_str, 16).ok(),
                SimRadix::Unsigned => return T::from_str_radix(&clean_str, 10).ok(),
                SimRadix::Unknown => return None,
            }
        } else {
            None
        }
    }

    pub fn get_bytes_value(&self) -> Option<Vec<u8>> {
        if let Some(clean_str) = self.clean_value_str() {
            let (radix, _val_str) = self.value.as_ref().unwrap();
            // Get radix and parse the value string
            match radix {
                SimRadix::Binary => {
                    let num = u128::from_str_radix(&clean_str, 2).ok()?;
                    Some(num.to_le_bytes().to_vec())
                }
                SimRadix::Octal => {
                    let num = u128::from_str_radix(&clean_str, 8).ok()?;
                    Some(num.to_le_bytes().to_vec())
                }
                SimRadix::Decimal => {
                    let num = u128::from_str_radix(&clean_str, 10).ok()?;
                    Some(num.to_le_bytes().to_vec())
                }
                SimRadix::Hexadecimal => {
                    let num = u128::from_str_radix(&clean_str, 16).ok()?;
                    Some(num.to_le_bytes().to_vec())
                }
                SimRadix::Unsigned => {
                    let num = u128::from_str_radix(&clean_str, 10).ok()?;
                    Some(num.to_le_bytes().to_vec())
                }
                SimRadix::Unknown => None,
            }
        } else {
            None
        }
    }
}
