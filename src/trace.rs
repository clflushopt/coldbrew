//! Runtime tracing module for coldbrew.
use std::collections::HashSet;
use std::fmt::Write;

use crate::runtime::{Instruction, ProgramCounter};

/// Trace recording involves capturing an execution trace of the program in
/// various places. Each record entry in the trace is a tuple of (pc, inst)
/// where pc is the program counter (position of the entry in the bytecode)
/// and inst is the instruction executed there.
#[derive(Debug, Clone)]
struct RecordEntry {
    pc: ProgramCounter,
    inst: Instruction,
}
#[derive(Debug, Clone)]
struct Recording {
    start: ProgramCounter,
    trace: Vec<RecordEntry>,
    inner_branch_targets: HashSet<ProgramCounter>,
    outer_branch_targets: HashSet<ProgramCounter>,
}

struct TraceRecorder {
    trace_start: ProgramCounter,
    loop_header: ProgramCounter,
    is_recording: bool,
    last_instruction_was_branch: bool,
    trace: Vec<RecordEntry>,
    inner_branch_targets: HashSet<ProgramCounter>,
    outer_branch_targets: HashSet<ProgramCounter>,
}

impl TraceRecorder {
    pub fn new() -> Self {
        Self {
            trace_start: ProgramCounter::new(),
            loop_header: ProgramCounter::new(),
            is_recording: false,
            last_instruction_was_branch: false,
            trace: Vec::new(),
            inner_branch_targets: HashSet::new(),
            outer_branch_targets: HashSet::new(),
        }
    }

    // Check if we are recording a trace already.
    pub const fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Init a trace recording.
    pub fn init(&mut self, loop_header: ProgramCounter, start: ProgramCounter) {
        if self.is_recording && self.trace_start == start {
            return;
        }
        self.is_recording = true;
        self.last_instruction_was_branch = false;
        self.trace_start = start;
        self.loop_header = loop_header;
        self.trace.clear();
        self.inner_branch_targets.clear();
        self.outer_branch_targets.clear();
    }

    /// Return the last recorded trace.
    pub fn get_recording(&mut self) -> Recording {
        self.is_recording = false;
        Recording {
            start: self.trace_start,
            trace: self.trace.clone(),
            inner_branch_targets: self.inner_branch_targets.clone(),
            outer_branch_targets: self.outer_branch_targets.clone(),
        }
    }

    /// Prints the recorded trace to stdout.
    pub fn debug(&self) {
        let mut s = String::new();
        write!(
            &mut s,
            "---- Trace recorded : ({},{}) ----",
            self.trace_start.get_method_index(),
            self.trace_start.get_instruction_index()
        );
        for record in &self.trace {}
    }
}
