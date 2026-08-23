use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        8 => ironsmith_verifier_ziffle::execute_for::<8>(operation, input),
        9 => ironsmith_verifier_ziffle::execute_for::<9>(operation, input),
        10 => ironsmith_verifier_ziffle::execute_for::<10>(operation, input),
        11 => ironsmith_verifier_ziffle::execute_for::<11>(operation, input),
        12 => ironsmith_verifier_ziffle::execute_for::<12>(operation, input),
        13 => ironsmith_verifier_ziffle::execute_for::<13>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
