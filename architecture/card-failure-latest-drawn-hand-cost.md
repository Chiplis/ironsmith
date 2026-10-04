# Exact latest-drawn hand cost

UNVALIDATED source proposal: Jandor's Ring, frozen oracle ID `737db899-dcdb-48f9-8d30-cbbce3ae3434`. No build, compilation, or tests executed.

A typed optional `ObjectFilter::last_drawn_this_turn` player predicate reads the last nonempty actual `CardsDrawnEvent` for the payer. Its recorded object identity is historical and is not rewritten. Matching requires a live exact object ID; the Hand/owner cost filter then requires that same incarnation to remain in the payer's hand. A later replaced draw cannot create a phantom latest card, and a departed latest card cannot make an older draw eligible or make a later return of that physical card eligible.

The grammar lowers the exact latest-draw discard phrase through the existing typed discard cost. No card-name special case, stable-ID follow, extra draw event, or cost bypass is introduced. Existing Public selection and transaction disclosure safeguards apply unchanged. The new optional serialized field defaults off; player/tag/target visitors and turn-context cache sensitivity include it.

Authored regressions use the exact full card and typed artifact round-trip, real draw and activation/stack resolution, another player's later draw, a real empty-library draw replacement, arbitrary hand arrivals, leave/return incarnations, and real turn cleanup. Deferred command: `cargo test -p ironsmith-compiler-runtime --test latest_drawn_hand_cost -- --nocapture`.
