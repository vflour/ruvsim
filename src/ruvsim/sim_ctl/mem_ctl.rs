use std::error::Error;

use super::super::sim_types::{ParsedLine, SimMemory};
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
    pub fn list(&mut self, ctl: &mut CommandCtl) -> Result<Vec<SimMemory>, Box<dyn Error>> {
        ctl.send_command("mem list")?;
        self.refresh(ctl.latest_buffer());
        Ok(self.memories.clone())
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
