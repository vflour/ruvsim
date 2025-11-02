// Output lines

use std::str::FromStr;

// Regexes for each LineType:
fn get_line_type_from_regex(line: &str) -> LineType {
    let prompt_re = regex::Regex::new(r"^(VSIM|Questa|ModelSim)( \d+)?>$").unwrap();
    let exit_re = regex::Regex::new(r"^# End time: (.*), Elapsed time: (.*)$").unwrap();   
    let tcl_comment_re = regex::Regex::new(r"^#.*$").unwrap();

    if prompt_re.is_match(line) {
        LineType::Prompt
    } else if exit_re.is_match(line) {
        LineType::Exit
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

#[derive(Debug)]
pub enum LineType {
    Prompt,
    Log,
    Output,
    Exit,
    Unknown,
}

impl FromStr for LineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Log" => Ok(LineType::Log),
            "Prompt" => Ok(LineType::Prompt),
            "Output" => Ok(LineType::Output),
            "Unknown" => Ok(LineType::Unknown),
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
pub struct ParsedPrompt {
    prompt_type: PromptType,
    id: Option<u32>,
}

impl ParsedPrompt {
    fn new(line: &str) -> Self {
        let prompt_type = get_prompt_type_from_line(line);
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
            prompt_type: prompt_type,
            id: id,
        }
    }
}


#[derive(Debug)]
pub struct ParsedLine {
    pub line_type: LineType,
    pub content: String,
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

#[allow(non_camel_case_types)]
pub enum SimObjectType {
    V__Function,
    V__ModuleInstance,
    V_NamedFork,
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
}

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

pub struct SimSignal {
    pub name: String,
    pub signal_type: SimSignalType,
    pub direction: SimSignalDirection,
    pub left_bound: Option<i32>,  // in bits
    pub right_bound: Option<i32>, // idem
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
        let mut left_bound = None;
        let mut right_bound = None;

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

            if let Some(bounds) = captures.get(2) {
                let bounds_str = bounds.as_str().trim_matches(&['[', ']'][..]);
                let parts: Vec<&str> = bounds_str.split(':').collect();
                if parts.len() == 2 {
                    left_bound = parts[0].parse::<i32>().ok();
                    right_bound = parts[1].parse::<i32>().ok();
                }
            }
        }
        SimSignal {
            name: path.to_string(),
            signal_type,
            direction,
            left_bound,
            right_bound,
            drivers,
            value: None,
        }
    }

    pub fn set(&mut self, radix: SimRadix, value: &str) {
        self.value = Some((radix,value.to_string()));
    }   

    pub fn get_numeric_value(&self) -> Option<u64> {
        if let Some((radix,val_str)) = &self.value {
            // Remove any prefixes like "0b", "0x", etc.
            let clean_str = val_str.trim_start_matches("b")
                                   .trim_start_matches("o")
                                   .trim_start_matches("x")
                                   .to_lowercase();

            // This gets tricky, because some values will contain 'x' or 'z' for unknown/high-impedance
            // Return None in these cases
            if clean_str.contains('x') || clean_str.contains('z') {
                return None;
            }

            // Determine radix based on original string
            match radix {
                SimRadix::Binary => return u64::from_str_radix(&clean_str, 2).ok(),
                SimRadix::Octal => return u64::from_str_radix(&clean_str, 8).ok(),
                SimRadix::Decimal => return u64::from_str_radix(&clean_str, 10).ok(),
                SimRadix::Hexadecimal => return u64::from_str_radix(&clean_str, 16).ok(),
                SimRadix::Unsigned => return u64::from_str_radix(&clean_str, 10).ok(),
                SimRadix::Unknown => return None,
            }
        } else {
            None
        }
    }
}
