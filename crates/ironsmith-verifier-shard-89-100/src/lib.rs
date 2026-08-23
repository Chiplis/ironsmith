use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=94 => ironsmith_verifier_shard_89_94::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_95_100::execute(operation, deck_count, input),
    }
}
