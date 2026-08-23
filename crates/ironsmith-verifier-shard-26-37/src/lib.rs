use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=31 => ironsmith_verifier_shard_26_31::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_32_37::execute(operation, deck_count, input),
    }
}
