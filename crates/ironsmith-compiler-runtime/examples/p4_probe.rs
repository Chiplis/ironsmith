//! Scratch probe (temporary): compile `<name>` with oracle text from stdin and print the definition.
use std::io::Read;
fn main() {
    std::thread::Builder::new().stack_size(1 << 29).spawn(run).unwrap().join().unwrap();
}
fn run() {
    let name = std::env::args().nth(1).unwrap_or_else(|| "Probe".to_string());
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();
    if std::env::var_os("P4_TRACE").is_some() {
        let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
            ironsmith_compiler_runtime::compile_to_runtime_definition(&name, &text, false)
        });
        println!("{}", trace.render());
        match definition {
            Ok(definition) => println!("{definition:#?}"),
            Err(error) => println!("ERROR {error:?}"),
        }
        return;
    }
    match ironsmith_compiler_runtime::compile_to_runtime_definition(&name, &text, false) {
        Ok(definition) => println!("{definition:#?}"),
        Err(error) => println!("ERROR {error:?}"),
    }
}
