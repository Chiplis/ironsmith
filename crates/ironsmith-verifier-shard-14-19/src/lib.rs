use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        14 => ironsmith_verifier_ziffle::execute_for::<14>(operation, input),
        15 => ironsmith_verifier_ziffle::execute_for::<15>(operation, input),
        16 => ironsmith_verifier_ziffle::execute_for::<16>(operation, input),
        17 => ironsmith_verifier_ziffle::execute_for::<17>(operation, input),
        18 => ironsmith_verifier_ziffle::execute_for::<18>(operation, input),
        19 => ironsmith_verifier_ziffle::execute_for::<19>(operation, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
