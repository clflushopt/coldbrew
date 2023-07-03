//! JVM bytecode definitions.
use crate::program::{BaseTypeKind, Type};

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

/// Bytecode instructions are composed of an opcode and list of optional
/// arguments or parameters.
#[derive(Debug, Clone)]
struct Instruction {
    mnemonic: OPCode,
    params: Vec<Value>,
}

/// OPCodes supported by the JVM as documented in the spec document.
/// ref: https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-7.html
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum OPCode {
    NOP,
    AconstNULL,
    IconstM1,
    Iconst0,
    Iconst1,
    Iconst2,
    Iconst3,
    Iconst4,
    Iconst5,
    Lconst0,
    Lconst1,
    Fconst0,
    Fconst1,
    Fconst2,
    Dconst0,
    Dconst1,
    BiPush,
    SiPush,
    Ldc,
    LdcW,
    Ldc2W,
    Iload,
    Lload,
    Fload,
    Dload,
    Aload,
    Iload0,
    Iload1,
    Iload2,
    Iload3,
    Lload0,
    Lload1,
    Lload2,
    Lload3,
    Fload0,
    Fload1,
    Fload2,
    Fload3,
    Dload0,
    Dload1,
    Dload2,
    Dload3,
    Aload0,
    Aload1,
    Aload2,
    Aload3,
    IAload,
    LAload,
    FAload,
    DAload,
    AAload,
    BAload,
    CAload,
    SAload,
    Istore,
    Lstore,
    Fstore,
    Dstore,
    Astore,
    Istore0,
    Istore1,
    Istore2,
    Istore3,
    Lstore0,
    Lstore1,
    Lstore2,
    Lstore3,
    Fstore0,
    Fstore1,
    Fstore2,
    Fstore3,
    Dstore0,
    Dstore1,
    Dstore2,
    Dstore3,
    Astore0,
    Astore1,
    Astore2,
    Astore3,
    IAstore,
    LAstore,
    FAstore,
    DAstore,
    AAstore,
    BAstore,
    CAstore,
    SAstore,
    Pop,
    Pop2,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Swap,
    IAdd,
    LAdd,
    FAdd,
    DAdd,
    ISub,
    LSub,
    FSub,
    DSub,
    IMul,
    LMul,
    FMul,
    DMul,
    IDiv,
    LDiv,
    FDiv,
    DDiv,
    IRem,
    LRem,
    FRem,
    DRem,
    INeg,
    LNeg,
    FNeg,
    DNeg,
    IShl,
    LShl,
    IShr,
    LShr,
    IUShr,
    LUShr,
    Iand,
    Land,
    IOr,
    LOr,
    IXor,
    LXor,
    IInc,
    I2L,
    I2F,
    I2D,
    L2I,
    L2F,
    L2D,
    F2I,
    F2L,
    F2D,
    D2I,
    D2L,
    D2F,
    I2B,
    I2C,
    I2S,
    LCmp,
    FCmpL,
    FCmpG,
    DCmpL,
    DCmpG,
    IFEq,
    IFNe,
    IFLt,
    IFGe,
    IFGt,
    IFLe,
    IfICmpEq,
    IfICmpNe,
    IfICmpLt,
    IfICmpGe,
    IfICmpGt,
    IfICmpLe,
    IfACmpEq,
    IfACmpNe,
    Goto,
    Jsr,
    Ret,
    TableSwitch,
    LookupSwitch,
    IReturn,
    LReturn,
    FReturn,
    DReturn,
    AReturn,
    Return,
    GetStatic,
    PutStatic,
    GetField,
    PutField,
    InvokeVirtual,
    InvokeSpecial,
    InvokeStatic,
    InvokeInterface,
    InvokeDynamic,
    New,
    NewArray,
    ANewArray,
    ArrayLength,
    AThrow,
    CheckCast,
    InstanceOf,
    MonitorEnter,
    MonitorExit,
    Wide,
    MultiANewArray,
    IfNull,
    IfNonNull,
    GotoW,
    JsrW,
    Breakpoint,
    // Proxy value to signal unknown opcode values.
    Unspecified,
}

