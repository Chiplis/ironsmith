use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        26 => ironsmith_verifier_ziffle::execute_for::<26>(operation, input),
        27 => ironsmith_verifier_ziffle::execute_for::<27>(operation, input),
        28 => ironsmith_verifier_ziffle::execute_for::<28>(operation, input),
        29 => ironsmith_verifier_ziffle::execute_for::<29>(operation, input),
        30 => ironsmith_verifier_ziffle::execute_for::<30>(operation, input),
        31 => ironsmith_verifier_ziffle::execute_for::<31>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
