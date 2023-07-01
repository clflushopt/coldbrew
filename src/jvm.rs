//! Lightweight implementation of a parser and decoder for JVM class files.
use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;

use std::io;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

/// Values of magic bytes of a JVM class file.
const JVM_CLASS_FILE_MAGIC: u32 = 0xCAFE_BABE;

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
        reference_kind: u8,
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
            1 => Self::Utf8,
            3 => Self::Integer,
            4 => Self::Float,
            5 => Self::Long,
            6 => Self::Double,
            7 => Self::Class,
            8 => Self::String,
            9 => Self::FieldRef,
            10 => Self::MethodRef,
            12 => Self::InterfaceMethodRef,
            15 => Self::MethodHandle,
            16 => Self::MethodType,
            17 => Self::Dynamic,
            18 => Self::InvokeDynamic,
            _ => Self::Unspecified,
        }
    }
}

/// Verification type specifies the type of a single variable location or
/// a single operand stack entry.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

impl From<u8> for VerificationType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::TopVerification,
            1 => Self::IntegerVerification,
            2 => Self::FloatVerification,
            3 => Self::DoubleVerification,
            4 => Self::LongVerification,
            5 => Self::NullVerification,
            6 => Self::UninitializedThisVerification,
            7 => Self::ObjectVerification,
            8 => Self::UninitializedVerification,
            _ => panic!("Unexpected verification type entry {v}"),
        }
    }
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
        exception_table: Vec<ExceptionEntry>,
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

const ATTRIBUTE_NAME_CONSTANT_VALUE: &str = "ConstantValue";
const ATTRIBUTE_NAME_CODE: &str = "Code";
const ATTRIBUTE_NAME_STACK_MAP_TABLE: &str = "StackmapTable";
const ATTRIBUTE_NAME_SOURCE_FILE: &str = "SourceFile";
const ATTRIBUTE_NAME_BOOTSTRAP_METHODS: &str = "BootstrapMethods";
const ATTRIBUTE_NAME_NEST_HOST: &str = "NestHost";
const ATTRIBUTE_NAME_NEST_MEMBERS: &str = "NestMembers";

