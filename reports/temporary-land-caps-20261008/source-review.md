# Temporary additional-land caps: source-only repair

Base: d65bd6564569a38132ae107cbe209af81c87f4f6. Isolated branch: repair/temporary-land-caps.

## Exact scope and frozen evidence

- Summer Bloom: e5df4597-1647-4ac2-bdb3-a517598d1431.
- Journey of Discovery: 1c586d8a-9d1a-48a7-bb3e-9b2c0c329f8d.

`fixtures/temporary_additional_land_caps.json.fixture` contains complete official metadata rows copied from the retained, measured October 8 `reports/current-refresh-20261008/data/cards-current.json`. The only added field is `text`, built from the original mana cost, type line, and unmodified full Oracle body. In particular, Journey retains both search/reveal/hand/shuffle and additional-land modes, and Entwine {2}{G} including the reminder text. No live fetch or card-name recognition is used.

## Repair

The temporary permission parser consumed every token between `play` and `additional lands this turn` as a numeric value. `up to two/three` is a permission ceiling, not a variable chosen at resolution. The parser now consumes one optional exact `up to` phrase before capturing the typed value. Count completeness, mandatory count, exact turn duration and sentence exhaustion remain enforced. The downstream `count_tokens` contains just the cap, so the existing permission helper and lowering remain unchanged.

The existing statement-recognition and leading-may paths already recognize this typed permission and avoid a MayEffect wrapper. Lowering retains the implicit controller as PlayerFilter::You and Until::EndOfTurn. Runtime AdditionalLandPlaysEffect registers a player-specific restriction, recomputes allowance, and does not reset lands_played_this_turn. Actual land plays remain optional special actions subject to active-player, priority, timing, and ordinary restrictions.

Journey search lowering uses an optional ChooseObjectsEffect with a basic-land filter, exact owner/library binding, a 0..2 cap and public reveal, then tagged moves to hand and shuffle. Mode order remains search first, permission second. Existing paid Entwine announcement selects all modes in authored order; no new alternative cost or mode machinery is introduced.

## Authored gates (all unrun)

- Grammar unit gates: optional caps of two and three, existing singular/article and bare numeric forms; malformed/missing/repeated cap introducers, foreign numeric tokens, recurring duration, trailing commands and global actor near-matches must not be consumed by this bounded grammar.
- Independent direct/runtime and artifact encode/decode/materialization gates: both complete frozen bodies, metadata, zero parse loss, no unimplemented content, exact cap/controller/duration and absence of a resolution-time MayEffect.
- Journey structural gates: exact optional Entwine cost, two casting-time modes, search filter/count/reveal, hand destination followed by shuffle, and no cross-contamination of the two modes.
- Strict baker gate: complete metadata-inclusive text, zero parse loss, artifact validation and materialization, no unimplemented content, no fabricated semantic score.
- Runtime gates through both compilation routes: Summer Bloom used-before counts of zero or one, zero through three actual optional plays, exact cap, cleanup without resetting used counts, two stacked grants, off-turn rejection and over-cap rejection.
- Journey full announcement/resolution gates: either mode or paid Entwine, exact three/six total mana payment, mode order, zero/one/two searched basic lands, public reveal, hand counts, nonbasic/opponent-library exclusion, exact land allowance, over-cap rejection and cleanup.
- Malformed full bodies reject through both direct and artifact routes.

## Holds and boundary

No builds, tests, probes, corpus runs, code generation, formatters, or remote writes were run. These are source-authored contracts, not passing evidence. Measured recoveries: zero. Both exact IDs remain uncredited until authorized execution of the complete gates and a fresh exact-ID comparison. No global `each player` static permission is claimed: Ghirapur Orrery, Rites of Flourishing and Storm Cauldron remain held. Nahiri's Lithoforming remains held for its enclosing replacement grammar. No collateral family or supplemental-face claim is made.

The inherited artifact15 descriptor was not rewritten. This is a new source surface beyond its inherited source boundary. Later exact-source boundary/admission work and executable verification are required before release or support promotion. No artifact schema change is introduced by this grammar-only production edit.

## Follow-up: exact reveal and shuffle runtime witnesses

Independent source review of c33c973f identified that maximum reveal/hand counts plus structural shuffle presence did not independently witness full Journey execution. The follow-up strengthens the existing full-announcement Journey matrix without changing production code or Summer Bloom controls:

- The decision maker records the exact chosen object IDs and stable card identities. Public reveal callbacks must contain exactly those IDs, in selection order, for both players. At every reveal callback, all selected objects must still belong to the controller and remain in the library; the controller's hand is still empty and no LibraryShuffle operation has occurred. Empty selection produces no public card-view callback.
- Following resolution, the hand contains exactly the chosen stable card identities. Every selected card is in hand; every unselected original library object remains in the library, with the opponent's ordered library unchanged.
- Immediately before resolution, the test fixes the match RNG seed and clones the pre-resolution game. The reference clone receives the exact ordered library remaining after the selected IDs are removed, and invokes the native shuffle for the controller. This predicts the exact permutation using the supported seeded engine owner, without assuming shuffling must change the order.
- The actual game's public crypto-audit API must report exactly one typed LibraryShuffle operation for the controller in search/entwine cases and zero for the land-only mode. Its ordered input must equal the exact remaining library, its output must equal both the native seeded reference permutation and final runtime library, and its irreversible-random before/after counters must advance by exactly one. Final RNG state must agree with the clone. A zero-card search still requires this operation.

Observation APIs were inspected in engine/src/game_state.rs: `set_random_seed`, `crypto_audit_checkpoint`, `crypto_audit_operations_since`, `irreversible_random_count`, `random_seed`, and `shuffle_player_library`. The native shuffle writes `HiddenInfoOperation::LibraryShuffle` only after randomizing the specified player's library, including exact ordered input/output identities and RNG counts. Public reveal callback grouping/order was inspected in engine/src/effects/helpers.rs, and stable identity is used because movement may change object IDs. These are supported observations, not invented engine events or order-change assumptions.

All follow-up assertions remain source-authored and unrun. No build/test/probe/corpus/codegen/formatter or remote operation was performed. No measured recovery, whole-body executable pass, or artifact15 admission is claimed.
