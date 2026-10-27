# Winnow leaf parser patterns

Recognition lives in `crates/ironsmith-compiler-grammar/src/grammar`.
New parser code uses `winnow`. Do not add `nom`.

A leaf consumes its input and returns a typed value plus any explicit source
range its caller needs. Keep token coordinates in the original lexed stream;
do not render a recognized fragment to text and tokenize it again. Check the
complete tail at the owner boundary so an accepted prefix cannot hide residue.

A small text leaf can follow this shape:

```rust
use winnow::ascii::digit1;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

fn parse_count(input: &mut &str) -> WResult<u32> {
    digit1.parse_to().parse_next(input)
}
```

For rules text that is already tokenized, use `LexStream`, typed token primitives,
and `repeat_till` with an explicit terminating grammar. Return the captured
original token slice instead of deriving token offsets from rendered words.
Compose leaves with `alt`, `opt`, and complete sequence parsers. Keep ambiguity
visible at registry boundaries; do not hide a recognized malformed form by
turning its error into a fallback match.

Raw `.split_once`, `.find`, and token-window searches should not recognize new
language in a lowering pass. Resolve references through typed symbols and
inspect effect types through typed visitors rather than Debug strings.

Run the relevant grammar tests and the workspace boundary tests. Audit binaries
support `--fail-on-findings` for strict checks; the workspace audit-budget gate
records existing debt and rejects unreviewed changes to that inventory.
