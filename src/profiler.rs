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

    // Count an entry to a loop header, since JVM bytecode is organized by
    // two indexes, the first `method_index` points to the method we are
    // currently executing and the second `instruction_index` actually points
    // to the bytecode offset within that method.
    //
    // For a `pc` entry to be considered a valid loop header it needs to
    // verify two conditions :
    //
    // - The loop header exists in the same method of the last accessed
    // method index.
    // - The instruction index within the method is before the last accessed
    // program counter's instruction index.
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

    // Count an exit from the JIT back to the interpreter, these "side-exits"
    // mark the non presence of a native trace which causes the exit back
    // to interpretation. Since we ideally want to spend as much time executing
    // native code we count these exists to trigger them for recording so we
    // can have a native trace next time we hit this `pc`.
    pub fn count_exit(&mut self, pc: &ProgramCounter) {
        match self.records.get_mut(pc) {
            Some(record) => *record += 1,
            None => {
                self.records.insert(*pc, 1);
            }
        }
        self.last_pc = *pc
    }

    // Returns whether a given `pc` is considered "hot" which just signals
    // to the recorder to start recording a trace.
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
