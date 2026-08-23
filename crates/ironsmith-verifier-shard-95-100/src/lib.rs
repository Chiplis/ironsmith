use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        95 => ironsmith_verifier_ziffle::execute_for::<95>(operation, input),
        96 => ironsmith_verifier_ziffle::execute_for::<96>(operation, input),
        97 => ironsmith_verifier_ziffle::execute_for::<97>(operation, input),
        98 => ironsmith_verifier_ziffle::execute_for::<98>(operation, input),
        99 => ironsmith_verifier_ziffle::execute_for::<99>(operation, input),
        100 => ironsmith_verifier_ziffle::execute_for::<100>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