impl AttributeInfo {
    // Returns default attribute name for an attribute.
    const fn attribute_name(&self) -> &'static str {
        match self {
            Self::ConstantValueAttribute { .. } => "ConstantValue",
            Self::CodeAttribute { .. } => "Code",
            Self::StackMapTableAttribute { .. } => "StackMapTable",
            Self::SourceFileAttribute { .. } => "SourceFile",
            Self::BootstrapMethodsAttribute { .. } => "BootstrapMethods",
            Self::NestHostAttribute { .. } => "NestHost",
            Self::NestMembersAttribute { .. } => "NestMembers",
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
    attributes: HashMap<String, AttributeInfo>,
}

/// `JVMParser` namespaces functions that handle parsing of Java class files.
#[derive(Debug)]
pub struct JVMParser;

impl JVMParser {
    // Parse a preloaded Java class file.
    fn parse(class_file_bytes: &[u8]) -> io::Result<JVMClassFile> {
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
                ConstantKind::MethodHandle => {
                    let ref_kind = buffer.read_u8().unwrap();
                    let ref_index = buffer.read_u16::<BigEndian>().unwrap();
                    let value = CPInfo::ConstantMethodHandle {
                        reference_kind: ref_kind,
                        reference_index: ref_index,
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::MethodType => {
                    let desc_index = buffer.read_u16::<BigEndian>().unwrap();
                    let value = CPInfo::ConstantMethodType {
                        descriptor_index: desc_index,
                    };
                    constant_pool[ii] = value;
                }
                ConstantKind::InvokeDynamic => {
                    let bootstrap_method_attr_index =
                        buffer.read_u16::<BigEndian>().unwrap();
                    let name_and_type_index =
                        buffer.read_u16::<BigEndian>().unwrap();
                    let value = CPInfo::ConstantInvokeDynamic {
                        bootstrap_method_attr_index,
                        name_and_type_index,
                    };
                    constant_pool[ii] = value;
                }
                _ => panic!("Unexpected constant kind"),
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

        let (methods_count, methods) =
            parse_methods(&mut buffer, &constant_pool);

        let (attributes_count, attributes) =
            parse_attribute_info(&mut buffer, &constant_pool);

        let jvm_class_file = JVMClassFile {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces_count,
            interfaces,
            fields_count,
            fields,
            methods_count,
            methods,
            attributes_count,
            attributes,
        };
        Ok(jvm_class_file)
    }
}

/// Parse fields.
fn parse_fields(
    reader: &mut (impl Read + Seek),
    constant_pool: &[CPInfo],
) -> (u16, Vec<FieldInfo>) {
    let fields_count = reader.read_u16::<BigEndian>().unwrap();
    let mut fields: Vec<FieldInfo> = Vec::new();

    for _ in 0..fields_count {
        let access_flag = reader.read_u16::<BigEndian>().unwrap();
        let name_index = reader.read_u16::<BigEndian>().unwrap();
        let descriptor_index = reader.read_u16::<BigEndian>().unwrap();
        let (_, attributes) = parse_attribute_info(reader, constant_pool);
        fields.push(FieldInfo {
            access_flag,
            name_index,
            descriptor_index,
            attributes,
        });
    }

    (fields_count, fields)
}

/// Parse methods.
fn parse_methods(
    reader: &mut (impl Read + Seek),
    constant_pool: &[CPInfo],
) -> (u16, Vec<MethodInfo>) {
    let methods_count = reader.read_u16::<BigEndian>().unwrap();
    let mut methods: Vec<MethodInfo> = Vec::new();

    for _ in 0..methods_count {
        let access_flag = reader.read_u16::<BigEndian>().unwrap();
        let name_index = reader.read_u16::<BigEndian>().unwrap();
        let descriptor_index = reader.read_u16::<BigEndian>().unwrap();
        let (_, attributes) = parse_attribute_info(reader, constant_pool);
        methods.push(MethodInfo {
            access_flag,
            name_index,
            descriptor_index,
            attributes,
        });
    }

    (methods_count, methods)
}

/// Parse attributes.
fn parse_attribute_info(
    reader: &mut (impl Read + Seek),
    constant_pool: &[CPInfo],
) -> (u16, HashMap<String, AttributeInfo>) {
    let attribute_count = reader.read_u16::<BigEndian>().unwrap();
    let mut attributes: HashMap<String, AttributeInfo> = HashMap::new();
    for _ in 0..attribute_count {
        let attribute_name_index = reader.read_u16::<BigEndian>().unwrap();
        let attr_name = &constant_pool[attribute_name_index as usize];
        let attribute_name = match attr_name {
            CPInfo::ConstantUtf8 { bytes } => bytes.clone(),
            _ => panic!(
                "Expected attribute name to be CPInfo::ConstantUtf8 got {attr_name:?}",
            ),
        };
        let mut attribute_info: Option<AttributeInfo> = None;
        let attribute_length = reader.read_u32::<BigEndian>().unwrap();
        // TODO this can be done more idiomatically with a pattern match
        if attribute_name == "ConstantValue" {
            let const_value_index = reader.read_u16::<BigEndian>().unwrap();
            attribute_info = Some(AttributeInfo::ConstantValueAttribute {
                constant_value_index: const_value_index,
                attribute_name: attribute_name.clone(),
            });
        } else if attribute_name == "Code" {
            let max_stack = reader.read_u16::<BigEndian>().unwrap();
            let max_locals = reader.read_u16::<BigEndian>().unwrap();
            let code_length = reader.read_u32::<BigEndian>().unwrap();
            let mut buf = vec![0u8; code_length as usize];
            reader.read_exact(&mut buf).unwrap();
            let exception_table_length =
                reader.read_u16::<BigEndian>().unwrap();
            let mut exception_table_entries: Vec<ExceptionEntry> = Vec::new();
            for _ in 0..exception_table_length {
                let start_pc = reader.read_u16::<BigEndian>().unwrap();
                let end_pc = reader.read_u16::<BigEndian>().unwrap();
                let handler_pc = reader.read_u16::<BigEndian>().unwrap();
                let catch_type = reader.read_u16::<BigEndian>().unwrap();

                exception_table_entries.push(ExceptionEntry {
                    start_pc,
                    end_pc,
                    handler_pc,
                    catch_type,
                });
            }
            let (_, attributes) = parse_attribute_info(reader, constant_pool);
            attribute_info = Some(AttributeInfo::CodeAttribute {
                max_stack,
                max_locals,
                code: buf,
                exception_table: exception_table_entries,
                attributes,
                attribute_name: "Code".to_string(),
            });
        } else if attribute_name == "StackMapTable" {
            let number_of_entries = reader.read_u16::<BigEndian>().unwrap();
            let mut stack_map_entries: Vec<StackMapFrame> = Vec::new();
            for _ in 0..number_of_entries {
                let tag = reader.read_u8().unwrap();
                let frame = parse_stack_frame_entry(reader, tag);
                stack_map_entries.push(frame);
            }
            attribute_info = Some(AttributeInfo::StackMapTableAttribute {
                entries: stack_map_entries,
                attribute_name: "StackMapTable".to_string(),
            });
        } else if attribute_name == "SourceFile" {
            let source_file_index = reader.read_u16::<BigEndian>().unwrap();
            attribute_info = Some(AttributeInfo::SourceFileAttribute {
                source_file_index,
                attribute_name: "SourceFile".to_string(),
            });
        } else if attribute_name == "BootstrapMethods" {
            let num_bootstrap_methods = reader.read_u16::<BigEndian>().unwrap();
            let mut bootstrap_method_table: Vec<BootstrapMethod> = Vec::new();

            for _ in 0..num_bootstrap_methods {
                let method_ref = reader.read_u16::<BigEndian>().unwrap();
                let argument_count = reader.read_u16::<BigEndian>().unwrap();
                let mut arguments = Vec::new();
                for _ in 0..argument_count {
                    let arg = reader.read_u16::<BigEndian>().unwrap();
                    arguments.push(arg);
                }
                bootstrap_method_table.push(BootstrapMethod {
                    method_ref,
                    arguments,
                });
            }
            attribute_info = Some(AttributeInfo::BootstrapMethodsAttribute {
                bootstrap_methods: bootstrap_method_table,
                attribute_name: "BootstrapMethods".to_string(),
            });
        } else if attribute_name == "NestHost" {
            let host_class_index = reader.read_u16::<BigEndian>().unwrap();
            attribute_info = Some(AttributeInfo::NestHostAttribute {
                host_class_index,
                attribute_name: "NestHost".to_string(),
            });
        } else if attribute_name == "NestMembers" {
            let num_classes = reader.read_u16::<BigEndian>().unwrap();
            let mut classes = Vec::new();
            for _ in 0..num_classes {
                let class_index = reader.read_u16::<BigEndian>().unwrap();
                classes.push(class_index);
            }
            attribute_info = Some(AttributeInfo::NestMembersAttribute {
                classes,
                attribute_name: "NestMembers".to_string(),
            });
        } else {
            reader
                .seek(std::io::SeekFrom::Current(i64::from(attribute_length)))
                .unwrap();
        }
        attribute_info.map_or((), |attr| {
            attributes.insert(attribute_name.clone(), attr);
        });
    }
    (attribute_count, attributes)
}

/// Helper function to parse the `StackMapFrameTable` entry give a tag.
fn parse_stack_frame_entry(reader: &mut impl Read, tag: u8) -> StackMapFrame {
    match tag {
        0..=63 => StackMapFrame {
            t: StackMapFrameType::Same,
            offset_delta: 0,
            locals: vec![],
            stack: vec![],
        },
        64..=127 => StackMapFrame {
            t: StackMapFrameType::SameLocals,
            offset_delta: 0,
            locals: vec![],
            stack: parse_verification_info(reader, 1),
        },
        247 => StackMapFrame {
            t: StackMapFrameType::SameLocalsExtended,
            offset_delta: 0,
            locals: vec![],
            stack: parse_verification_info(reader, 1),
        },
        248 | 249 | 250 => StackMapFrame {
            t: StackMapFrameType::Chop,
            offset_delta: reader.read_u16::<BigEndian>().unwrap(),
            locals: vec![],
            stack: vec![],
        },
        251 => StackMapFrame {
            t: StackMapFrameType::SameExtended,
            offset_delta: reader.read_u16::<BigEndian>().unwrap(),
            locals: vec![],
            stack: vec![],
        },
        252 | 253 | 254 => StackMapFrame {
            t: StackMapFrameType::Append,
            offset_delta: reader.read_u16::<BigEndian>().unwrap(),
            locals: parse_verification_info(reader, (tag - 251).into()),
            stack: vec![],
        },
        255 => {
            let offset_delta = reader.read_u16::<BigEndian>().unwrap();
            let n_locals_entries = reader.read_u16::<BigEndian>().unwrap();
            let n_stack_entries = reader.read_u16::<BigEndian>().unwrap();
            StackMapFrame {
                t: StackMapFrameType::Full,
                offset_delta,
                locals: parse_verification_info(reader, n_locals_entries),
                stack: parse_verification_info(reader, n_stack_entries),
            }
        }
        _ => panic!("Unexpected tag entry {tag}"),
    }
}

/// Helper function parse verification info.
fn parse_verification_info(
    reader: &mut impl Read,
    num_entries: u16,
) -> Vec<VerificationInfo> {
    let mut verifications: Vec<VerificationInfo> = Vec::new();
    for _ in 0..num_entries {
        let tag = VerificationType::from(reader.read_u8().unwrap());
        let cpool_index_or_offset = if tag
            == VerificationType::ObjectVerification
            || tag == VerificationType::UninitializedVerification
        {
            reader.read_u16::<BigEndian>().unwrap()
        } else {
            0
        };
        verifications.push(VerificationInfo {
            tag,
            cpool_index_or_offset,
        });
    }
    verifications
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
        let result = JVMParser::parse(&class_file_bytes);
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
        let result = JVMParser::parse(&class_file_bytes);
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
}
