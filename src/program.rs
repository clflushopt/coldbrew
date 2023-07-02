//! Abstract representation of a Java program.
use crate::jvm::*;
use std::collections::HashMap;

use regex::Regex;

/// Primitive types supported by the JVM.
#[derive(Debug, Copy, Clone)]
pub enum BaseTypeKind {
    Int,
    Long,
    Float,
    Double,
    Void,
    String,
    List,
}

/// JVM value type.
#[derive(Debug, Clone)]
pub struct Type {
    t: BaseTypeKind,
    sub_t: Option<Box<Type>>,
}

/// JVM value types.
#[derive(Debug, Copy, Clone)]
enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
}

/// Representation of Java programs that we want to run.
#[derive(Debug, Clone)]
pub struct Program {
    // Constant pool.
    constant_pool: Vec<CPInfo>,
    // Methods.
    methods: HashMap<u16, Method>,
}

/// Java class method representation for the interpreter.
#[derive(Debug, Clone)]
struct Method {
    name_index: u16,
    return_type: Type,
    arg_types: Vec<Type>,
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    constant: Option<u16>,
    stack_map_table: Option<Vec<StackMapFrame>>,
}

impl Program {
    // Build a new program from a parsed class file.
    pub fn new(class_file: &JVMClassFile) -> Self {
        let constants = class_file.constant_pool();
        let mut methods: HashMap<u16, Method> = HashMap::new();
        for method_info in &class_file.methods() {
            let mut arg_types: Vec<Type> = Vec::new();
            let mut ret_type: Type = Type {
                t: BaseTypeKind::Void,
                sub_t: None,
            };
            let descriptor =
                &constants[method_info.descriptor_index() as usize];
            let _method_name = &constants[method_info.name_index() as usize];
            match descriptor {
                CPInfo::ConstantUtf8 { bytes } => {
                    println!("Utf8 bytes : {}", bytes);
                    (arg_types, ret_type) = Program::parse_method_types(bytes);
                }
                _ => (),
            }
            let attr = method_info.attributes();

            let (max_stack, max_locals, code) =
                if let Some(AttributeInfo::CodeAttribute {
                    max_stack,
                    max_locals,
                    code,
                    ..
                }) = attr.get("Code")
                {
                    (*max_stack, *max_locals, code.clone())
                } else {
                    panic!("Expected at least one code attribute")
                };

            let constant =
                if let Some(AttributeInfo::ConstantValueAttribute {
                    constant_value_index,
                    ..
                }) = attr.get("ConstantValue")
                {
                    Some(*constant_value_index)
                } else {
                    None
                };

            let stack_map_table =
                if let Some(AttributeInfo::StackMapTableAttribute {
                    entries,
                    ..
                }) = attr.get("StackMapTable")
                {
                    Some(entries.clone())
                } else {
                    None
                };

            let method = Method {
                name_index: method_info.name_index(),
                return_type: ret_type,
                arg_types: arg_types,
                max_stack: max_stack,
                max_locals: max_locals,
                code: code,
                constant: constant,
                stack_map_table: stack_map_table,
            };
            methods.insert(method_info.name_index(), method);
        }

        Self {
            // Get a copy of the constant pool.
            constant_pool: class_file.constant_pool(),
            // Get a copy of the program methods.
            methods: methods,
        }
    }

    // Parse constant method types, returns a tuple of argument types and
    // return types.
    fn parse_method_types(bytes: &str) -> (Vec<Type>, Type) {
        let re = Regex::new(r"\(([^\)]*)\)([^$]+)").unwrap();
        let caps = re.captures(&bytes).unwrap();
        let arg_string = caps.get(1).map_or("", |m| m.as_str());
        let return_type_string = caps.get(2).map_or("", |m| m.as_str());
        let mut types: Vec<Type> = Vec::new();
        let ret_type = Program::decode_type(return_type_string);

        let mut arg_string_slice = &arg_string[..];
        while arg_string_slice.len() > 0 {
            let t = Program::decode_type(arg_string_slice);
            types.push(t.clone());
            let length = Program::decode_type_string_length(&t);
            arg_string_slice = substr(
                &arg_string_slice,
                length,
                arg_string_slice.len() - length,
            );
        }
        (types, ret_type)
    }

