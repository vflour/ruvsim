use super::super::sim_types::{ParsedLine, SimMemory, SimError};
use super::command_ctl::CommandCtl;

pub struct MemCtl {
    memories: Vec<SimMemory>,
}

impl MemCtl {
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
        }
    }

    pub fn memories(&self) -> &Vec<SimMemory> {
        &self.memories
    }

    pub fn list(&mut self, ctl: &mut CommandCtl) -> Result<Vec<SimMemory>, SimError> {
        ctl.send_command("mem list -r")?;
        self.refresh(ctl.latest_buffer());
        Ok(self.memories.clone())
    }

    pub fn read(
        &mut self,
        ctl: &mut CommandCtl,
        mem: &mut SimMemory,
    ) -> Result<Vec<u8>, SimError> {
        // Mem display shows memory contents, seperated by spaces and newlines
        let command = format!("puts \"MEM_DATA:[string map {{\\n ;}} [mem display {}]]\\n\"", mem.name);
        let output = ctl.send_and_expect_result(&command, |line| {
            !line.is_empty() && line.starts_with("MEM_DATA:")
        })?;

        // Parse the line:
        let line = output.last().unwrap().content.trim_start_matches("MEM_DATA:");
        // First, split by newline separator (now ; character)
        let lines = line.split(';');
        let mut data = Vec::new();

        for l in lines {
            // Each line starts with a number, followed by a colon. Trim it and then split by spaces.
            // ie. 0: 00AABBCC DDEEFF00 -> [00AABBCC, DDEEFF00]
            let values: Vec<&str> = l.splitn(2, ':').collect::<Vec<&str>>()[1].trim().split_whitespace().collect();
            
            // Each character is a hex value containing 4 bits
            for val_str in values {
                for byte in val_str.as_bytes().chunks(2).rev() {
                    let mut byte_str = std::str::from_utf8(byte)
                        .map_err(|e| SimError::CommandError(format!("Invalid UTF-8 in memory data: {}", e).into()))?
                        .to_ascii_lowercase();
                    let byte_str = &byte_str
                        .replace("x", "0")
                        .replace("z", "0"); // Replace 'x' with '0' for unknowns
                    let byte_value = u8::from_str_radix(byte_str, 16).map_err(|e| SimError::CommandError(format!("Failed to parse byte from memory data: {}", e).into()))?;
                    
                    data.push(byte_value);

                }
                
            }
                
        }
        mem.data = Some(data.clone());
        Ok(data)

    }

    pub fn write(
        &mut self,
        ctl: &mut CommandCtl,
        mem: &SimMemory,
        offset: usize,
        value: Vec<u8>,
    ) -> Result<(), SimError> {
    
        // First, convert into binary string
        let mut bin_str = String::new();
        for byte in value {
            bin_str.push_str(&format!("{:08b}", byte));
        }
    
        // Then issue a force command for each bit
        let width = mem.width;
        for (bit_pos, bit_char) in bin_str.chars().rev().enumerate() {
            // Find the index of the memory word to write to
            let index = (offset + bit_pos) / (width as usize);
            // Then the bit position within that word
            let word_bit_pos = (offset + bit_pos) % (width as usize);

            // Write value
            let bit_value = match bit_char {
                '0' => 0,
                '1' => 1,
                _ => return Err(SimError::CommandError(format!("Invalid bit character: {}", bit_char).into())),
            };
            let command = if width == 1 {
                format!(
                    "force -freeze {}[{}] {}",
                    mem.name,
                    index,
                    bit_value
                )
            } else {
                format!(
                    "force -freeze {}[{}][{}] {}",
                    mem.name,
                    index,
                    word_bit_pos,
                    bit_value
                )
            };
            ctl.send_command(&command)?;
        }

        Ok(())
    
    
    }


    pub fn refresh(&mut self, buffer: &[ParsedLine]) {
        // Each memory line looks like:
        let mut mems = Vec::new();
        for line in buffer {
            if let Some(mem) = SimMemory::from_mem_list_output(&line.content) {
                mems.push(mem);

            }
        }
        self.memories = mems;
    }
}
