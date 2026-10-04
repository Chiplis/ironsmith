# SNC exiled-source land mana grants

Status: **UNVALIDATED source implementation and authored regressions**. No compiler, build, or tests ran. Exact frozen identities and payloads are in `fixtures/snc_exiled_land_mana_grants.json.fixture`.

The five shared mechanic candidates are Glamorous Outlaw, Masked Bandits, Rakish Revelers, Shattered Seraph, and Spara's Adjudicators. These were held partial because their private-hand activations shared the payment-disclosure transaction/error boundary. Stage40 now admits their mechanic source proposals after reviewing the shared transaction and exact self-exile behavior. Runtime validation is deferred; Rakish Revelers separately retains the token-cap gap.

## Typed implementation

- A new appended `Until::ObjectIsCast` carries an exact materialized incarnation and origin zone. An ordinary zone exit is not its ending event. Existing enum positions and payloads remain intact.
- The shared self-exile result tag records precisely the incarnation produced when the source exiles itself as a cost or instruction. It is independent of the source's larger linked-exile collection and never follows stable-card identity to a later incarnation. The grant and cast permission share that precise result.
- Final casting, after costs are paid, records the existing announcement-time `cast_origin_snapshot`'s object and zone. Merely proposing a cast does not record it. The durable facts survive turns and later spell resolution; sync checkpoint export/import carries them with an absent-field default for older payloads.
- Continuous materialization refuses missing references and does not start a grant if its ending cast already happened. The mana ability remains available during its own card's casting payment and disappears when casting completes, before spell resolution.
- The existing `GrantPlayTagged` permission is reused with the activation controller as grantee. Its open-ended exile form now requires the exact tagged exile incarnation to remain in exile when the permission resolves. It cannot grant from a different zone or follow a leave/re-exile cycle.
- Quoted mana grants, the cast-event duration, and the independent self-card permission are lowered through ordinary compositional grammar. There are no production card-name recognizers.

Primary rules evidence: [Wizards' Streets of New Capenna release notes](https://magic.wizards.com/en/news/feature/streets-new-capenna-release-notes-2022-04-20). The Glamorous Outlaw notes explicitly permit using the granted mana while casting it and retain the land ability when it leaves exile without being cast.

## Authored, unrun checks

`crates/ironsmith-compiler-runtime/tests/snc_exiled_land_mana_grants.rs` covers all five full frozen programs directly and through typed artifact JSON round trips. Live hand activations pay the actual self-exile cost and target a real land. Each card then uses the granted land mana to pay its actual exile cast. Other cases cover opponent-controlled target lands and mana ownership, leaving/reentering exile, source departure before ability resolution, unrelated later-incarnation casting, illegal land targets fizzing the whole ability, canceled casting payment, true turn rollover, and old/new duration serialization.

`snc_payment_disclosure_undo_tests.rs` exercises the five exact hand-to-exile activations through the live WASM state machine and checks completed-action Undo before ability resolution. The small Undo extension recognizes cost-caused Hand-to-public-zone moves whose resulting object is face up. Existing mana-only Undo controls remain in the adjacent suite. This is a bounded completed-action guard, not whole-payment disclosure staging.

`completed_cast_origin_wire_tests` authors a native sync checkpoint round trip and absent-field compatibility control. The grammar duration test protects quote boundaries and rejects an unrelated `that card` referent.

Deferred commands:

- `cargo test -p ironsmith-compiler-runtime --test snc_exiled_land_mana_grants -- --nocapture`
- `cargo test -p ironsmith-web-session payment_disclosure_exact_snc -- --nocapture`
- `cargo test -p ironsmith-web-session snc_completed_cast_origins -- --nocapture`
- `cargo test -p ironsmith-compiler-grammar snc_quoted_mana_grant -- --nocapture`

## Required shared transaction primitive

Event history and Undo guards cannot establish confidentiality after an error restores an earlier action. The common remaining primitive for the earlier seven and this five-card cohort is an **announcement/payment transaction with a disclosure commitment boundary**, retaining exact source, payer, announced X/targets, selected incarnations, and validated cost order across its commands.

Before that boundary, availability checks, local previews, pending choices, and canceled/failed proposals must expose no previously hidden identities to other seats. Public proof requirements still exist, but their material must be staged locally or bound to an irrevocable accepted transaction, not replaced by unchecked claims.

At the first legitimate disclosure required to finish payment or choose a replacement, the transaction must become irrevocable as a whole, not just pin one decision command. All peers must retain that same transaction identity and the exact disclosed choices. A later normal replacement decision continues it. A retry must resume the same accepted transaction and choices without spending costs twice, duplicating keyword events, rerolling randomness, or changing targets/X/source. A transient transport/quorum failure cannot reopen alternate choices or restore a private-hand presentation of already published cards. Cancellation is still free before disclosure; ordinary reversible mana-only Undo remains available. An impossible or inconsistent continuation must fail closed and retain recovery evidence, rather than silently treating a rolled-back action plus already-public openings as a successful neutral command.

The engine must distinguish a legal cost modified/prevented by replacement (still paid under CR 118.11) from an invalid proposal and a genuine implementation/transport failure. Exact movement receipts and ownership/controller context must survive legitimate pending replacement prompts. Recovery checkpoints must persist both payment state and disclosure commitments; speculative probes must clone/restore them without permanently latching knowledge.

Stage40 implements and source-reviews that cross-command boundary, including corrected Cancel routing, actual decision-owner authority, invalid-input prechecks, immutable signed attempt and original timing recovery. The bounded preview redaction and Undo corrections remain prerequisites. All execution remains deferred.

### Replaced/prevented payment edge

The source-result tag also retains the exact result of a self-exile instruction whose legal cost is modified or prevented. Its snapshot comes only from that instruction's movement receipt results, falling back to the captured original when no movement occurred. It never searches for an arbitrary current stable-card incarnation. Thus the land grant is still made under CR 118.11, while the separate permission fails its actual-exile membership check. A truly missing duration reference now produces an unresolved-reference error instead of silently omitting the grant. Authored exact five-card cases cover prevention and redirection to graveyard, plus an explicit unbound-reference control.
