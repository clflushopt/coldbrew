//! JVM runtime module responsible for creating a new runtime
//! environment and running programs.
use crate::bytecode::OPCode;
use crate::program::{BaseTypeKind, Program};

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
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
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
            (Self::Int(lhs), Self::Int(rhs)) => Self::Int(lhs + rhs),
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
    params: Option<Vec<Value>>,
}

/// Program counter for the runtime points to the current instruction
/// and method we're executing.
#[derive(Debug, Clone, Copy)]
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
            return_values: vec![],
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

    /// Returns the top value in the return values stack.
    /// Used for testing only
    pub fn top_return_value(&self) -> Option<Value> {
        self.return_values.last().copied()
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
    fn jump(&mut self, offset: usize) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc.instruction_index += offset;
        }
    }

    /// Evaluate a given instruction.
    fn eval(&mut self, inst: &Instruction) {
        if let Some(_frame) = self.frames.last_mut() {
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
                | OPCode::DLoad => inst.params.as_ref().map_or_else(
                    || {
                        panic!(
                            "Expected instruction to have parameters got None"
                        )
                    },
                    |params| match params.get(0) {
                        Some(Value::Int(v)) => self.load(*v as usize),
                        _ => panic!(
                            "Expected parameter to be of type Value::Int"
                        ),
                    },
                ),
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
                | OPCode::DStore => inst.params.as_ref().map_or_else(
                    || {
                        panic!(
                            "Expected instruction to have parameters got None"
                        )
                    },
                    |params| match params.get(0) {
                        Some(Value::Int(v)) => self.store(*v as usize),
                        _ => panic!(
                            "Expected parameter to be of type Value::Int"
                        ),
                    },
                ),
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
                // Arithmetic operations.
                OPCode::IAdd | OPCode::LAdd | OPCode::FAdd | OPCode::DAdd => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::add(&a, &b))
                    }
                }
                OPCode::ISub | OPCode::LSub | OPCode::FSub | OPCode::DSub => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::sub(&a, &b))
                    }
                }
                OPCode::IMul | OPCode::LMul | OPCode::FMul | OPCode::DMul => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::mul(&a, &b))
                    }
                }
                OPCode::IDiv | OPCode::LDiv | OPCode::FDiv | OPCode::DDiv => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::div(&a, &b))
                    }
                }
                OPCode::IRem | OPCode::LRem | OPCode::FRem | OPCode::DRem => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        self.push(Value::rem(&a, &b))
                    }
                }
                OPCode::IInc => {
                    let (index, constant) = if let Some(params) = &inst.params {
                        match (params[0], params[1]) {
                            (Value::Int(ii), Value::Int(cnst)) => {
                                (ii as usize, cnst)
                            }
                            _ => panic!("Expected at least one parameter"),
                        }
                    } else {
                        panic!("Instruction IInc is missing parameters")
                    };
                    self.frames[0]
                        .locals
                        .entry(index)
                        .and_modify(|val| {
                            *val = Value::add(val, &Value::Int(constant))
                        })
                        .or_insert(Value::Int(constant));
                }
                // Type conversion operations.
                OPCode::L2I | OPCode::F2I | OPCode::D2I => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_int())
                }
                OPCode::I2F | OPCode::L2F | OPCode::D2F => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_float())
                }
                OPCode::I2D | OPCode::L2D | OPCode::F2D => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_double())
                }
                OPCode::I2L | OPCode::F2L | OPCode::D2L => {
                    let val = self.pop();
                    self.push(val.expect("expected value").to_long())
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
                    }
                }
                // Control flow operations.
                OPCode::IfEq => {
                    let Some(Value::Int(value)) = self.pop() else { panic!("expected value to be integer") };

                    let relative_offset = inst.params.as_ref().map_or_else(
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
                }
                OPCode::IfNe => {
                    let Some(Value::Int(value)) = self.pop() else { panic!("expected value to be integer") };

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );
                    if value != 0 {
                        self.jump(relative_offset);
                    }
                }
                OPCode::IfLt => {
                    let Some(Value::Int(value)) = self.pop() else { panic!("expected value to be integer") };

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value < 0 {
                        self.jump(relative_offset);
                    }
                }
                OPCode::IfGt => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer");
                    };

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value > 0 {
                        self.jump(relative_offset);
                    }
                }
                OPCode::IfLe => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer");
                    };

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value <= 0 {
                        self.jump(relative_offset);
                    }
                }
                OPCode::IfGe => {
                    let Some(Value::Int(value)) = self.pop() else {
                        panic!("expected value to be integer");
                    };

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if value >= 0 {
                        self.jump(relative_offset);
                    }
                }
                OPCode::IfICmpEq => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a == b {
                            self.jump(relative_offset);
                        }
                    }
                }
                OPCode::IfICmpNe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a != b {
                            self.jump(relative_offset);
                        }
                    }
                }
                OPCode::IfICmpLt => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a < b {
                            self.jump(relative_offset);
                        }
                    }
                }
                OPCode::IfICmpGt => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a > b {
                            self.jump(relative_offset);
                        }
                    }
                }
                OPCode::IfICmpLe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a <= b {
                            self.jump(relative_offset);
                        }
                    }
                }
                OPCode::IfICmpGe => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    if let (Some(a), Some(b)) = (lhs, rhs) {
                        if a >= b {
                            self.jump(relative_offset);
                        }
                    }
                }
                // Goto
                OPCode::Goto => {
                    let relative_offset = inst.params.as_ref().map_or_else(
                        || {
                            panic!(
                             "Expected instruction to have parameters got None"
                         )
                        },
                        |params| Self::get_relative_offset(params),
                    );

                    self.jump(relative_offset);
                }
                // Return with value.
                OPCode::IReturn
                | OPCode::LReturn
                | OPCode::FReturn
                | OPCode::DReturn => {
                    if let Some(mut frame) = self.frames.pop() {
                        let value = frame.stack.pop().unwrap();
                        self.return_values.push(value);
                        self.push(value);
                    }
                }
                // Void return
                OPCode::Return => {
                    self.frames.pop();
                }
                // TODO: Add InvokeVirtual/InvokeStatic
                OPCode::NOP | OPCode::Dup => (),
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

    /// Returns the relative offset from the mnemonics parameters list.
    fn get_relative_offset(params: &[Value]) -> usize {
        match params.get(0) {
            Some(Value::Int(v)) => (v - 3) as usize,
            _ => panic!("Expected parameter to be of type Value::Int"),
        }
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
                    // TODO: add LDC2_W and LDC instructions
                    _ => None,
                };
                println!("Frame : {frame:?}");
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
    fn compare_operations_works() {
        let test_files = vec![
            "support/CompareEq.class",
            "support/CompareNe.class",
            "support/CompareGt.class",
            "support/CompareLt.class",
            "support/CompareGe.class",
            "support/CompareLe.class",
        ];
        for test_file in test_files {
            println!("Testing : {test_file}");
            let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
            let path = Path::new(&env_var).join(test_file);
            let class_file_bytes = read_class_file(&path);
            let result = JVMParser::parse(&class_file_bytes);
            assert!(result.is_ok());
            let class_file = result.unwrap();
            let program = Program::new(&class_file);
            let mut runtime = Runtime::new(program);
            runtime.run();
            assert_eq!(runtime.top_return_value(), Some(Value::Int(1)));
        }
    }

    #[test]
    fn remainder_operations_works() {
        let test_files = vec!["support/Rem.class"];
        for test_file in test_files {
            println!("Testing : {test_file}");
            let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
            let path = Path::new(&env_var).join(test_file);
            let class_file_bytes = read_class_file(&path);
            let result = JVMParser::parse(&class_file_bytes);
            assert!(result.is_ok());
            let class_file = result.unwrap();
            let program = Program::new(&class_file);
            let mut runtime = Runtime::new(program);
            runtime.run();
            assert_eq!(runtime.top_return_value(), Some(Value::Int(2)));
        }
    }
}