// Since bytecode is initially loaded as `Vec<u8>` we need a way to convert it
// to `OPCode` enum, this might be done better with a macro but copy paste and
// move on for now.
impl From<u8> for OPCode {
    fn from(byte: u8) -> Self {
        match byte {
            0 => Self::NOP,
            1 => Self::AconstNULL,
            2 => Self::IconstM1,
            3 => Self::Iconst0,
            4 => Self::Iconst1,
            5 => Self::Iconst2,
            6 => Self::Iconst3,
            7 => Self::Iconst4,
            8 => Self::Iconst5,
            9 => Self::Lconst0,
            10 => Self::Lconst1,
            11 => Self::Fconst0,
            12 => Self::Fconst1,
            13 => Self::Fconst2,
            14 => Self::Dconst0,
            15 => Self::Dconst1,
            16 => Self::BiPush,
            17 => Self::SiPush,
            18 => Self::Ldc,
            19 => Self::LdcW,
            20 => Self::Ldc2W,
            21 => Self::Iload,
            22 => Self::Lload,
            23 => Self::Fload,
            24 => Self::Dload,
            25 => Self::Aload,
            26 => Self::Iload0,
            27 => Self::Iload1,
            28 => Self::Iload2,
            29 => Self::Iload3,
            30 => Self::Lload0,
            31 => Self::Lload1,
            32 => Self::Lload2,
            33 => Self::Lload3,
            34 => Self::Fload0,
            35 => Self::Fload1,
            36 => Self::Fload2,
            37 => Self::Fload3,
            38 => Self::Dload0,
            39 => Self::Dload1,
            40 => Self::Dload2,
            41 => Self::Dload3,
            42 => Self::Aload0,
            43 => Self::Aload1,
            44 => Self::Aload2,
            45 => Self::Aload3,
            46 => Self::IAload,
            47 => Self::LAload,
            48 => Self::FAload,
            49 => Self::DAload,
            50 => Self::AAload,
            51 => Self::BAload,
            52 => Self::CAload,
            53 => Self::SAload,
            54 => Self::Istore,
            55 => Self::Lstore,
            56 => Self::Fstore,
            57 => Self::Dstore,
            58 => Self::Astore,
            59 => Self::Istore0,
            60 => Self::Istore1,
            61 => Self::Istore2,
            62 => Self::Istore3,
            63 => Self::Lstore0,
            64 => Self::Lstore1,
            65 => Self::Lstore2,
            66 => Self::Lstore3,
            67 => Self::Fstore0,
            68 => Self::Fstore1,
            69 => Self::Fstore2,
            70 => Self::Fstore3,
            71 => Self::Dstore0,
            72 => Self::Dstore1,
            73 => Self::Dstore2,
            74 => Self::Dstore3,
            75 => Self::Astore0,
            76 => Self::Astore1,
            77 => Self::Astore2,
            78 => Self::Astore3,
            79 => Self::IAstore,
            80 => Self::LAstore,
            81 => Self::FAstore,
            82 => Self::DAstore,
            83 => Self::AAstore,
            84 => Self::BAstore,
            85 => Self::CAstore,
            86 => Self::SAstore,
            87 => Self::Pop,
            88 => Self::Pop2,
            89 => Self::Dup,
            90 => Self::DupX1,
            91 => Self::DupX2,
            92 => Self::Dup2,
            93 => Self::Dup2X1,
            94 => Self::Dup2X2,
            95 => Self::Swap,
            96 => Self::IAdd,
            97 => Self::LAdd,
            98 => Self::FAdd,
            99 => Self::DAdd,
            100 => Self::ISub,
            101 => Self::LSub,
            102 => Self::FSub,
            103 => Self::DSub,
            104 => Self::IMul,
            105 => Self::LMul,
            106 => Self::FMul,
            107 => Self::DMul,
            108 => Self::IDiv,
            109 => Self::LDiv,
            110 => Self::FDiv,
            111 => Self::DDiv,
            112 => Self::IRem,
            113 => Self::LRem,
            114 => Self::FRem,
            115 => Self::DRem,
            116 => Self::INeg,
            117 => Self::LNeg,
            118 => Self::FNeg,
            119 => Self::DNeg,
            120 => Self::IShl,
            121 => Self::LShl,
            122 => Self::IShr,
            123 => Self::LShr,
            124 => Self::IUShr,
            125 => Self::LUShr,
            126 => Self::Iand,
            127 => Self::Land,
            128 => Self::IOr,
            129 => Self::LOr,
            130 => Self::IXor,
            131 => Self::LXor,
            132 => Self::IInc,
            133 => Self::I2L,
            134 => Self::I2F,
            135 => Self::I2D,
            136 => Self::L2I,
            137 => Self::L2F,
            138 => Self::L2D,
            139 => Self::F2I,
            140 => Self::F2L,
            141 => Self::F2D,
            142 => Self::D2I,
            143 => Self::D2L,
            144 => Self::D2F,
            145 => Self::I2B,
            146 => Self::I2C,
            147 => Self::I2S,
            148 => Self::LCmp,
            149 => Self::FCmpL,
            150 => Self::FCmpG,
            151 => Self::DCmpL,
            152 => Self::DCmpG,
            153 => Self::IFEq,
            154 => Self::IFNe,
            155 => Self::IFLt,
            156 => Self::IFGe,
            157 => Self::IFGt,
            158 => Self::IFLe,
            159 => Self::IfICmpEq,
            160 => Self::IfICmpNe,
            161 => Self::IfICmpLt,
            162 => Self::IfICmpGe,
            163 => Self::IfICmpGt,
            164 => Self::IfICmpLe,
            165 => Self::IfACmpEq,
            166 => Self::IfACmpNe,
            167 => Self::Goto,
            168 => Self::Jsr,
            169 => Self::Ret,
            170 => Self::TableSwitch,
            171 => Self::LookupSwitch,
            172 => Self::IReturn,
            173 => Self::LReturn,
            174 => Self::FReturn,
            175 => Self::DReturn,
            176 => Self::AReturn,
            177 => Self::Return,
            178 => Self::GetStatic,
            179 => Self::PutStatic,
            180 => Self::GetField,
            181 => Self::PutField,
            182 => Self::InvokeVirtual,
            183 => Self::InvokeSpecial,
            184 => Self::InvokeStatic,
            185 => Self::InvokeInterface,
            186 => Self::InvokeDynamic,
            187 => Self::New,
            188 => Self::NewArray,
            189 => Self::ANewArray,
            190 => Self::ArrayLength,
            191 => Self::AThrow,
            192 => Self::CheckCast,
            193 => Self::InstanceOf,
            194 => Self::MonitorEnter,
            195 => Self::MonitorExit,
            196 => Self::Wide,
            197 => Self::MultiANewArray,
            198 => Self::IfNull,
            199 => Self::IfNonNull,
            200 => Self::GotoW,
            201 => Self::JsrW,
            202 => Self::Breakpoint,
            203..=u8::MAX => Self::Unspecified,
        }
    }
}