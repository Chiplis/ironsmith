use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        63 => ironsmith_verifier_ziffle::execute_for::<63>(operation, input),
        64 => ironsmith_verifier_ziffle::execute_for::<64>(operation, input),
        65 => ironsmith_verifier_ziffle::execute_for::<65>(operation, input),
        66 => ironsmith_verifier_ziffle::execute_for::<66>(operation, input),
        67 => ironsmith_verifier_ziffle::execute_for::<67>(operation, input),
        68 => ironsmith_verifier_ziffle::execute_for::<68>(operation, input),
        69 => ironsmith_verifier_ziffle::execute_for::<69>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