    // Get type string representation length.
    pub fn decode_type_string_length(t: &Type) -> usize {
        match t.t {
            BaseTypeKind::String => 18,
            BaseTypeKind::List => {
                1 + Self::decode_type_string_length(
                    &(t.sub_t.as_ref().unwrap()),
                )
            }
            _ => 1,
        }
    }

    // Decode Java type from string.
    pub fn decode_type(type_str: &str) -> Type {
        match &type_str[0..1] {
            "I" => Type {
                t: BaseTypeKind::Int,
                sub_t: None,
            },
            "J" => Type {
                t: BaseTypeKind::Long,
                sub_t: None,
            },
            "F" => Type {
                t: BaseTypeKind::Float,
                sub_t: None,
            },
            "D" => Type {
                t: BaseTypeKind::Double,
                sub_t: None,
            },
            "V" => Type {
                t: BaseTypeKind::Void,
                sub_t: None,
            },
            "[" => {
                let st = Self::decode_type(&type_str[1..(type_str.len() - 1)]);
                let subtype = Type {
                    t: st.t,
                    sub_t: st.sub_t,
                };
                Type {
                    t: BaseTypeKind::List,
                    sub_t: Some(Box::new(subtype)),
                }
            }
            // We can support byte, char... later
            _ => Type {
                t: BaseTypeKind::String,
                sub_t: None,
            },
        }
    }
}

fn substr(s: &str, start: usize, length: usize) -> &str {
    let end = start + length;
    &s[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::path::Path;

    #[test]
    fn can_build_program() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/Factorial.class");
        let class_file_bytes = read_class_file(&path);
        let result = JVMParser::parse(&class_file_bytes);
        assert!(result.is_ok());
        let class_file = result.unwrap();
        let program = Program::new(&class_file);
        println!("{:?}", program);
    }

    #[test]
    fn can_decode_class_file_program() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/Factorial.class");
        let class_file_bytes = read_class_file(&path);
        let result = JVMParser::parse(&class_file_bytes);
        assert!(result.is_ok());
        let class_file = result.unwrap();
        let program = Program::new(&class_file);
        println!("{:?}", program);
        /*
        println!("{:?}", class_file);
        let constants = class_file.constant_pool();

        // let re = Regex::new(r"\(([^\\)]*)\)([^$]+)").unwrap();
        //"\(([^\)]*)\)([^$]+)"
        let re = Regex::new(r"\(([^\)]*)\)([^$]+)").unwrap();
        // let mut methods: Vec<MethodInfo> = Vec::new();
        let mut methods: HashMap<u16, Method> = HashMap::new();

        for method_info in &class_file.methods() {
            let mut arg_types: Vec<Type> = Vec::new();
            let mut ret_type: Type = Type {
                t: BaseTypeKind::Void,
                sub_t: None,
            };
            let descriptor =
                &constants[method_info.descriptor_index() as usize];
            let method_name = &constants[method_info.name_index() as usize];
            match descriptor {
                CPInfo::ConstantUtf8 { bytes } => {
                    println!("Utf8 bytes : {}", bytes);
                    (arg_types, ret_type) = Program::parse_method_types(bytes);
                }
                _ => (),
            }
            let attr = method_info.attributes();
            println!("Attribute : {:?}", attr);

            let (max_stack,max_locals,code) = if let AttributeInfo::CodeAttribute{max_stack,max_locals,code,..} = &attr["Code"] {
                    (*max_stack, *max_locals, code.clone())
            } else {
                panic!("Expected at least one code attribute")
            };

            let method = Method {
                name_index: method_info.name_index(),
                return_type: ret_type,
                arg_types: arg_types,
                max_stack: max_stack,
                max_locals: max_locals,
                code: code,
            };
            println!("Found method : {:?}", method);
            methods.insert(method_info.name_index(), method);

        }
        */
    }
}
