//! Runtime tracing module for coldbrew.
use std::collections::HashSet;
use std::fmt::Write;

use crate::bytecode::OPCode;
use crate::runtime::{Instruction, ProgramCounter, Value};

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

    /// Check if we are recording a trace already.
    pub const fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Check if we finished recording a trace.
    pub fn is_done_recording(&mut self, pc: ProgramCounter) -> bool {
        if self.trace.len() == 0 {
            return false;
        }
        match self.trace.get(self.trace.len() - 1) {
            Some(entry) => match entry.inst.get_mnemonic() {
                OPCode::Return
                | OPCode::IReturn
                | OPCode::LReturn
                | OPCode::FReturn
                | OPCode::DReturn => {
                    if pc.get_method_index() == entry.pc.get_method_index() {
                        println!("Found recursive return -- abort recording");
                        self.is_recording = false;
                        return false;
                    }
                    pc == self.loop_header
                }
                _ => pc == self.loop_header,
            },
            None => false,
        }
    }

    /// Core recording routine, given the current program counter
    /// and instruction we are executing decide if we should recording
    /// branching targets in the case of instructions that have an implicit
    /// jump such as equality instructions (IfEq, IfNe..).
    pub fn record(&mut self, pc: ProgramCounter, inst: Instruction) {
        // Branch flip if the last recorded instruction was a branch.
        if self.last_instruction_was_branch {
            // self.flip_branch(pc);
        }
        match inst.get_mnemonic() {
            OPCode::Goto => {
                let offset = match inst.get_params() {
                    Some(params) => match params.get(0) {
                        Some(Value::Int(v)) => *v,
                        _ => {
                            panic!("Expected Goto to have integer parameter")
                        }
                    },
                    None => {
                        panic!("Expected Goto to have at least one parameter")
                    }
                };
                if offset > 0 {
                    return;
                } else {
                    let mut branch_target = pc;
                    branch_target.inc_instruction_index(offset);
                    if self.trace_start == branch_target {
                        self.inner_branch_targets.insert(branch_target);
                    } else {
                        self.outer_branch_targets.insert(branch_target);
                    }
                }
            }
            OPCode::IfNe
            | OPCode::IfEq
            | OPCode::IfGt
            | OPCode::IfICmpGe
            | OPCode::IfICmpGt
            | OPCode::IfICmpLt
            | OPCode::IfICmpLe
            | OPCode::IfICmpNe
            | OPCode::IfICmpEq => self.last_instruction_was_branch = true,
            OPCode::InvokeStatic => {
                // Check for recursive function calls.
            }
            _ => (),
        }
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
        // Clear existing traces.
        self.trace.clear();
        self.inner_branch_targets.clear();
        self.outer_branch_targets.clear();
    }

    /// Return the last recorded trace.
    pub fn recording(&mut self) -> Recording {
        self.is_recording = false;
        Recording {
            start: self.trace_start,
            trace: self.trace.clone(),
            inner_branch_targets: self.inner_branch_targets.clone(),
            outer_branch_targets: self.outer_branch_targets.clone(),
        }
    }

    /// Prints the recorded trace to stdout.
    pub fn debug(&self) -> std::fmt::Result {
        let mut s = String::new();
        write!(
            &mut s,
            "---- Trace recorded : ({},{}) ----",
            self.trace_start.get_method_index(),
            self.trace_start.get_instruction_index()
        )?;
        for record in &self.trace {
            let inst = &record.inst;
            write!(&mut s, "{} ", inst.get_mnemonic());
            for param in &inst.get_params() {
                write!(&mut s, "{:?} ", param);
            }
            write!(&mut s, "\n");
        }
        writeln!(&mut s, "---- ------------------ ----");

        println!("{}", s);
        Ok(())
    }
}
