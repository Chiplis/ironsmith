use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        50 => ironsmith_verifier_ziffle::execute_for::<50>(operation, input),
        51 => ironsmith_verifier_ziffle::execute_for::<51>(operation, input),
        52 => ironsmith_verifier_ziffle::execute_for::<52>(operation, input),
        53 => ironsmith_verifier_ziffle::execute_for::<53>(operation, input),
        54 => ironsmith_verifier_ziffle::execute_for::<54>(operation, input),
        55 => ironsmith_verifier_ziffle::execute_for::<55>(operation, input),
        56 => ironsmith_verifier_ziffle::execute_for::<56>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
