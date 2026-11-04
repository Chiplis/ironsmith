//! Scratch probe (temporary, p6): compile `<name>` with oracle text from stdin.
//! KW=a,b adds custom keywords; TRACE=1 prints the parse trace on success too.
use std::io::Read;
fn main() {
    let name = std::env::args().nth(1).unwrap_or_else(|| "Probe".to_string());
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();
    let keywords = std::env::var("KW").unwrap_or_default();
    let (result, trace) = ironsmith_compiler::parse_trace::capture(|| {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
                let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    &name,
                );
                for keyword in keywords.split(',').filter(|k| !k.is_empty()) {
                    builder = builder.with_custom_keyword(keyword);
                }
                ironsmith_compiler::CompilerFacade::new().compile_definition(
                    builder,
                    text.clone(),
                    ironsmith_compiler::CompilePolicy { allow_unsupported: false },
                )
            })
        });
        (result, loss)
    });
    let (result, loss) = result;
    if loss.is_lossy() {
        println!("LOSSY {}", loss.reasons_text());
    }
    match result {
        Ok(compiled) => {
            println!("{:#?}", compiled.definition);
            if std::env::var("TRACE").is_ok() {
                println!("{}", trace.render());
            }
        }
        Err(error) => println!("ERROR {error:?}\n{}", trace.render()),
    }
}
