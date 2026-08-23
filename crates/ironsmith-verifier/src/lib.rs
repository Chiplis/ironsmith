//! Stable byte-oriented facade over independently code-generated Ziffle shards.

pub use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(operation: Operation, input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    if operation == Operation::Keygen {
        return ironsmith_verifier_ziffle::execute_keygen(input);
    }

    let deck_count = ironsmith_verifier_ziffle::input_deck_count(input)?;
    match deck_count {
        2..=13 => ironsmith_verifier_shard_02_13::execute(operation, deck_count, input),
        14..=25 => ironsmith_verifier_shard_14_25::execute(operation, deck_count, input),
        26..=37 => ironsmith_verifier_shard_26_37::execute(operation, deck_count, input),
        38..=49 => ironsmith_verifier_shard_38_49::execute(operation, deck_count, input),
        50..=62 => ironsmith_verifier_shard_50_62::execute(operation, deck_count, input),
        63..=75 => ironsmith_verifier_shard_63_75::execute(operation, deck_count, input),
        76..=88 => ironsmith_verifier_shard_76_88::execute(operation, deck_count, input),
        89..=100 => ironsmith_verifier_shard_89_100::execute(operation, deck_count, input),
        other => Err(ironsmith_verifier_ziffle::unsupported_deck_count(other)),
    }
}
