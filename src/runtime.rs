//! JVM runtime module responsible for creating a new runtime
//! environment and running programs.
use crate::bytecode::OPCode;
use crate::jvm::CPInfo;
use crate::profiler::Profiler;
use crate::program::{BaseTypeKind, Program};
use crate::trace;

use std::collections::HashMap;
use std::fmt;

/// `RuntimeErrorKind` represents the possible errors that can occur
/// during runtime
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    InvalidValue,
    InvalidOperandType(OPCode),
    MissingOperands(OPCode),
}

/// `RuntimeError` is a custom type used to handle and represents
/// possible execution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    kind: RuntimeErrorKind,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            RuntimeErrorKind::InvalidValue => {
                write!(f, "Expected value of type (int, float, long, double)")
            }
            RuntimeErrorKind::MissingOperands(opcode) => {
                write!(f, "Instruction {opcode} expects at least on operand, got none.")
            }
            RuntimeErrorKind::InvalidOperandType(opcode) => {
                write!(f, "Invalid operand type for instruction {opcode}")
            }
        }
    }
}

/// JVM value types.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
}

/// Trait used to represent a JVM value.
pub trait TypedValue {
    /// Returns the base type of the value.
    fn t() -> BaseTypeKind;
}

/// Implementation of JVM value helper functions to get the type and operate
/// on them.
/// We could use operator overloading for all the arithmetic operators
/// but to keep things simple we chose to implement them as functions.
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

    /// Given a value returns its basetype.
    pub const fn kind(v: &Value) -> BaseTypeKind {
        v.t()
    }

    /// Converts an existing value from it's base type to `BaseTypeKind::Long`.
    pub fn to_long(&self) -> Value {
        match *self {
            Self::Int(val) => Value::Long(val as i64),
            Self::Long(val) => Value::Long(val),
            Self::Float(val) => Value::Long(val as i64),
            Self::Double(val) => Value::Long(val as i64),
        }
    }
    /// Converts an existing value from it's base type to `BaseTypeKind::Int`.
    pub fn to_int(&self) -> Value {
        match *self {
            Self::Int(val) => Value::Int(val),
            Self::Long(val) => Value::Int(val as i32),
            Self::Float(val) => Value::Int(val as i32),
            Self::Double(val) => Value::Int(val as i32),
        }
    }
    /// Converts an existing value from it's base type to `BaseTypeKind::Double`.
    pub fn to_double(&self) -> Value {
        match *self {
            Self::Int(val) => Value::Double(val as f64),
            Self::Long(val) => Value::Double(val as f64),
            Self::Float(val) => Value::Double(val as f64),
            Self::Double(val) => Value::Double(val),
        }
    }
    /// Converts an existing value from it's base type to `BaseTypeKind::Float`.
    pub fn to_float(&self) -> Value {
        match *self {
            Self::Int(val) => Value::Float(val as f32),
            Self::Long(val) => Value::Float(val as f32),
            Self::Float(val) => Value::Float(val),
            Self::Double(val) => Value::Float(val as f32),
        }
    }

    /// Computes the sum of two values of the same type.
    pub fn add(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => {
                Self::Int(lhs.wrapping_add(*rhs))
            }
            (Self::Long(lhs), Self::Long(rhs)) => Self::Long(lhs + rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::Float(lhs + rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::Double(lhs + rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the difference of two values of the same type.
    pub fn sub(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Self::Int(lhs - rhs),
            (Self::Long(lhs), Self::Long(rhs)) => Self::Long(lhs - rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::Float(lhs - rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::Double(lhs - rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the product of two values of the same type.
    pub fn mul(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Self::Int(lhs * rhs),
            (Self::Long(lhs), Self::Long(rhs)) => Self::Long(lhs * rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::Float(lhs * rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::Double(lhs * rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the division of two values of the same type.
    pub fn div(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Self::Int(lhs / rhs),
            (Self::Long(lhs), Self::Long(rhs)) => Self::Long(lhs / rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::Float(lhs / rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::Double(lhs / rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Computes the remainder of the division of two values of the same type.
    pub fn rem(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Self::Int(lhs % rhs),
            (Self::Long(lhs), Self::Long(rhs)) => Self::Long(lhs % rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::Float(lhs % rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::Double(lhs % rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Compares two values of the same type, returns 1 if rhs is greater than lhs
    /// -1 if rhs is less than lhs and 0 otherwise.
    pub fn compare(lhs: &Self, rhs: &Self) -> i32 {
        match (lhs, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Self::cmp(lhs, rhs),
            (Self::Long(lhs), Self::Long(rhs)) => Self::cmp(lhs, rhs),
            (Self::Float(lhs), Self::Float(rhs)) => Self::cmp(lhs, rhs),
            (Self::Double(lhs), Self::Double(rhs)) => Self::cmp(lhs, rhs),
            _ => panic!("Expected value type"),
        }
    }

    /// Comparison function for primitive types that implement `PartialOrd`.
    fn cmp<T: PartialOrd>(lhs: &T, rhs: &T) -> i32 {
        if lhs < rhs {
            -1
        } else {
            i32::from(lhs > rhs)
        }
    }
}

/// Instructions are composed of an opcode and list of optional
/// arguments or parameters.
#[derive(Debug, Clone)]
pub struct Instruction {
    mnemonic: OPCode,
    operands: Option<Vec<Value>>,
}

impl Instruction {
    // Creates a new instruction.
    pub fn new(mnemonic: OPCode, params: Option<Vec<Value>>) -> Self {
        Self {
            mnemonic,
            operands: params,
        }
    }
    // Returns instruction mnemonic.
    pub fn get_mnemonic(&self) -> OPCode {
        self.mnemonic
    }

    /// Returns the nth parameter of an instruction.
    pub fn nth(&self, index: usize) -> Option<Value> {
        match &self.operands {
            Some(params) => params.get(index).map(|v| return *v),
            None => None,
        }
    }

    // Set instruction mnemonic.
    pub fn set_mnemonic(&mut self, mnemonic: OPCode) {
        self.mnemonic = mnemonic
    }

    // Returns a copy of instruction parameters.
    pub fn get_params(&self) -> Option<Vec<Value>> {
        return self.operands.clone();
    }
}

/// Program counter for the runtime points to the current instruction
/// and method we're executing.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProgramCounter {
    instruction_index: usize,
    method_index: usize,
}

impl fmt::Display for ProgramCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.instruction_index, self.method_index)
    }
}

impl ProgramCounter {
    pub fn new() -> Self {
        Self {
            instruction_index: 0,
            method_index: 0,
        }
    }

    pub fn get_instruction_index(&self) -> usize {
        self.instruction_index
    }

    pub fn get_method_index(&self) -> usize {
        self.method_index
    }

    pub fn inc_instruction_index(&mut self, offset: i32) {
        self.instruction_index =
            ((self.instruction_index as i32) + offset) as usize
    }
}

impl Default for ProgramCounter {
    fn default() -> Self {
        Self::new()
    }
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
    program: Program,
    // Stack frames.
    frames: Vec<Frame>,
    // Trace profiling statistics, indexed by the program counter
    // where each trace starts.
    pub recorder: trace::Recorder,
    profiler: Profiler,
    // traces: Vec<Trace>,
    // used to store return values
    return_values: Vec<Value>,
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
            pc,
            stack: Vec::new(),
            locals: HashMap::new(),
        };
        Self {
            program,
            frames: vec![initial_frame],
            recorder: trace::Recorder::new(),
            profiler: Profiler::new(),
            return_values: vec![],
        }
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        while !self.frames.is_empty() {
            let inst = self.fetch();
            let pc = self.frames.last().unwrap().pc;
            self.profiler.count_entry(&pc);

            if self.profiler.is_hot(&pc) {
                self.recorder.init(pc, pc);
            }
            if self.recorder.is_recording() {
                self.recorder.record(pc, inst.clone());
            }
            if self.jit_cache.has_native_trace(pc) {
                // If we have a native trace at this pc run it
                // and capture the return value which is the next
                // pc to execute.
                pc = self.jit_cache.execute(pc);
            }
            self.eval(&inst)?
        }

        // let _ = self.recorder.debug();
        Ok(())
    }

    /// Returns the top value in the return values stack.
    /// Used for testing only
    pub fn top_return_value(&self) -> Option<Value> {
        return self.return_values.last().copied();
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
            if let Some(frame) = self.frames.last_mut() {
                frame.locals.insert(index, value);
            }
        }
    }

    /// Load a local value and push it to the stack.
    fn load(&mut self, index: usize) {
        if let Some(frame) = self.frames.last_mut() {
            if let Some(value) = frame.locals.get(&index) {
                frame.stack.push(*value);
            }
        }
    }

    /// Jump with a relative offset.
    fn jump(&mut self, offset: i32) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc.instruction_index = (frame.pc.instruction_index as isize
                + offset as isize)
                as usize;
        }
    }

    /// Evaluate a given instruction.
    fn eval(&mut self, inst: &Instruction) -> Result<(), RuntimeError> {
        if let Some(_frame) = self.frames.last_mut() {
            match inst.mnemonic {
                OPCode::IconstM1 => {
                    self.push(Value::Int(-1));
                    return Ok(());
                }
                OPCode::Iconst0 => {
                    self.push(Value::Int(0));
                    return Ok(());
                }
                OPCode::Iconst1 => {
                    self.push(Value::Int(1));
                    return Ok(());
                }
                OPCode::Iconst2 => {
                    self.push(Value::Int(2));
                    return Ok(());
                }
                OPCode::Iconst3 => {
                    self.push(Value::Int(3));
                    return Ok(());
                }
                OPCode::Iconst4 => {
                    self.push(Value::Int(4));
                    return Ok(());
                }
                OPCode::Iconst5 => {
                    self.push(Value::Int(5));
                    return Ok(());
                }
                OPCode::Lconst0 => {
                    self.push(Value::Long(0));
                    return Ok(());
                }
                OPCode::Lconst1 => {
                    self.push(Value::Long(1));
                    return Ok(());
                }
                OPCode::Fconst0 => {
                    self.push(Value::Float(0.));
                    return Ok(());
                }
                OPCode::Fconst1 => {
                    self.push(Value::Float(1.));
                    return Ok(());
                }
                OPCode::Fconst2 => {
                    self.push(Value::Float(2.));
                    return Ok(());
                }
                OPCode::Dconst0 => {
                    self.push(Value::Double(0.));
                    return Ok(());
                }
                OPCode::Dconst1 => {
                    self.push(Value::Double(1.));
                    return Ok(());
                }
                OPCode::BiPush
                | OPCode::SiPush
                | OPCode::Ldc
                | OPCode::Ldc2W => match &inst.operands {
                    Some(params) => {
                        self.push(params[0]);
                        return Ok(());
                    }
                    None => Err(RuntimeError {
                        kind: RuntimeErrorKind::MissingOperands(inst.mnemonic),
                    }),
                },
                // Load operations.
                OPCode::ILoad
                | OPCode::LLoad
                | OPCode::FLoad
                | OPCode::DLoad => inst.operands.as_ref().map_or_else(
                    || {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::MissingOperands(
                                inst.mnemonic,
                            ),
                        })
                    },
                    |params| match params.get(0) {
                        Some(Value::Int(v)) => {
                            self.load(*v as usize);
                            return Ok(());
                        }
                        _ => Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidOperandType(
                                inst.mnemonic,
                            ),
                        }),
                    },
                ),
                OPCode::ILoad0
                | OPCode::LLoad0
                | OPCode::FLoad0
                | OPCode::DLoad0 => {
                    self.load(0);
                    return Ok(());
                }
                OPCode::ILoad1
                | OPCode::LLoad1
                | OPCode::FLoad1
                | OPCode::DLoad1 => {
                    self.load(1);
                    return Ok(());
                }
                OPCode::ILoad2
                | OPCode::LLoad2
                | OPCode::FLoad2
                | OPCode::DLoad2 => {
                    self.load(2);
                    return Ok(());
                }
                OPCode::ILoad3
                | OPCode::LLoad3
                | OPCode::FLoad3
                | OPCode::DLoad3 => {
                    self.load(3);
                    return Ok(());
                }
                // Store operations.
                OPCode::IStore
                | OPCode::LStore
                | OPCode::FStore
                | OPCode::DStore => inst.operands.as_ref().map_or_else(
                    || {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::MissingOperands(
                                inst.mnemonic,
                            ),
                        })
                    },
                    |params| match params.get(0) {
                        Some(Value::Int(v)) => {
                            self.store(*v as usize);
                            return Ok(());
                        }
                        _ => Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidOperandType(
                                inst.mnemonic,
                            ),
                        }),
                    },
                ),
                OPCode::IStore0
                | OPCode::LStore0
                | OPCode::FStore0
                | OPCode::DStore0 => {
                    self.store(0);
                    return Ok(());
                }
                OPCode::IStore1
                | OPCode::LStore1
                | OPCode::FStore1
                | OPCode::DStore1 => {
                    self.store(1);
                    return Ok(());
                }
                OPCode::IStore2
                | OPCode::LStore2
                | OPCode::FStore2
                | OPCode::DStore2 => {
                    self.store(2);
                    return Ok(());
                }
                OPCode::IStore3
                | OPCode::LStore3
                | OPCode::FStore3
                | OPCode::DStore3 => {
                    self.store(3);
                    return Ok(());
                }
                // Arithmetic operations.
                OPCode::IAdd | OPCode::LAdd | OPCode::FAdd | OPCode::DAdd => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::add(&a, &b));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::ISub | OPCode::LSub | OPCode::FSub | OPCode::DSub => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::sub(&a, &b));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IMul | OPCode::LMul | OPCode::FMul | OPCode::DMul => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::mul(&a, &b));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IDiv | OPCode::LDiv | OPCode::FDiv | OPCode::DDiv => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::div(&a, &b));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IRem | OPCode::LRem | OPCode::FRem | OPCode::DRem => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::rem(&a, &b));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IInc => {
                    if let Some(params) = &inst.operands {
                        if params.len() < 2 {
                            Err(RuntimeError {
                                kind: RuntimeErrorKind::MissingOperands(
                                    inst.mnemonic,
                                ),
                            })
                        } else {
                            match (params[0], params[1]) {
                                (Value::Int(index), Value::Int(constant)) => {
                                    self.frames
                                        .last_mut()
                                        .unwrap()
                                        .locals
                                        .entry(index as usize)
                                        .and_modify(|val| {
                                            *val = Value::add(
                                                val,
                                                &Value::Int(constant),
                                            )
                                        })
                                        .or_insert(Value::Int(constant));
                                    Ok(())
                                }
                                _ => Err(RuntimeError {
                                    kind: RuntimeErrorKind::InvalidOperandType(
                                        inst.mnemonic,
                                    ),
                                }),
                            }
                        }
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::MissingOperands(
                                inst.mnemonic,
                            ),
                        })
                    }
                }
                // Type conversion operations.
                OPCode::L2I | OPCode::F2I | OPCode::D2I => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_int());
                    return Ok(());
                }
                OPCode::I2F | OPCode::L2F | OPCode::D2F => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_float());
                    return Ok(());
                }
                OPCode::I2D | OPCode::L2D | OPCode::F2D => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_double());
                    return Ok(());
                }
                OPCode::I2L | OPCode::F2L | OPCode::D2L => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_long());
                    return Ok(());
                }
                // Comparison operations.
                OPCode::LCmp
                | OPCode::FCmpL
                | OPCode::FCmpG
                | OPCode::DCmpL
                | OPCode::DCmpG => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::Int(Value::compare(&a, &b)));
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                // Control flow operations.
                OPCode::IfEq => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer")
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );
                    if value == 0 {
                        self.jump(relative_offset);
                    }
                    Ok(())
                }
                OPCode::IfNe => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer")
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );
                    if value != 0 {
                        self.jump(relative_offset)
                    }
                    Ok(())
                }
                OPCode::IfLt => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer")
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value < 0 {
                        self.jump(relative_offset)
                    }
                    Ok(())
                }
                OPCode::IfGt => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer")
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value > 0 {
                        self.jump(relative_offset)
                    }
                    Ok(())
                }
                OPCode::IfLe => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer");
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value <= 0 {
                        self.jump(relative_offset)
                    }
                    Ok(())
                }
                OPCode::IfGe => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer");
                    };

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value >= 0 {
                        self.jump(relative_offset)
                    }
                    Ok(())
                }
                OPCode::IfICmpEq => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a == b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IfICmpNe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a != b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IfICmpLt => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a < b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IfICmpGt => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a > b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IfICmpLe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a <= b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                OPCode::IfICmpGe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a >= b {
                            self.jump(relative_offset)
                        }
                        Ok(())
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                // Goto
                OPCode::Goto => {
                    let relative_offset = inst.operands.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    self.jump(relative_offset);
                    return Ok(());
                }
                // Return with value.
                OPCode::IReturn
                | OPCode::LReturn
                | OPCode::FReturn
                | OPCode::DReturn => {
                    if let Some(mut frame) = self.frames.pop() {
                        let value = frame.stack.pop().unwrap();
                        // This is for debugging purposes.
                        self.return_values.push(value);
                        self.push(value);
                        return Ok(());
                    } else {
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::InvalidValue,
                        })
                    }
                }
                // Void return
                OPCode::Return => {
                    self.frames.pop();
                    Ok(())
                }
                // Function calls.
                OPCode::InvokeStatic => {
                    let name_index = match &inst.operands {
                        Some(params) => match params.get(0) {
                            Some(Value::Int(index)) => index,
                            _ => panic!(
                                "InvokeStatic expected integer parameter"
                            ),
                        },
                        _ => panic!("InvokeStatic expected parameters"),
                    };
                    self.invoke(*name_index as usize);
                    return Ok(());
                }
                // Currently only supports System.out.println.
                OPCode::InvokeVirtual => {
                    let value = self.pop();
                    println!("System.out.println : {value:?}");
                    Ok(())
                }
                OPCode::GetStatic | OPCode::NOP | OPCode::Dup => Ok(()),
                _ => todo!(),
            }
        } else {
            Ok(println!("Reached last frame...leaving"))
        }
    }

    /// Returns the opcode parameter encoded as two `u8` values in the bytecode
    /// as an `i32`.
    const fn encode_arg(lo: u8, hi: u8) -> i32 {
        let arg = (lo as i16) << 8 | hi as i16;
        arg as i32
    }

    /// Returns the next bytecode value in the current method.
    fn next(&mut self, frame: &mut Frame) -> u8 {
        let method_index = frame.method_index();
        let code = self.program.code(method_index);
        let bc = code[frame.instruction_index()];
        frame.inc_instruction_index();
        bc
    }

    /// Returns the relative offset from the mnemonics parameters list.
    fn get_relative_offset(params: &[Value]) -> i32 {
        match params.get(0) {
            Some(Value::Int(v)) => v - 3,
            _ => panic!("Expected parameter to be of type Value::Int"),
        }
    }

    /// Invoke a function by creating a new stack frame, building the locals
    /// and pushing the new frame into the runtime stack.
    fn invoke(&mut self, method_name_index: usize) {
        let method = &self.program.methods[&method_name_index];
        let stack = vec![];
        let mut locals = HashMap::new();
        let arg_types = method.arg_types.clone();
        let mut key = arg_types.iter().map(|arg_type| arg_type.size()).sum();

        for arg_type in arg_types.iter().rev() {
            key -= arg_type.size();
            let val = self.pop().unwrap();
            locals.insert(key, val);
        }
        assert_eq!(key, 0);
        let pc = ProgramCounter {
            instruction_index: 0,
            method_index: method_name_index,
        };
        let frame = Frame { pc, stack, locals };
        self.frames.push(frame);
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
                        let method_name_index =
                            self.program.find_method(method_ref_index);
                        Some(vec![Value::Int(method_name_index)])
                    }
                    OPCode::Ldc2W => {
                        let lo = self.next(&mut frame);
                        let hi = self.next(&mut frame);
                        let index = Self::encode_arg(lo, hi);
                        let entry = &self.program.constant_pool[index as usize];

                        match entry {
                            CPInfo::ConstantDouble { hi_bytes, lo_bytes } => {
                                let result = ((*hi_bytes as i64) << 32)
                                    + (*lo_bytes as i64);
                                Some(vec![Value::Double(result as f64)])
                            }
                            CPInfo::ConstantLong { hi_bytes, lo_bytes } => {
                                let result = ((*hi_bytes as i64) << 32)
                                    + (*lo_bytes as i64);
                                Some(vec![Value::Long(result)])
                            }
                            _ => panic!("unexpected entry in constant pool"),
                        }
                    }
                    OPCode::Ldc => {
                        let index = self.next(&mut frame);
                        let entry = &self.program.constant_pool[index as usize];

                        match entry {
                            CPInfo::ConstantFloat { bytes } => {
                                Some(vec![Value::Float(*bytes as f32)])
                            }
                            CPInfo::ConstantInteger { bytes } => {
                                Some(vec![Value::Int(*bytes as i32)])
                            }
                            _ => panic!("unexpected entry in constant pool"),
                        }
                    }
                    _ => None,
                };
                self.frames.push(frame);

                Instruction {
                    mnemonic,
                    operands: params,
                }
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

    // Macro to generate unit tests for the runtime.
    macro_rules! test_runtime_case {
        ($name: ident, $test_files:expr, $expected:expr) => {
            #[test]
            fn $name() {
                for test_file in $test_files {
                    let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
                    let path = Path::new(&env_var).join(test_file);
                    let class_file_bytes = read_class_file(&path)
                        .unwrap_or_else(|_| {
                            panic!(
                                "Failed to parse file : {:?}",
                                path.as_os_str()
                            )
                        });
                    let class_file = JVMParser::parse(&class_file_bytes);
                    assert!(class_file.is_ok());
                    let program = Program::new(&class_file.unwrap());
                    let mut runtime = Runtime::new(program);
                    assert!(runtime.run().is_ok());
                    assert_eq!(runtime.top_return_value(), $expected);
                }
            }
        };
    }

    test_runtime_case!(
        comparison,
        [
            "support/tests/CompareEq.class",
            "support/tests/CompareNe.class",
            "support/tests/CompareGt.class",
            "support/tests/CompareLt.class",
            "support/tests/CompareGe.class",
            "support/tests/CompareLe.class"
        ],
        Some(Value::Int(1))
    );

    test_runtime_case!(
        remainder,
        ["support/tests/Rem.class"],
        Some(Value::Int(2))
    );

    test_runtime_case!(
        function_calls,
        ["support/tests/FuncCall.class"],
        Some(Value::Int(500))
    );

    test_runtime_case!(
        loops,
        ["support/tests/Loop.class"],
        Some(Value::Int(1000))
    );

    test_runtime_case!(
        loop_with_function_call,
        ["support/tests/MultiFuncCall.class"],
        Some(Value::Int(5))
    );
}
