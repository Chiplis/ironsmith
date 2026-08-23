use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(
    operation: Operation,
    deck_count: usize,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match deck_count {
        ..=43 => ironsmith_verifier_shard_38_43::execute(operation, deck_count, input),
        _ => ironsmith_verifier_shard_44_49::execute(operation, deck_count, input),
    }
}
