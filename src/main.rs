use coldbrew::jvm::{read_class_file, JVMParser};
use coldbrew::program::Program;
use coldbrew::runtime::Runtime;

fn main() {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let to_skip: Vec<&str> = vec![
        "DoubleFibonacci.class",
        "MixedTypes.class",
        "MixedArg.class",
    ];
    for path in std::path::Path::new(r"./support/integration/")
        .read_dir()
        .unwrap()
    {
        let path = match path {
            Ok(entry) => entry.path(),
            Err(err) => {
                panic!("Error occured when reading file paths : {}", err)
            }
        };
        if let Some(extension) = path.extension() {
            println!("File : {:?}", path.file_name());
            if to_skip.contains(&path.file_name().unwrap().to_str().unwrap()) {
                continue;
            }
            if extension == "class" {
                paths.push(path);
            }
        }
    }
    for path in &paths {
        // What are the program components ?
        // 1. Reads and parse Java class files.
        println!("[+] Reading class file {:?}", path.as_os_str());
        let class_file_bytes = read_class_file(path).unwrap_or_else(|_| {
            panic!("Failed to read class file : {:?}", path.as_os_str())
        });
        let class_file =
            JVMParser::parse(&class_file_bytes).unwrap_or_else(|_| {
                panic!("Failed to parse class file {:?}", path.as_os_str())
            });

        // 2. Build abstract program from class file to run in the interpreter.
        println!("[+] Building program");
        let program = Program::new(&class_file);
        // 3. Interepreter executes bytecode and records a trace.
        //  When trace is hot it is compiled to assembly
        let mut runtime = Runtime::new(program);
        match runtime.run() {
            Ok(()) => {
                println!("[+] All programs were run successfully !");
            }
            Err(err) => println!("Error : {err}"),
        }
    }
}
