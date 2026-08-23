use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        76 => ironsmith_verifier_ziffle::execute_for::<76>(operation, input),
        77 => ironsmith_verifier_ziffle::execute_for::<77>(operation, input),
        78 => ironsmith_verifier_ziffle::execute_for::<78>(operation, input),
        79 => ironsmith_verifier_ziffle::execute_for::<79>(operation, input),
        80 => ironsmith_verifier_ziffle::execute_for::<80>(operation, input),
        81 => ironsmith_verifier_ziffle::execute_for::<81>(operation, input),
        82 => ironsmith_verifier_ziffle::execute_for::<82>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
