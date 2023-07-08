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
    pub const fn t(&self) -> BaseTypeKind {
        match self {
            Self::Int(_) => BaseTypeKind::Int,
            Self::Long(_) => BaseTypeKind::Long,
            Self::Float(_) => BaseTypeKind::Float,
            Self::Double(_) => BaseTypeKind::Double,
        }
    }
    /// We could use operator overloading for all the arithmetic operators
    /// but to keep things simple we chose to implement them as functions.

    /// Computes the sum of two values of the same type}
    pub fn add(lhs: &Value, rhs: &Value) -> Value {
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
            (Value::Long(lhs), Value::Long(rhs)) => Value::Long(lhs + rhs),
            (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs + rhs),
            (Value::Double(lhs), Value::Double(rhs)) => {
                Value::Double(lhs + rhs)
            }
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the difference of two values of the same type}
    pub fn sub(lhs: &Value, rhs: &Value) -> Value {
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
            (Value::Long(lhs), Value::Long(rhs)) => Value::Long(lhs - rhs),
            (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs - rhs),
            (Value::Double(lhs), Value::Double(rhs)) => {
                Value::Double(lhs - rhs)
            }
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the product of two values of the same type}
    pub fn mul(lhs: &Value, rhs: &Value) -> Value {
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
            (Value::Long(lhs), Value::Long(rhs)) => Value::Long(lhs * rhs),
            (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs * rhs),
            (Value::Double(lhs), Value::Double(rhs)) => {
                Value::Double(lhs * rhs)
            }
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the division of two values of the same type}
    pub fn div(lhs: &Value, rhs: &Value) -> Value {
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs / rhs),
            (Value::Long(lhs), Value::Long(rhs)) => Value::Long(lhs / rhs),
            (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs / rhs),
            (Value::Double(lhs), Value::Double(rhs)) => {
                Value::Double(lhs / rhs)
            }
            _ => panic!("Expected value type"),
        }
    }

    /// Compares two values of the same type, returns 1 if rhs is greater than lhs
    /// -1 if rhs is less than lhs and 0 otherwise.
    pub fn cmp(lhs: &Value, rhs: &Value) -> i32 {
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Self::compare(lhs, rhs),
            (Value::Long(lhs), Value::Long(rhs)) => Self::compare(lhs, rhs),
            (Value::Float(lhs), Value::Float(rhs)) => Self::compare(lhs, rhs),
            (Value::Double(lhs), Value::Double(rhs)) => Self::compare(lhs, rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Comparison function for primitive types that implement `PartialOrd`.
    fn compare<T: PartialOrd>(lhs: T, rhs: T) -> i32 {
        if lhs < rhs {
            -1
        } else if rhs > lhs {
            1
        } else {
            0
        }
    }
}

/// Instructions are composed of an opcode and list of optional
/// arguments or parameters.
#[derive(Debug, Clone)]
pub struct Instruction {
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

/// Frames are used to store data and partial results within a method's scope.
/// Each frame has an operand stack and array of local variables.
#[derive(Debug, Clone)]
struct Frame {
    pc: ProgramCounter,
    stack: Vec<Value>,
    locals: HashMap<usize, Value>,
}

impl Frame {
    /// Returns current method index pointed at by the program counter.
    const fn method_index(&self) -> usize {
        self.pc.method_index
    }

    /// Returns current instruction index pointed at by the program counter.
    const fn instruction_index(&self) -> usize {
        self.pc.instruction_index
    }
    /// Increment program counter instruction index.
    fn inc_instruction_index(&mut self) {
        self.pc.instruction_index += 1;
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
    frames: Vec<Frame>,
}

impl Runtime {
    // TODO: considering moving Program to JVM module instead
    // to avoid repetition here and keeps things tight.
    pub fn new(program: Program) -> Self {
        let main = program.entry_point();
        let pc = ProgramCounter {
            instruction_index: 0,
            method_index: main,
        };
        let initial_frame = Frame {
            pc: pc,
            stack: Vec::new(),
            locals: HashMap::new(),
        };
        Self {
            program: program,
            frames: vec![initial_frame],
        }
    }

    pub fn run(&mut self) -> Result<()> {
        while !self.frames.is_empty() {
            let inst = self.fetch();
            println!("Next instruction: {inst:?}");
            self.eval(&inst);
        }
        Ok(())
    }

    /// Push a JVM value into the stack
    fn push(&mut self, value: Value) {
        if let Some(frame) = self.frames.last_mut() {
            frame.stack.push(value);
        }
    }

    /// Pop a JVM value from the stack.
    fn pop(&mut self) -> Option<Value> {
        match self.frames.last_mut() {
            Some(frame) => frame.stack.pop(),
            None => None,
        }
    }

    /// Store the topmost value in the stack as local value.
    fn store(&mut self, index: usize) {
        if let Some(value) = self.pop() {
            match self.frames.last_mut() {
                Some(frame) => {
                    frame.locals.insert(index, value);
                }
                None => (),
            }
        }
    }

    /// Load a local value and push it to the stack.
    fn load(&mut self, index: usize) {
        if let Some(frame) = self.frames.last_mut() {
            match frame.locals.get(&index) {
                Some(value) => frame.stack.push(*value),
                None => (),
            }
        }
    }

    /// Evaluate a given instruction.
    fn eval(&mut self, inst: &Instruction) {
        if let Some(frame) = self.frames.last_mut() {
            match inst.mnemonic {
                OPCode::IconstM1 => {
                    println!("Executing IconstM1");
                    self.push(Value::Int(-1));
                }
                OPCode::Iconst0 => self.push(Value::Int(0)),
                OPCode::Iconst1 => self.push(Value::Int(1)),
                OPCode::Iconst2 => self.push(Value::Int(2)),
                OPCode::Iconst3 => self.push(Value::Int(3)),
                OPCode::Iconst4 => self.push(Value::Int(4)),
                OPCode::Iconst5 => self.push(Value::Int(5)),
                OPCode::Lconst0 => self.push(Value::Long(0)),
                OPCode::Lconst1 => self.push(Value::Long(1)),
                OPCode::Fconst0 => self.push(Value::Float(0.)),
                OPCode::Fconst1 => self.push(Value::Float(1.)),
                OPCode::Fconst2 => self.push(Value::Float(2.)),
                OPCode::Dconst0 => self.push(Value::Double(0.)),
                OPCode::Dconst1 => self.push(Value::Double(1.)),
                OPCode::BiPush
                | OPCode::SiPush
                | OPCode::Ldc
                | OPCode::Ldc2W => match &inst.params {
                    Some(params) => self.push(params[0]),
                    None => panic!(
                        "Expected instruction to have parameters got None"
                    ),
                },
                // Load operations.
                OPCode::ILoad
                | OPCode::LLoad
                | OPCode::FLoad
                | OPCode::DLoad => match &inst.params {
                    Some(params) => match params[0] {
                        Value::Int(v) => self.load(v as usize),
                        _ => panic!(
                            "Expected parameter to be of type Value::Int"
                        ),
                    },
                    None => panic!(
                        "Expected instruction to have parameters got None"
                    ),
                },
                OPCode::ILoad0
                | OPCode::LLoad0
                | OPCode::FLoad0
                | OPCode::DLoad0 => self.load(0),
                OPCode::ILoad1
                | OPCode::LLoad1
                | OPCode::FLoad1
                | OPCode::DLoad1 => self.load(1),
                OPCode::ILoad2
                | OPCode::LLoad2
                | OPCode::FLoad2
                | OPCode::DLoad2 => self.load(2),
                OPCode::ILoad3
                | OPCode::LLoad3
                | OPCode::FLoad3
                | OPCode::DLoad3 => self.load(3),
                // Store operations.
                OPCode::IStore
                | OPCode::LStore
                | OPCode::FStore
                | OPCode::DStore => match &inst.params {
                    Some(params) => match params[0] {
                        Value::Int(v) => self.store(v as usize),
                        _ => panic!(
                            "Expected parameter to be of type Value::Int"
                        ),
                    },
                    None => panic!(
                        "Expected instruction to have parameters got None"
                    ),
                },
                OPCode::IStore0
                | OPCode::LStore0
                | OPCode::FStore0
                | OPCode::DStore0 => self.store(0),
                OPCode::IStore1
                | OPCode::LStore1
                | OPCode::FStore1
                | OPCode::DStore1 => self.store(1),
                OPCode::IStore2
                | OPCode::LStore2
                | OPCode::FStore2
                | OPCode::DStore2 => self.store(2),
                OPCode::IStore3
                | OPCode::LStore3
                | OPCode::FStore3
                | OPCode::DStore3 => self.store(3),
                // Comparison operations.
                OPCode::LCmp
                | OPCode::FCmpL
                | OPCode::FCmpG
                | OPCode::DCmpL
                | OPCode::DCmpG => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    match (lhs, rhs) {
                        (Some(a), Some(b)) => {
                            self.push(Value::Int(Value::cmp(&a, &b)))
                        }
                        _ => (),
                    }
                }
                // Return with value.
                OPCode::IReturn
                | OPCode::LReturn
                | OPCode::FReturn
                | OPCode::DReturn => todo!(),
                // Void return
                OPCode::Return => {
                    self.frames.pop();
                }
                OPCode::NOP => (),
                _ => (),
            }
        }
        println!("Frames : {:?}", self.frames);
    }

    /// Returns the opcode parameter encoded as two `u8` values in the bytecode
    /// as an `i32`.
    const fn encode_arg(lo: u8, hi: u8) -> i32 {
        (lo as i32) << 8 | hi as i32
    }

    /// Returns the next bytecode value in the current method.
    fn next(&mut self, frame: &mut Frame) -> u8 {
        let method_index = frame.method_index();
        let code = self.program.code(method_index);
        let bc = code[frame.instruction_index()];
        frame.inc_instruction_index();
        bc
    }

    /// Returns the next instruction to execute.
    fn fetch(&mut self) -> Instruction {
        // Ugly hack, since we can't borrow frame as mutable more than once
        // we pop it out, do what we want then push it back.
        let current_frame = self.frames.pop();
        match current_frame {
            Some(mut frame) => {
                let mnemonic = OPCode::from(self.next(&mut frame));
                let params = match mnemonic {
                    OPCode::SiPush
                    | OPCode::IfEq
                    | OPCode::IfNe
                    | OPCode::IfLt
                    | OPCode::IfLe
                    | OPCode::IfGt
                    | OPCode::IfGe
                    | OPCode::IfICmpEq
                    | OPCode::IfICmpNe
                    | OPCode::IfICmpLt
                    | OPCode::IfICmpLe
                    | OPCode::IfICmpGt
                    | OPCode::IfICmpGe
                    | OPCode::Goto => {
                        let lo = self.next(&mut frame);
                        let hi = self.next(&mut frame);
                        let param = Self::encode_arg(lo, hi);
                        Some(vec![Value::Int(param)])
                    }
                    OPCode::InvokeSpecial
                    | OPCode::GetStatic
                    | OPCode::InvokeVirtual
                    | OPCode::IInc => {
                        let first = i32::from(self.next(&mut frame));
                        let second = i32::from(self.next(&mut frame));
                        Some(vec![Value::Int(first), Value::Int(second)])
                    }
                    OPCode::BiPush
                    | OPCode::ILoad
                    | OPCode::FLoad
                    | OPCode::LLoad
                    | OPCode::DLoad
                    | OPCode::IStore
                    | OPCode::FStore
                    | OPCode::LStore
                    | OPCode::DStore => {
                        let arg = i32::from(self.next(&mut frame));
                        Some(vec![Value::Int(arg)])
                    }
                    OPCode::InvokeStatic => {
                        let lo = self.next(&mut frame);
                        let hi = self.next(&mut frame);
                        let method_ref_index =
                            Self::encode_arg(lo, hi) as usize;
                        println!("Method Ref Index: {method_ref_index}");
                        let method_name_index =
                            self.program.find_method(method_ref_index);
                        Some(vec![Value::Int(method_name_index as i32)])
                    }
                    _ => None,
                };
                self.frames.push(frame);

                println!("Mnemonic : {mnemonic}");

                Instruction { mnemonic, params }
            }
            None => panic!("no next instruction"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::read_class_file;
    use crate::jvm::JVMParser;
    use crate::program::Program;
    use std::env;
    use std::path::Path;

    #[test]
    fn it_works() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/NakedMain.class");
        let class_file_bytes = read_class_file(&path);
        let result = JVMParser::parse(&class_file_bytes);
        assert!(result.is_ok());
        let class_file = result.unwrap();
        let program = Program::new(&class_file);
        let mut runtime = Runtime::new(program);
        println!("{:?}", runtime.run());
    }
}
