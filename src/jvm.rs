//! Lightweight implementation of a parser and decoder for JVM bytecode
//! class files.

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;

    #[test]
    fn can_you_read_class_file() {
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(
           &env_var,
        ).join("support/SingleFuncCall.class");

        use std::io;
        use std::io::prelude::*;
        use std::fs::File;

        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();
        assert_eq!(0xcafebabe, u32::from_be_bytes(buffer[..4].try_into().unwrap()));
    }


}
