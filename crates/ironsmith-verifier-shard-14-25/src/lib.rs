use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=19 => ironsmith_verifier_shard_14_19::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_20_25::execute(operation, deck_count, input),
    }
}
