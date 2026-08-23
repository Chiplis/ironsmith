use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        83 => ironsmith_verifier_ziffle::execute_for::<83>(operation, input),
        84 => ironsmith_verifier_ziffle::execute_for::<84>(operation, input),
        85 => ironsmith_verifier_ziffle::execute_for::<85>(operation, input),
        86 => ironsmith_verifier_ziffle::execute_for::<86>(operation, input),
        87 => ironsmith_verifier_ziffle::execute_for::<87>(operation, input),
        88 => ironsmith_verifier_ziffle::execute_for::<88>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
