//! Strict compiler compatibility, without artifact encoding, game state, or card databases.
use ironsmith_compiler::{CardDefinitionBuilder, CompilePolicy, CompilerFacade, ids::CardId};
use ironsmith_compiler_api::{CardTextError, parse_loss};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct Input {
    name: String,
    /// The compiler's metadata block followed by unmodified Oracle rules text.
    text: String,
}

fn check(input: &str) -> Value {
    let parsed = serde_json::from_str::<Input>(input);
    let Ok(input) = parsed else {
        return json!({"status":"invalid_input","parse_complete":false,"error":"Expected name and text strings."});
    };
    if input.name.trim().is_empty() || input.name.len() > 800 || input.text.len() > 24000 {
        return json!({"status":"invalid_input","parse_complete":false,"error":"Invalid name or oversized compiler input."});
    }
    let (result, loss) = parse_loss::capture(|| {
        CompilerFacade::new().compile_definition(
            CardDefinitionBuilder::new(CardId::from_raw(1), input.name),
            input.text,
            CompilePolicy {
                allow_unsupported: false,
            },
        )
    });
    let warnings: Vec<_> = loss
        .diagnostics()
        .iter()
        .map(|d| json!({"code":d.code,"message":d.message}))
        .collect();
    match result {
        Ok(_) => json!({"status":if loss.is_lossy(){"needs_review"}else{"accepted"},
            "parse_complete":!loss.is_lossy(),"warnings":warnings}),
        Err(error) => {
            let status = match &error {
                CardTextError::UnsupportedLine(_) => "unsupported",
                CardTextError::ParseError(_) => "rejected",
                CardTextError::InvariantViolation(_) => "parser_error",
            };
            json!({"status":status,"parse_complete":false,"error":error.to_string(),"warnings":warnings})
        }
    }
}

/// JSON in/out keeps the worker boundary small; a pass is compiler support, not a rules proof.
#[wasm_bindgen(js_name = checkOracleCard)]
pub fn check_oracle_card(input: &str) -> String {
    check(input).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_input() {
        assert_eq!(check("{}")["status"], "invalid_input");
    }
    #[test]
    fn checks_the_entire_text() {
        let valid = json!({"name":"Probe","text":"Type: Sorcery\nDraw a card."});
        assert_eq!(check(&valid.to_string())["parse_complete"], true);
        let invalid = json!({"name":"Probe","text":"Type: Sorcery\nDraw a card. Gobble the moon."});
        assert_eq!(check(&invalid.to_string())["parse_complete"], false);
    }
}
