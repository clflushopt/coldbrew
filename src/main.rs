use std::env;
use std::process::exit;

use coldbrew::jvm::{read_class_file, JVMParser};
use coldbrew::program::Program;
use coldbrew::runtime::Runtime;

const USAGE_CMD: &str = "
    Coldbrew Tracing JIT usage guide :

    Run `coldbrew unit` to run small test programs (interpreter only).
    Run `coldbrew integration` to run end to end CPU intensive test programs (interpreter only).
    Run `coldbrew jit` to run small test programs with hot loops (interpreter + tracing jit).
    Run `coldbrew help` to see this message.
";

fn main() {
    // Decide which test files to run.
    let args: Vec<String> = env::args().collect();
    let jit_mode = args[1].as_str() == "jit";
    assert!(
        (args.len() >= 2),
        "Unexpected argument use `coldbrew help` to see usage guide."
    );
    let folder = match args[1].as_str() {
        "unit" => "./support/tests/",
        "integration" => "./support/integration/",
        "jit" => "./support/jit/",
        "help" => {
            println!("{USAGE_CMD}");
            exit(0);
        }
        _ => {
            println!(
                "Unexpected argument use `coldbrew help` to see usage guide."
            );
            exit(64);
        }
    };

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let to_skip: Vec<&str> = vec![
        "DoubleFibonacci.class",
        "MixedTypes.class",
        "MixedArg.class",
        "MEDouble.class",
        "FloatFibonacci.class",
        "LongFibonacci.class",
    ];
    for path in std::path::Path::new(folder).read_dir().unwrap() {
        let path = match path {
            Ok(entry) => entry.path(),
            Err(err) => {
                println!("Error occured when reading file paths : {err}");
                exit(1);
            }
        };
        if let Some(extension) = path.extension() {
            if to_skip.contains(&path.file_name().unwrap().to_str().unwrap()) {
                continue;
            }
            if extension == "class" {
                paths.push(path);
            }
        }
    }
    for path in &paths {
        let class_file_bytes = read_class_file(path).unwrap_or_else(|_| {
            panic!("Failed to read class file : {:?}", path.as_os_str())
        });
        let class_file =
            JVMParser::parse(&class_file_bytes).unwrap_or_else(|_| {
                panic!("Failed to parse class file {:?}", path.as_os_str())
            });

        let program = Program::new(&class_file);
        let mut runtime = Runtime::new(program);
        match runtime.run(jit_mode) {
            Ok(()) => {
                println!(
                    "[+] Program {:?} finished running successfully !",
                    path.file_name().unwrap()
                );
            }
            Err(err) => println!("Error : {err}"),
        }
    }
}
