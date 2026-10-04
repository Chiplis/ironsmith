# Generic X payment allocation

Status: **UNVALIDATED source prerequisite. No builds, compilation, tests or runtime probes.**
Bounded source review admits all seven proposed/unvalidated cards: Atalya, Samite Master; Consume Spirit; Crimson Hellkite; Crypt Rats; Emblazoned Golem; Drain Life; and Soul Burn. The five payment bodies cleared through `f5770d5ee`; the two capped-damage bodies cleared through `bd3ecf899 → 99dd13121 → 862f2f209`. The fixture now contains ten admitted proposals including the earlier three producer-source constraints. No execution recovery is claimed.

## Payment model

X remains generic (CR 107.4b). `ManaSpendingRestriction::OnX` constrains actual mana allocated to it, not the color that an as-though permission lets a unit pay. `XPaymentScope` preserves the original number/value of X symbols and the capacity of the ordinary generic base/additions, even when all generic pips are coalesced. Generic reductions and convoke/improvise/delve remove pips without falsely converting them into spent mana.

Given actual generic mana count G (including Assist), original X amount X, and unrestricted generic capacity O, the actual amount allocated to X may range from max(0, G - O) through min(G, X). A valid selection must also meet its actual-color and per-color constraints. Thus a generic reduction can be allocated legally to either portion; neither fixed colored pips nor generic taxes are silently relabeled as X. The default server witness is deterministic, and the player can request a different exact W/U/B/R/G allocation. That preference is validated at the actual per-pip assignment owner and is included in request/plan identity.

`ManaCost::with_pips` retains scope and records additions to the ordinary generic portion. X must be bound before its expansion; the shared expansion owner performs a single rewrite. Optional X additions are included before binding. The scope is omitted entirely on ordinary historical costs, so their JSON and hashes remain unchanged.

## Assist

Assist is a payment, not a reduction. For a constrained-X spell, the helper request carries a typed, independently priced caster continuation. The bulk assignment owner tries actual helper unit assignments until one permits completion, using the same speculative unit/provenance/life commit as real payment. A first affordable but incorrectly colored helper assignment does not prove absence. The successful helper plan locks its actual colors, and final payment revalidates source restrictions. The caster's allocation includes those real helper units, so both players share one per-color cap.

The continuation preserves source, payer, X scope, spend policy, reservations and preferences, is covered by the root hash, and cannot itself contain Assist. The helper cannot convoke/delve the caster's spell. Existing typed incomplete-query propagation remains the owner of failed speculative continuation queries.

## Authored, unrun scenarios

`constrained_x_payment` covers fixed/base/tax versus X allocation, actual versus as-though colors, selectable generic reduction allocation, stale plan rejection without mutation, announced X above five with only five actual distinct colors, helper assignment search, shared Assist color caps, and speculative state preservation. Core tests cover zero/nonmana payments and price accounting. UI draft tests preserve explicit allocation through unrelated source edits; the payment editor offers validated actual-color counts.

Deferred commands, not executed:
- `cargo test -p ironsmith-core x_payment -- --nocapture`
- `cargo test -p ironsmith-compiler-runtime --test constrained_x_payment -- --nocapture`
- `node --test web/ui/tests/payment-draft.test.js`

## Source prerequisites now represented (execution deferred)

- Activate/spell/modal X spending grammar and exact frozen artifact scenarios.
- Final cast-retained black-on-X receipt, including successful zero residual payments and Assist, with checkpoint and zone/copy semantics.
- Drain Life/Soul Burn's actual-damage gain, explicit pre-damage player-life and planeswalker-loyalty cap, and correct creature-toughness timing.
- Whole-card positive/negative/cancel/real-resolution scenarios for Atalya, Consume Spirit, Crimson Hellkite, Crypt Rats, Drain Life, Emblazoned Golem and Soul Burn.

## Subsequent source checkpoints

`de6444878` retains actual X-color payment in native objects, cast-payment checkpoints and historical snapshots, resets it for spell copies, and appends `Value::ManaSpentOnX(Color)` for Soul Burn. An authored actual-payment/copy/departure-LKI scenario and retained-schema cases cover this carrier; execution remains deferred.

The following grammar/body source proposal represents Atalya, Samite Master; Consume Spirit; Crimson Hellkite; Crypt Rats; and Emblazoned Golem. Their full frozen definitions and typed artifacts have authored strict cases, plus real announcement/payment/resolution scenarios, Atalya's two modes/prevention/cancellation, Consume Spirit's gain despite prevention, Rats' simultaneous recipients, and Golem's declined kicker or announced X=7 with a generic reduction. A mixed white/red modal rider is explicitly rejected rather than strengthened into one global rule. The shared rule is hoisted only when every mode agrees. At this checkpoint those five remained pending review; the later disposition below admits them.

