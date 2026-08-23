use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        57 => ironsmith_verifier_ziffle::execute_for::<57>(operation, input),
        58 => ironsmith_verifier_ziffle::execute_for::<58>(operation, input),
        59 => ironsmith_verifier_ziffle::execute_for::<59>(operation, input),
        60 => ironsmith_verifier_ziffle::execute_for::<60>(operation, input),
        61 => ironsmith_verifier_ziffle::execute_for::<61>(operation, input),
        62 => ironsmith_verifier_ziffle::execute_for::<62>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
