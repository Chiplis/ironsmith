use wasm_bindgen::prelude::*;

use ironsmith_verifier_ziffle::Operation;

fn execute(operation: Operation, input: JsValue) -> Result<JsValue, JsValue> {
    ironsmith_verifier_wasm_api::execute_verifier(operation, input, ironsmith_verifier::execute)
}

#[wasm_bindgen(js_name = ziffleKeygen)]
pub fn ziffle_keygen(input: JsValue) -> Result<JsValue, JsValue> {
    ironsmith_verifier_wasm_api::execute_keygen(input)
}

#[wasm_bindgen(js_name = ziffleBuildShuffleStep)]
pub fn build_shuffle_step(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::BuildShuffleStep, input)
}

#[wasm_bindgen(js_name = ziffleVerifyShuffle)]
pub fn verify_shuffle(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::VerifyShuffle, input)
}

#[wasm_bindgen(js_name = ziffleBuildRevealToken)]
pub fn build_reveal_token(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::BuildRevealToken, input)
}

#[wasm_bindgen(js_name = ziffleBuildRevealTokens)]
pub fn build_reveal_tokens(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::BuildRevealTokens, input)
}

#[wasm_bindgen(js_name = ziffleRevealCard)]
pub fn reveal_card(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::RevealCard, input)
}

#[wasm_bindgen(js_name = ziffleRevealCards)]
pub fn reveal_cards(input: JsValue) -> Result<JsValue, JsValue> {
    execute(Operation::RevealCards, input)
}
