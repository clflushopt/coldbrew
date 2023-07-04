//! JVM runtime module responsible for creating a new runtime
//! environment and running programs.
use crate::bytecode::OPCode;
use crate::program::{BaseTypeKind, Program, Type};

use std::collections::HashMap;
use std::fmt;

type Result<T> = std::result::Result<T, RuntimeError>;

/// `RuntimeErrorKind` represents the possible errors that can occur
/// during runtime
#[derive(Debug, Copy, Clone)]
pub enum RuntimeErrorKind {}

/// `RuntimeError` is a custom type used to handle and represents
/// possible execution failures.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    kind: RuntimeErrorKind,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "runtime error occured")
    }
}

/// JVM value types.
#[derive(Debug, Copy, Clone)]
enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
}

impl Value {
    /// Returns the type of the value.
    pub fn t(&self) -> BaseTypeKind {
        match self {
            Self::Int(_) => BaseTypeKind::Int,
            Self::Long(_) => BaseTypeKind::Long,
            Self::Float(_) => BaseTypeKind::Float,
            Self::Double(_) => BaseTypeKind::Double,
        }
    }
}

/// Instructions are composed of an opcode and list of optional
/// arguments or parameters.
#[derive(Debug, Clone)]
struct Instruction {
    mnemonic: OPCode,
    params: Option<Vec<Value>>,
}

/// Program counter for the runtime points to the current instruction
/// and method we're executing.
#[derive(Debug, Clone)]
struct ProgramCounter {
    instruction_index: usize,
    method_index: usize,
}

/// Execution environment state for that encloses an execution scope.
/// We create a new scope each time we start executing a new method and
/// destroy it once we leave it.
///
/// The execution environment holds a program counter and a stack of values.
#[derive(Debug, Clone)]
struct State {
    pc: ProgramCounter,
    stack: Vec<Value>,
    locals: HashMap<usize, Value>,
}

impl State {
    /// Returns current method index pointed at by the program counter.
    fn method_index(&self) -> usize {
        self.pc.method_index
    }

    /// Returns current instruction index pointed at by the program counter.
    fn instruction_index(&self) -> usize {
        self.pc.instruction_index
    }
    /// Increment program counter instruction index.
    fn inc_instruction_index(&mut self) {
        self.pc.instruction_index += 1
    }
}

/// `Runtime` represents an execution context for JVM programs
/// and is responsible for interpreting the program's instructions
/// in a bytecode format, building execution traces and dispatching
/// execution to the `Jit` when a block is considered hot.
///
/// `Trace` structure :
/// +-------------------------
/// + `Profile`   | `Record` +
/// +------------------------+
///
/// `Profile` has all the profiling information for a trace, such
/// as how many times the trace was executed at this pc value and
/// if it's hot. `Record` contains a stream of assembly instruction
/// and an exit pc so we can redirect execution from the native CPU
/// back to the runtime.
///
/// `JitContext`is a minimal struct used to encode a record to execute
/// and is responsible for keeping track of the CPU <> Runtime context
/// switching.
pub struct Runtime {
    // Program to run.
    // program: Program,
    // Trace profiling statistics, indexed by the program counter
    // where each trace starts.
    // traces: Vec<Trace>,
    program: Program,
    states: Vec<State>,
}

impl Runtime {
    // TODO: considering moving Program to JVM module instead
    // to avoid repetition here and keeps things tight.
    pub fn new(program: Program) -> Self {
        let main = program.entry_point();
        let pc = ProgramCounter {
            instruction_index: 0,
            method_index: main as usize,
        };
        let initial_state = State {
            pc: pc,
            stack: Vec::new(),
            locals: HashMap::new(),
        };
        Self {
            program: program,
            states: vec![initial_state],
        }
    }

    pub fn run(&mut self) -> Result<()> {
        while !self.states.is_empty() {
            let next = self.next();
            println!("Next instruction: {:?}", next);
            self.eval(next);
        }
        Ok(())
    }

    /// Evaluate a given instruction.
    fn eval(&mut self, inst: Instruction) {
        match self.states.last_mut() {
            Some(state) => {
                match inst.mnemonic {
                    OPCode::IconstM1 => {
                        println!("Executing IconstM1")
                    },
                    OPCode::NOP => (),
                    OPCode::Return => {
                        self.states.pop();
                    },
                    _ => (),

                }


            },
            None => (),
        }
    }


    /// Returns the next instruction to execute.
    fn next(&mut self) -> Instruction {
        match self.states.last_mut() {
            Some(state) => {
                let method_index = state.method_index();
                let code = self.program.code(method_index);
                let opcode = code[state.instruction_index()];
                state.inc_instruction_index();
                Instruction {
                    mnemonic: OPCode::from(opcode),
                    params: None,
                }
            }
            None => panic!("no next instruction"),
        }
    }
}
