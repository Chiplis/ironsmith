//! Shared byte/JavaScript boundary for the verifier.

use ironsmith_verifier_ziffle::{Operation, VerifierError};
use wasm_bindgen::prelude::*;

pub type VerifierExecutor = fn(Operation, &[u8]) -> Result<Vec<u8>, VerifierError>;

fn input_json(input: JsValue) -> Result<String, JsValue> {
    js_sys::JSON::stringify(&input)
        .map_err(|_| JsValue::from_str("failed to stringify verifier input"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("verifier input is not JSON"))
}

fn output_json(output: Vec<u8>) -> Result<JsValue, JsValue> {
    let output = std::str::from_utf8(&output)
        .map_err(|_| JsValue::from_str("verifier output is not UTF-8 JSON"))?;
    js_sys::JSON::parse(output).map_err(|_| JsValue::from_str("failed to parse verifier output"))
}

pub fn execute_verifier(
    operation: Operation,
    input: JsValue,
    executor: VerifierExecutor,
) -> Result<JsValue, JsValue> {
    let json = input_json(input)?;
    let output = executor(operation, json.as_bytes())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    output_json(output)
}

pub fn execute_keygen(input: JsValue) -> Result<JsValue, JsValue> {
    let json = input_json(input)?;
    let output = ironsmith_verifier_ziffle::execute_keygen(json.as_bytes())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    output_json(output)
}
