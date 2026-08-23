use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        32 => ironsmith_verifier_ziffle::execute_for::<32>(operation, input),
        33 => ironsmith_verifier_ziffle::execute_for::<33>(operation, input),
        34 => ironsmith_verifier_ziffle::execute_for::<34>(operation, input),
        35 => ironsmith_verifier_ziffle::execute_for::<35>(operation, input),
        36 => ironsmith_verifier_ziffle::execute_for::<36>(operation, input),
        37 => ironsmith_verifier_ziffle::execute_for::<37>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
