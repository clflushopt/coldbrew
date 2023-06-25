//! Lightweight implementation of a parser and decoder for JVM bytecode
//! class files.

/// Values of magic bytes of a JVM class file.
const JVM_CLASS_FILE_MAGIC: u32 = 0xCAFEBABE;

/// `CPInfo` represents constant pool entries.
#[derive(Debug, Clone)]
struct CPInfo {
    // Value of `ConstantKind` indicates the kind of the constant represented
    // by this entry.
    tag: u8,
    info: Vec<u8>,
}

/// `ConstantKind` encodes the kind of a constant in the constants pool.
#[derive(Debug, Copy, Clone)]
enum ConstantKind {
    Class = 7,
    FieldRef = 9,
    MethodRef = 10,
    InterfaceMethodref = 11,
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
}

#[derive(Debug, Copy, Clone)]
struct FieldInfo;

#[derive(Debug, Copy, Clone)]
struct MethodInfo;

#[derive(Debug, Copy, Clone)]
struct AttributeInfo;

/// `JVMClassFile` represents a Java class file.
#[derive(Debug, Clone)]
struct JVMClassFile {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;

    #[test]
    fn can_you_read_class_file() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join("support/SingleFuncCall.class");

        use std::fs::File;
        use std::io;
        use std::io::prelude::*;

        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();
        assert_eq!(
            JVM_CLASS_FILE_MAGIC,
            u32::from_be_bytes(buffer[..4].try_into().unwrap())
        );
    }
}
