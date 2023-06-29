//! Lightweight implementation of a parser and decoder for JVM class files.
use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;

use std::io;
use std::io::{Cursor, Read};
use std::path::Path;

/// Values of magic bytes of a JVM class file.
const JVM_CLASS_FILE_MAGIC: u32 = 0xCAFEBABE;

/// `CPInfo` represents constant pool entries,
#[derive(Debug, Clone)]
enum CPInfo {
    ConstantClass {
        name_index: u16,
    },
    ConstantFieldRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantInterfaceMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantString {
        string_index: u16,
    },
    ConstantInteger {
        bytes: u32,
    },
    ConstantFloat {
        bytes: u32,
    },
    ConstantLong {
        hi_bytes: u32,
        lo_bytes: u32,
    },
    ConstantDouble {
        hi_bytes: u32,
        lo_bytes: u32,
    },
    ConstantNameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    ConstantUtf8 {
        bytes: String,
    },
    ConstantMethodHandle {
        reference_kind: u16,
        reference_index: u16,
    },
    ConstantMethodType {
        descriptor_index: u16,
    },
    ConstantInvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    // Proxy value used mostly to populate the gaps in the constant pool.
    Unspecified,
}

/// `ConstantKind` encodes the kind of a constant in the constants pool.
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum ConstantKind {
    Class = 7,
    FieldRef = 9,
    MethodRef = 10,
    InterfaceMethodRef = 11,
    String = 8,
    Integer = 3,
    Float = 4,
    Long = 5,
    Double = 6,
    NameAndType = 12,
    Utf8 = 1,
    MethodHandle = 15,
    MethodType = 16,
    Dynamic = 17,
    InvokeDynamic = 18,
    Module = 19,
    Package = 20,
    Unspecified,
}

impl From<u8> for ConstantKind {
    fn from(v: u8) -> Self {
        match v {
            1 => ConstantKind::Utf8,
            3 => ConstantKind::Integer,
            4 => ConstantKind::Float,
            5 => ConstantKind::Long,
            6 => ConstantKind::Double,
            7 => ConstantKind::Class,
            8 => ConstantKind::String,
            9 => ConstantKind::FieldRef,
            10 => ConstantKind::MethodRef,
            12 => ConstantKind::InterfaceMethodRef,
            15 => ConstantKind::MethodHandle,
            16 => ConstantKind::MethodType,
            17 => ConstantKind::Dynamic,
            18 => ConstantKind::InvokeDynamic,
            _ => ConstantKind::Unspecified,
        }
    }
}

/// Verification type specifies the type of a single variable location or
/// a single operand stack entry.
#[derive(Debug, Copy, Clone)]
enum VerificationType {
    TopVerification = 0,
    IntegerVerification = 1,
    FloatVerification = 2,
    LongVerification = 4,
    DoubleVerification = 3,
    NullVerification = 5,
    UninitializedThisVerification = 6,
    ObjectVerification = 7,
    UninitializedVerification = 8,
}

/// Verification info struct.
#[derive(Debug, Copy, Clone)]
struct VerificationInfo {
    tag: VerificationType,
    cpool_index_or_offset: u16,
}

/// Stack map frame type.
#[derive(Debug, Copy, Clone)]
enum StackMapFrameType {
    Same,
    SameLocals,
    SameLocalsExtended,
    Chop,
    SameExtended,
    Append,
    Full,
}

/// Stack map frame.
#[derive(Debug, Clone)]
struct StackMapFrame {
    t: StackMapFrameType,
    offset_delta: u16,
    locals: Vec<VerificationInfo>,
    stack: Vec<VerificationInfo>,
}

/// Bootstrap method.
#[derive(Debug, Clone)]
struct BootstrapMethod {
    method_ref: u16,
    arguments: Vec<u16>,
}

/// Exception table.
#[derive(Debug, Clone)]
struct ExceptionEntry {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: u16,
}

#[derive(Debug, Clone)]
enum AttributeInfo {
    ConstantValueAttribute {
        constant_value_index: u16,
        attribute_name: String,
    },
    CodeAttribute {
        max_stack: u16,
        max_locals: u16,
        code: Vec<u8>,
        // Exception table.
        attributes: HashMap<String, AttributeInfo>,
        attribute_name: String,
    },
    StackMapTableAttribute {
        entries: Vec<StackMapFrame>,
        attribute_name: String,
    },
    SourceFileAttribute {
        source_file_index: u16,
        attribute_name: String,
    },
    BootstrapMethodsAttribute {
        bootstrap_methods: Vec<BootstrapMethod>,
        attribute_name: String,
    },
    NestHostAttribute {
        host_class_index: u16,
        attribute_name: String,
    },
    NestMembersAttribute {
        classes: Vec<u16>,
        attribute_name: String,
    },
}

