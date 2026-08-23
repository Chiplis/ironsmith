//! Shared byte/JavaScript boundary for independently linked verifier shards.

use ironsmith_verifier_ziffle::{Operation, VerifierError, input_deck_count};
use wasm_bindgen::prelude::*;

pub type ShardExecutor = fn(Operation, usize, &[u8]) -> Result<Vec<u8>, VerifierError>;
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

pub fn execute_shard(
    operation: Operation,
    input: JsValue,
    executor: ShardExecutor,
) -> Result<JsValue, JsValue> {
    let json = input_json(input)?;
    let deck_count =
        input_deck_count(json.as_bytes()).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let output = executor(operation, deck_count, json.as_bytes())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    output_json(output)
}

pub fn execute_keygen(input: JsValue) -> Result<JsValue, JsValue> {
    let json = input_json(input)?;
    let output = ironsmith_verifier_ziffle::execute_keygen(json.as_bytes())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    output_json(output)
}
