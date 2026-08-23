use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        2 => ironsmith_verifier_ziffle::execute_for::<2>(operation, input),
        3 => ironsmith_verifier_ziffle::execute_for::<3>(operation, input),
        4 => ironsmith_verifier_ziffle::execute_for::<4>(operation, input),
        5 => ironsmith_verifier_ziffle::execute_for::<5>(operation, input),
        6 => ironsmith_verifier_ziffle::execute_for::<6>(operation, input),
        7 => ironsmith_verifier_ziffle::execute_for::<7>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
