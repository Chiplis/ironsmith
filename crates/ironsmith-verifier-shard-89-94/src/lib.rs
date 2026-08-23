use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        89 => ironsmith_verifier_ziffle::execute_for::<89>(operation, input),
        90 => ironsmith_verifier_ziffle::execute_for::<90>(operation, input),
        91 => ironsmith_verifier_ziffle::execute_for::<91>(operation, input),
        92 => ironsmith_verifier_ziffle::execute_for::<92>(operation, input),
        93 => ironsmith_verifier_ziffle::execute_for::<93>(operation, input),
        94 => ironsmith_verifier_ziffle::execute_for::<94>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