const ATTRIBUTE_NAME_CONSTANT_VALUE: &'static str = "ConstantValue";
const ATTRIBUTE_NAME_CODE: &'static str = "Code";
const ATTRIBUTE_NAME_STACK_MAP_TABLE: &'static str = "StackmapTable";
const ATTRIBUTE_NAME_SOURCE_FILE: &'static str = "SourceFile";
const ATTRIBUTE_NAME_BOOTSTRAP_METHODS: &'static str = "BootstrapMethods";
const ATTRIBUTE_NAME_NEST_HOST: &'static str = "NestHost";
const ATTRIBUTE_NAME_NEST_MEMBERS: &'static str = "ConstantValue";

impl AttributeInfo {
    // Returns default attribute name for an attribute.
    fn attribute_name(&self) -> &'static str {
        match self {
            ConstantValueAttribute => "ConstantValue",
            CodeAttribute => "Code",
            StackMapTableAttribute => "StackMapTable",
            SourceFileAttribute => "SourceFile",
            BootstrapMethodsAttribute => "BootstrapMethods",
            NestHostAttribute => "NestHost",
            NestMembersAttribute => "NestMembers",
        }
    }
}

#[derive(Debug, Clone)]
struct FieldInfo {
    access_flag: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: HashMap<String, AttributeInfo>,
}

#[derive(Debug, Clone)]
struct MethodInfo {
    access_flag: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: HashMap<String, AttributeInfo>,
}

/// `JVMClassFile` represents a Java class file.
#[derive(Debug, Clone)]
pub struct JVMClassFile {
    magic: u32,
    minor_version: u16,
    major_version: u16,
    constant_pool_count: u16,
    constant_pool: Vec<CPInfo>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces_count: u16,
    interfaces: Vec<u16>,
    fields_count: u16,
    fields: Vec<FieldInfo>,
    methods_count: u16,
    methods: Vec<MethodInfo>,
    attributes_count: u16,
    attributes: Vec<AttributeInfo>,
}

/// `JVMParser` namespaces functions that handle parsing of Java class files.
#[derive(Debug)]
pub struct JVMParser;

impl JVMParser {
    #[must_use]
    // Creates a new JVMParser with a given Java class file to parse.
    pub fn new() -> Self {
        Self {}
    }

