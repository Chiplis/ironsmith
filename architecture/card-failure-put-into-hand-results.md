# Exact hand-arrival result conditions

Implementation-first source proposal; no compiler, tests, build or corpus replay has run.

Frozen candidates: Blossom Prancer, Contagious Vorrac, Pulsar Squadron Ace, and Rosecot Knight. Each complete ETB body looks at a bounded set, optionally reveals a matching card and moves it to hand, randomizes the remainder onto the library bottom, then grants a consolation only if the hand move did not happen. Printed reach/vigilance remain part of the full-card regression scope.

The complete `you [didn't/did not] put a card into your hand this way` reader emits a typed `PriorEffectAction::PutIntoHand`, including the actor and negation. Reference resolution binds the gate to the exact hand-move producer across the intervening remainder instruction. It does not test selection, revelation, generic acceptance, or the last ambient result.

The zone owner records `CardsPutIntoHand` only from successful original arrivals into Hand, before authored work or deferred replacement additions. It records the recipient and non-token card evidence. The existing instruction-result boundary excludes independent replacement programs, and composition aggregates original participant results. A prevented or redirected-to-exile move produces no hand arrival. An original hand arrival remains true if an additional program subsequently moves that card away. Pending/error rollback uses the existing checked move owner.

The context-aware result evaluator matches the captured hand recipient to the resolving actor. No live-zone lookup reconstructs history. Existing wire refusal/native branches/full-transcript replay remain the completeness boundary for runtime programs.

Authored, unrun coverage: whole direct and retained-artifact bodies with success/decline/no-hit, complete remainders, life/proliferate/counter consequences and keywords; native prevention/redirection/independent replacement move/later departure and pending rollback; complete grammar negatives and exact producer binding. These are candidate source proposals pending independent source review and eventual runtime validation.

Independent bounded source review is clear through `86f7f33a6`; the generic arrival fact now requires a physical Card, excluding spell copies and emblems. The complete body scenarios additionally retain an unviewed bottom sentinel to prove remainder placement. Four identities are proposed/unvalidated in stage52, with all execution still deferred.
