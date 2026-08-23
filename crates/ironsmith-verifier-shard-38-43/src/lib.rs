use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        38 => ironsmith_verifier_ziffle::execute_for::<38>(operation, input),
        39 => ironsmith_verifier_ziffle::execute_for::<39>(operation, input),
        40 => ironsmith_verifier_ziffle::execute_for::<40>(operation, input),
        41 => ironsmith_verifier_ziffle::execute_for::<41>(operation, input),
        42 => ironsmith_verifier_ziffle::execute_for::<42>(operation, input),
        43 => ironsmith_verifier_ziffle::execute_for::<43>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