    // Parse a preloaded Java class file.
    fn parse(&self, class_file_bytes: &[u8]) -> io::Result<JVMClassFile> {
        // Create a new cursor on the class file bytes.
        let mut buffer = Cursor::new(class_file_bytes);
        // Read magic header..
        let magic = buffer.read_u32::<BigEndian>()?;
        // Read the class file version numbers.
        let minor_version = buffer.read_u16::<BigEndian>()?;
        let major_version = buffer.read_u16::<BigEndian>()?;
        // Read the number of constants in the pool.
        let constant_pool_count = buffer.read_u16::<BigEndian>()?;
        // Allocate a new pool and populate it with the constants.
        // let mut constant_pool = Vec::with_capacity(constant_pool_count.into());
        let mut constant_pool =
            vec![CPInfo::Unspecified; constant_pool_count as usize];
        // The first entry in the pool is at index 1 according to JVM
        // spec.
        for mut ii in 1..constant_pool_count as usize {
            let tag = buffer.read_u8()?;
            match ConstantKind::from(tag) {
                ConstantKind::Class => {
                    let value = CPInfo::ConstantClass {
                        name_index: buffer.read_u16::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::FieldRef => {
                    let value = CPInfo::ConstantFieldRef {
                        class_index: buffer.read_u16::<BigEndian>().unwrap(),
                        name_and_type_index: buffer
                            .read_u16::<BigEndian>()
                            .unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::MethodRef => {
                    let value = CPInfo::ConstantMethodRef {
                        class_index: buffer.read_u16::<BigEndian>().unwrap(),
                        name_and_type_index: buffer
                            .read_u16::<BigEndian>()
                            .unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::InterfaceMethodRef => {
                    let value = CPInfo::ConstantInterfaceMethodRef {
                        class_index: buffer.read_u16::<BigEndian>().unwrap(),
                        name_and_type_index: buffer
                            .read_u16::<BigEndian>()
                            .unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::String => {
                    let value = CPInfo::ConstantString {
                        string_index: buffer.read_u16::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::Integer => {
                    let value = CPInfo::ConstantInteger {
                        bytes: buffer.read_u32::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::Float => {
                    let value = CPInfo::ConstantFloat {
                        bytes: buffer.read_u32::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::Long => {
                    let value = CPInfo::ConstantLong {
                        hi_bytes: buffer.read_u32::<BigEndian>().unwrap(),
                        lo_bytes: buffer.read_u32::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                    ii += 1;
                }
                ConstantKind::Double => {
                    let value = CPInfo::ConstantDouble {
                        hi_bytes: buffer.read_u32::<BigEndian>().unwrap(),
                        lo_bytes: buffer.read_u32::<BigEndian>().unwrap(),
                    };
                    constant_pool[ii] = value;
                    ii += 1;
                }
                ConstantKind::NameAndType => {
                    let value = CPInfo::ConstantNameAndType {
                        name_index: buffer.read_u16::<BigEndian>().unwrap(),
                        descriptor_index: buffer
                            .read_u16::<BigEndian>()
                            .unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::Utf8 => {
                    let length = buffer.read_u16::<BigEndian>().unwrap();
                    let mut buf = vec![0u8; length as usize];
                    buffer.read_exact(&mut buf).unwrap();
                    let value = CPInfo::ConstantUtf8 {
                        bytes: String::from_utf8(buf).unwrap(),
                    };
                    constant_pool[ii] = value;
                }
                _ => println!("found : {}", tag),
            }
        }

        let access_flags = buffer.read_u16::<BigEndian>()?;
        let this_class = buffer.read_u16::<BigEndian>()?;
        let super_class = buffer.read_u16::<BigEndian>()?;

        let interfaces_count = buffer.read_u16::<BigEndian>()?;
        let mut interfaces = Vec::new();

        for _ in 0..interfaces_count {
            let interface = buffer.read_u16::<BigEndian>()?;
            interfaces.push(interface);
        }

        let (fields_count, fields) = parse_fields(&mut buffer, &constant_pool);

        let jvm_class_file = JVMClassFile {
            magic: magic,
            minor_version: minor_version,
            major_version: major_version,
            constant_pool_count: constant_pool_count,
            constant_pool: constant_pool,
            access_flags: access_flags,
            this_class: this_class,
            super_class: super_class,
            interfaces_count: interfaces_count,
            interfaces: interfaces,
            fields_count: fields_count,
            fields: fields,
            methods_count: 0,
            methods: Vec::new(),
            attributes_count: 0,
            attributes: Vec::new(),
        };
        Ok(jvm_class_file)
    }
}

/// Parse fields.
fn parse_fields(
    reader: &mut impl Read,
    constant_pool: &[CPInfo],
) -> (u16, Vec<FieldInfo>) {
    let fields_count = reader.read_u16::<BigEndian>().unwrap();
    let mut fields: Vec<FieldInfo> = Vec::new();

    for _ in 0..fields_count {
        let access_flag = reader.read_u16::<BigEndian>().unwrap();
        let name_index = reader.read_u16::<BigEndian>().unwrap();
        let descriptor_index = reader.read_u16::<BigEndian>().unwrap();
        // let attributes = parse_attribute_info(reader, constant_pool);
        fields.push(FieldInfo {
            access_flag: access_flag,
            name_index: name_index,
            descriptor_index: descriptor_index,
            attributes: HashMap::new(),
        });
    }

    (fields_count, fields)
}

/// Parse attributes.
fn parse_attribute_info(reader: &mut impl Read, constant_pool: &[CPInfo]) {
    let attribute_count = reader.read_u16::<BigEndian>().unwrap();
    let attributes: Vec<AttributeInfo> = Vec::new();

    for _ in 0..attribute_count {
        let attribute_name_index = reader.read_u16::<BigEndian>().unwrap();
        let attribute_name = match &constant_pool[attribute_name_index as usize]
        {
            CPInfo::ConstantUtf8 { bytes } => bytes.clone(),
            _ => panic!("Expected attribute name to be CPInfo::ConstantUtf8"),
        };
        let attribute_length = reader.read_u32::<BigEndian>().unwrap();
        if attribute_name == "ConstantValue" {}
        println!("{:?}", attribute_name)
    }
}

/// Helper function to read file into a buffer.
fn read_class_file(fp: &Path) -> Vec<u8> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = File::open(fp).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;

    #[test]
    fn can_you_read_class_file() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/SingleFuncCall.class");
        let class_file_bytes = read_class_file(&path);
        let result = JVMParser::new().parse(&class_file_bytes);
        assert!(result.is_ok());
        let class_file = result.unwrap();
        assert_eq!(JVM_CLASS_FILE_MAGIC, class_file.magic);
        assert!(
            class_file.minor_version == 0 || class_file.minor_version == 65535
        );
        assert!(class_file.major_version > 61);
    }

    #[test]
    fn can_parse_class_file_header() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/SingleFuncCall.class");
        let class_file_bytes = read_class_file(&path);
        let result = JVMParser::new().parse(&class_file_bytes);
        assert!(result.is_ok());
        let class_file = result.unwrap();
        let expected_strings = vec![
            "java/lang/Object",
            "<init>",
            "SingleFuncCall",
            "(II)I",
            "java/lang/System",
            "Ljava/io/PrintStream;",
            "java/io/PrintStream",
            "println",
            "(I)V",
            "Code",
            "LineNumberTable",
            "main",
            "([Ljava/lang/String;)V",
            "SourceFile",
            "SingleFuncCall.java",
        ];
        let mut actual_strings = Vec::new();
        for constant in &class_file.constant_pool {
            match constant {
                CPInfo::ConstantUtf8 { bytes } => {
                    actual_strings.push(bytes.clone())
                }
                _ => (),
            }
        }
        for s in expected_strings {
            assert!(actual_strings.contains(&s.to_string()));
        }
        println!("{:?}", class_file);
    }
    #[test]
    fn can_check_attribute_name() {
        let attr_info = AttributeInfo::ConstantValueAttribute {
            constant_value_index: 12u16,
            attribute_name: ATTRIBUTE_NAME_CONSTANT_VALUE.to_string(),
        };
        println!("{}", attr_info.attribute_name());
    }
}
