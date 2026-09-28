//! Stable byte-oriented facade over the Ziffle verifier.

pub use ironsmith_verifier_ziffle::{Operation, VerifierError};

pub fn execute(operation: Operation, input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    if operation == Operation::Keygen {
        return ironsmith_verifier_ziffle::execute_keygen(input);
    }

    ironsmith_verifier_ziffle::execute_with_input_chain(operation, input, dispatch)
}

fn dispatch(operation: Operation, input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    let deck_count = ironsmith_verifier_ziffle::input_deck_count(input)?;
    ironsmith_verifier_ziffle::execute(operation, deck_count, input)
}