At that checkpoint Drain Life and Soul Burn remained excluded; their complete body closure is recorded below.

### Unknown historical X payment

The retained `mana_spent_on_x` option distinguishes missing legacy evidence from a known zero. Newly created objects and genuine spell copies start with an explicit zero receipt; successful scoped payment replaces it with the actual allocation. Importing an older paid-spell or snapshot payload without the field leaves it unknown, and `ManaSpentOnX` returns a typed unresolved-value error rather than inventing zero. The exact live carrier takes precedence over an older source snapshot. Authored cases cover actual, explicit zero, unknown legacy and exact departure LKI separately.

### Review corrections: component composition and incarnation lifetime

The activation total-price owner merges separately priced mana components. Rule-only inheritance was insufficient after a right-hand X component had already expanded to generic pips. `ManaCost::combined_with` now composes both scopes' original X and ordinary-generic capacities, including a bound right operand, and rejects incompatible bound scopes. Runtime, compiler and WASM aggregation use the same primitive. An authored wrong-color X announcement covers Atalya, Crimson Hellkite and Crypt Rats through the real payment state machine.

Ordinary zone changes reset the new incarnation's X-payment receipt to known zero, while Stack→Battlefield keeps the cast payment and the departing object's historical snapshot retains its old receipt. An authored paid→graveyard→hand→fully-reduced recast scenario verifies that announced X can stay positive without inheriting the first cast's actual black allocation.

## Bounded source-review disposition

The five-body chain is `fc570fc01 → de6444878 → fdab4a493 → 7707d568a → 1d9345fec → f89872925 → f5770d5ee`. Review found and corrected missing historical evidence, right-hand bound-price composition, incarnation reset, scalar clamping and the invalid-choice fixture boundary. Source clearance does not validate execution. The exact five identities were admitted by the first fixture-only rollup; the later two-body rollup is separate.

## Drain Life / Soul Burn body proposal (source-reviewed)

The appended `DamageDealtCappedByRecipient` metric binds to the exact preceding damage instruction through the existing prior-effect action binder. Its damage total comes from the original instruction result, excluding independently executed replacement additions. `DamageRecipientBefore` captures each original recipient before damage's consequences, independently of redirection. The singular cap requires one exact recipient; it never guesses among multiple recipients or an absent receipt.

The printed pre-damage life and loyalty limits use captured scalars. The creature limit uses current exact-incarnation toughness when the gain instruction applies, with checked continuous discovery and exact departure LKI if gone. It does not chase a stable ID after blink. This follows the frozen Oracle wording's explicit timing and CR 608.2h–i; wither/infect's counter consequences precede the later gain instruction (120.3d). No Battle-defense limit is invented. Soul Burn additionally takes the minimum with `ManaSpentOnX(Black)`, preserving the actual chosen cost allocation.

Authored, unrun additions include exact full artifacts for both cards, real paid casts against low-life players/low-loyalty planeswalkers/creatures, fixed black versus black-on-X, two player-selected reduction allocations producing different gains, full prevention, target blink, actual wither/current toughness and exact LKI, original-target redirection, and exclusion of auxiliary replacement-added damage. Bounded review accepted both identities through `862f2f209`; the final fixture rollup admits them as proposed/unvalidated.

Primary rule reference: the campaign's official 2026-09-25 Comprehensive Rules, also distributed from https://magic.wizards.com/en/rules .

### Final review correction and authored target

`99dd13121` and `862f2f209` make the creature cap respect the current/last-known card type: known battlefield noncreatures have zero toughness for this query; known creatures lacking toughness evidence produce a typed unresolved-value error. Paired damage-added type-removal scenarios cover both a remaining object and its exact departed incarnation. `0f626eafd` corrects public builder paths in the authored payment/price tests.

`constrained_x_payment` now contains 25 authored runtime scenarios. Core, grammar and UI cases accompany it. None has been run under the implementation-first workflow. The original frozen oracle IDs remain unchanged in the fixture.

### Combined numeric-range integration

The retention scalar owner (`a998d5dad`, integrated centrally as `acba3f13b`) makes an infallible characteristics lookup return no frame on numeric overflow. Original damage-recipient capture and the later live toughness cap now consume `try_current_characteristics` as one authoritative Result-bearing frame, distinguishing known noncreatures from unavailable/incomplete calculation. An authored exact Drain Life scenario adds a non-mana P/T overflow during damage completion and requires typed failure plus whole-resolution rollback. No execution was performed.
