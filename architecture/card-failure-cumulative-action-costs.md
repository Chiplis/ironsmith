# Cumulative upkeep action costs

Status: source-only proposal for independent review. No build, test, compiler
probe, formatter, corpus run, or runtime measurement has been performed.

## Frozen scope

`fixtures/cumulative_action_costs.json.fixture` retains each complete frozen
body and its card/oracle identity from `cards-20261003.json.xz`:

| Card | Frozen oracle ID | Baseline |
| --- | --- | --- |
| Braid of Fire | 796009b3-73b5-42c0-a490-66f12be5a42e | unsupported cumulative-upkeep effect |
| Jötun Grunt | 43fbfeec-bcaf-48b8-befe-b7346fec5a3a | unsupported cumulative-upkeep effect |
| Psychic Vortex | 451e35dd-e0b5-4157-8511-bacb315e19bd | unsupported cumulative-upkeep effect; end-step body retained |

The evidence is the e8740178 snapshot, not a new corpus measurement. No central
campaign matrix or count is changed by this branch.

## Rules and typed ownership

Primary pinned reference: [Wizards Comprehensive Rules, September 25, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf).
CR 702.24a requires separate choices for every age counter before paying the
whole set; partial payment is forbidden. CR 118.5 requires acknowledgement of a
zero cost. CR 118.11 counts replacement-modified actions as payment and expressly
uses Psychic Vortex's skipped draws as its example. CR 121.2b and 121.3
distinguish draw prohibitions from an empty library. CR 401.4 assigns ordering
of cards entering a library together to that library's owner.

The grammar already supplies declarative effect AST action costs for adding
mana and drawing cards, and a typed fixed-card graveyard movement cost. The
semantic payment validator now admits the native AddMana/Draw primitives;
neither is lowered as mana expenditure. These primitives also expose native
CostExecutable validation. Existing effect model conversion and artifact codecs
carry the typed primitives. LibraryPlacementOrder appends `Owners` without
changing existing variants; lowering preserves that owner policy.

The cumulative native owner recognizes only bounded typed action programs.
Its graveyard capacity is the sum of each graveyard's complete-installment
capacity, not an aggregate card count and not one graveyard for all ages.
It retains a separate choice for each installment, rejects partial, duplicate,
mixed-graveyard or reused submissions, and moves nothing while any choice is
pending. After all group constraints are established, the ordinary native
movement owner handles the simultaneous original action, per-owner ordering,
replacement scopes, and additions. Payment success is independent of actual
drawn/moved/added counts. A successful zero payment emits one paid receipt.

There is no speculative draw or mana-effect execution to decide whether these
action costs are payable. Draw prohibitions are checked against the complete
announced cost. Actual draws retain their normal replacement decisions and
private state. The source ObjectId and its pre-payment LKI survive departure;
there is no stable-id retargeting of the paid notice. Action causes are marked
as costs, then the parent cause and mana-payment scope are restored.

All native errors and pending choices remain inside the existing full game and
execution-context checkpoint. The shared resource meter is reused. An invalid
submitted group is typed incomplete evidence, never proof of nonpayment and
never permission to sacrifice the source. Normal explicit decline or genuinely
insufficient complete groups still uses the ordinary unpaid owner.

## Authored regressions, unrun

The compiler-runtime target contains direct compile and JSON artifact round
trips for all exact full bodies, two actual upkeep resolutions, Psychic
Vortex's whole end step with and without a sacrificable land, zero/nonzero
decline, pending choices, disjoint per-age pairs from different graveyards,
per-owner bottom order, insufficient split groups, malformed choices,
prevented/Instead actions, added-program suspension/errors/resource exhaustion,
draw prohibitions and empty libraries, blinked source identity, departed source
paid LKI, and draw observer capture before newly created observers.

The web-session native regression uses the real WasmReplayDecisionMaker: the
accepted upkeep answer survives a draw replacement's later prompt; native state
rolls back while a separate prompt snapshot contains the private drawn card;
no ordinary draw becomes a public viewed-card window. The existing resolution
replay cancellation rule blocks cancelling the accepted upkeep. Replaying the
retained answer plus a decline of the replacement's optional addition completes
the payment once. No new blanket private-state rejection was added.

## Explicit limits

This proposal does not complete arbitrary private casting-cost replay or the
general cost-program planner. Existing authenticated openings, disclosure
commitments, retained native history/program transport guards, and cancellation
rules remain authoritative; no transport bypass or forged success is introduced.
The shared compound simultaneous-Instead life-payment boundary documented in
`card-failure-life-payment-trigger.md` remains a campaign gap. This work neither
suppresses that typed rejection nor claims to recover its held body.

All three identities require independent source review, then the later campaign
compile/runtime/semantic gates. Nothing in this document is an executed pass.

Independent source review cleared all three full bodies through afc0d1537.
The strict direct and artifact compilation routes are independently authored.
This is source-review admission only; every execution gate remains unrun.
