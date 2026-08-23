use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=56 => ironsmith_verifier_shard_50_56::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_57_62::execute(operation, deck_count, input),
    }
}
