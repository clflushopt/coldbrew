//! Code profiler for the interpreter works by keeping track of loop
//! entries and exits. When a given loop entry has exceeded the threshold
//! it's considered hot and a trace will be compiled for it.
use std::collections::HashMap;

use crate::runtime::ProgramCounter;

#[derive(Debug)]
pub struct Profiler {
    // Threshold before a loop entry is considered hot.
    threshold: usize,
    // Last accessed program counter.
    last_pc: ProgramCounter,
    // Record of loop entries and their access counts.
    records: HashMap<ProgramCounter, usize>,
}

impl Profiler {
    pub fn new() -> Profiler {
        Profiler {
            threshold: 2,
            last_pc: ProgramCounter::new(),
            records: HashMap::new(),
        }
    }

    pub fn count_entry(&mut self, pc: &ProgramCounter) {
        if pc.get_method_index() == self.last_pc.get_method_index()
            && pc.get_instruction_index() < self.last_pc.get_instruction_index()
        {
            match self.records.get_mut(pc) {
                Some(record) => *record += 1,
                None => {
                    self.records.insert(*pc, 1);
                }
            }
        }
        self.last_pc = *pc;
    }
    pub fn count_exit(&mut self, pc: &ProgramCounter) {
        match self.records.get_mut(pc) {
            Some(record) => *record += 1,
            None => {
                self.records.insert(*pc, 1);
            }
        }
        self.last_pc = *pc
    }
    pub fn is_hot(&self, pc: &ProgramCounter) -> bool {
        if let Some(record) = self.records.get(pc) {
            return record > &self.threshold;
        }
        false
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}
