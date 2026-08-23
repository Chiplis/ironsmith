use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        70 => ironsmith_verifier_ziffle::execute_for::<70>(operation, input),
        71 => ironsmith_verifier_ziffle::execute_for::<71>(operation, input),
        72 => ironsmith_verifier_ziffle::execute_for::<72>(operation, input),
        73 => ironsmith_verifier_ziffle::execute_for::<73>(operation, input),
        74 => ironsmith_verifier_ziffle::execute_for::<74>(operation, input),
        75 => ironsmith_verifier_ziffle::execute_for::<75>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
