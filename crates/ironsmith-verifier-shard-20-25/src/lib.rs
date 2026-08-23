use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        20 => ironsmith_verifier_ziffle::execute_for::<20>(operation, input),
        21 => ironsmith_verifier_ziffle::execute_for::<21>(operation, input),
        22 => ironsmith_verifier_ziffle::execute_for::<22>(operation, input),
        23 => ironsmith_verifier_ziffle::execute_for::<23>(operation, input),
        24 => ironsmith_verifier_ziffle::execute_for::<24>(operation, input),
        25 => ironsmith_verifier_ziffle::execute_for::<25>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
