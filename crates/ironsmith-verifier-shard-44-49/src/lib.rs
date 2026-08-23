use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        44 => ironsmith_verifier_ziffle::execute_for::<44>(operation, input),
        45 => ironsmith_verifier_ziffle::execute_for::<45>(operation, input),
        46 => ironsmith_verifier_ziffle::execute_for::<46>(operation, input),
        47 => ironsmith_verifier_ziffle::execute_for::<47>(operation, input),
        48 => ironsmith_verifier_ziffle::execute_for::<48>(operation, input),
        49 => ironsmith_verifier_ziffle::execute_for::<49>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
