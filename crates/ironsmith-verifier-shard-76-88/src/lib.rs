use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=82 => ironsmith_verifier_shard_76_82::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_83_88::execute(operation, deck_count, input),
    }
}
