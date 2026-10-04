# Passive damage recipients and threshold occurrences

UNVALIDATED source work. No build, compiler, CLI probe or test was run.

Six exact stack07 identities are retained in
`fixtures/passive_damage_recipients.json.fixture`: Innocent Bystander, Pain
Magnification, Risona, Asari Commander, Smaug the Impenetrable, Phyrexian Totem,
and Vengeful Pharaoh. All six are complete source proposals after bounded
independent review of the shared result, recipient, history and arithmetic
boundaries below. This is proposed source coverage only, never a measured compile
or gameplay recovery. No build, CLI probe or test was run.

## Implemented source boundary

The typed DamageReceived AST lowers to the existing IsDealtDamage model. New
minimum and single-source fields default to the old behavior on JSON restoration.
The serialized variant ordinal is unchanged. Player and object recipients stay
separate, including the player/planeswalker alternative. Generic self nouns keep
their source identity. Existing plain heads and qualified damaging-source readers
keep their prior grammar owners.

Completed trigger-queue batches attach recipient and (source, recipient) totals
from the actual replacement-adjusted assignments. This is an occurrence total,
never marked damage or a turn-history total. Combat and noncombat contributions
remain distinguishable. Thresholds are checked before matching any assignment;
body amounts still sum each original contribution only once. A new native
DamageSourceTarget grouping key keeps distinct sources and recipients separate.
Both the ordinary and incremental combat paths retain the same completed receipt.

Recipient filters prefer exact captured target characteristics. Damage receipts
can now be captured before combat replacement/prevention-added programs and
lifelink completions remove their observers. The later publication reuses the
marked receipt, preserving single history/event publication and rollback.

CR 603.2c and 120.4b establish occurrence and damage-trigger timing; CR 120.9
keeps named-source amounts separate. The official [MKM release notes](https://magic.wizards.com/en/news/feature/murders-at-karlov-manor-release-notes)
confirm that Bystander requires its threshold in one damage occurrence, rather
than several separate events. Pharaoh has one occurrence per damaged recipient,
so simultaneous damage to its owner and their planeswalker triggers twice; its
intervening graveyard condition and exact source identity govern both resolutions.

## Initial shared prerequisite, closed by the receipts below

The original `rules/damage.rs::apply_processed_damage_assignment_with_scope`
immediately executed life-loss and infect/wither counter replacement programs while applying
individual assignments. Such a program can remove a damage observer or a later
recipient before all simultaneous original damage consequences commit. The
unignored public scenario
`damage_result_life_loss_additions_wait_for_original_damage_trigger_capture`
pins this with Pain Magnification. Closing the matcher alone cannot close this
card family.

The implemented prerequisite introduces prepared damage consequence proposals,
original-commit receipts and deferred completion, reusing the existing life and
counter primitive owners. Damage observers must be captured at CR 120.4b before
those consequence programs; all simultaneous originals must remain simultaneous.
The multi-source and existing single-source owners consume the same API without
serializing source-by-source completion. Native savepoints and fail-closed wire
program/history boundaries must retain this contract.

## Authored scenarios (unrun)

Strict full-card direct/artifact gates; old/new typed wire shapes; grammar
negatives; same-recipient 1+2 versus separate 1 then 2; per-source 3+3 and one
source hitting multiple opponents; actual combat assignment; prevention and
zero-damage negatives; Clue activation/draw; Smaug Treasure amounts; both Risona
counter clauses; Totem mana, paid animation, trample, sacrifice and intervening
condition; Pharaoh graveyard timing, multiple recipients, illegal targets and
extra zone moves; before-additions observers, pause/error rollback and replay.

## Shared result API and combat owner follow-up

`e159e5339` adds `prepare_processed_damage_assignment`,
`commit_prepared_damage_original`, and `complete_damage_original`. Counter
results reuse the same permanent/player counter executors, with a staged owner
that retains pre-placement restrictions, exact replacement targets and the
replacement source's snapshot. The legacy immediate wrappers delegate to it.
The native API scenarios preserve life loss separately from added programs,
commit later wither recipients before an earlier addition exiles them, and keep
damage dealt when infect's poison-counter result is prevented.

All three combat paths now use one result owner: prepare every assignment and
toxic result; capture the complete damage occurrence at CR 120.4b; commit every
original life/counter result; capture those receipts; then complete lifelink,
result-added programs, damage-added programs and prevention follow-ups. The
whole combat-step savepoint owns failure/pending rollback. Returned combat
receipts carry matching proof, so later publication does not duplicate damage
history or re-match changed observers. General and unblocked paths share this
owner instead of maintaining source-by-source consequence loops.

The normal one-source and new multi-source effect producers consume this
same staged API through the reviewed quantity-lane consumer integration. The life-loss, wither and toxic
program observer scenarios remain unignored in the public target.

## Initial bounded arithmetic finding, closed below

The initial native damage replacement Multiply/Add used saturating u32 arithmetic,
and several damage occurrence/result totals saturated or entered i32 values
without complete checks. This is a concrete representation gap, not a game rule or a permitted
cap. Relevant owners include `events/damage/damage_event.rs`, replacement-action
application in `events/processing/mod.rs`, `effects/damage/deal_damage.rs`, and
the receipt sums in `events/damage/receipt_amounts.rs` / the combat result owner.

The existing eight damage-multiplier proposals stay runtime-partial until the
representation is extended or the entire owning transaction returns an explicit
representation error without partial damage, receipts or consumed replacements.
Do not count a saturated result, suppress its diagnostic, or silently clamp it.
This ledger is deliberately limited to that identified arithmetic family.

### Common completion frame

`freeze_damage_original` freezes each receipt exactly once. A simultaneous owner
freezes all damage, toxic and lifelink completions after all original results
commit, then runs additions. Later completion cannot recapture a board already
changed by an earlier sibling. An authored native two-receipt regression pins
the common life-total frame and single freeze invocation. No execution.

Lifelink preparation now shares the pre-result state with life-loss and counter
proposals. Every original result commits inside the same batch before any
result observers match. The real trample/lifelink scenario checks a low-life
replacement and a low-life intervening trigger against their distinct frames.
Toxic originals are deduplicated by exact (source, damaged player) after
positive final damage is established. Split/reconverged redirection and distinct
recipient scenarios retain two versus one toxic occurrences correctly.

Captured original receipts are now rejected before both history staging and
committed history ingestion. Public queue owners filter them before grouping,
so their already-completed amounts cannot inflate a fresh addition's occurrence.
The retained receipt proof survives scope-map cleanup; unmarked added events
remain publishable. Authored combat reingestion and ordinary effect-wrapper
scenarios assert exact damage, life-loss, life-gain amounts and occurrence counts.

The passive multi-source helper now exercises `DealDamageBySourcesEffect`,
selecting a captured `All` source set with per-source power and the original
recipient selector. It no longer depends on generic serial ForEach composition.
The player/multiplayer cases use the quantity lane's reviewed typed recipient-set
extension. Generic serial damage composition remains outside this closure.

## Checked damage representation follow-up

Damage Double/Multiply/Add and its life/counter numeric consequences now use
wide intermediates followed by explicit checked u32 event representations.
Final signed outcome quantities, completed combat totals, per-source lifelink,
prevented-damage grouping, marked damage, commander totals, and actual signed
life/counter state are checked before successful publication. Rules-imposed
zero floors and counter-event maximums remain actual mathematical restrictions,
not host overflow fallbacks. Dynamic replacement floors propagate query errors.

Threshold-only completed recipient sums use u128, so their evidence is not
saturated. Current-turn damage/loss/gain receipt totals are checked before
matching or effect staging, deduplicating existing receipt/projection aliases;
separate actions cannot make later signed history queries wrap.

Every failure uses ResourceLimitExceeded, with requested and supported bounds.
Existing incomplete-execution/query adapters therefore retain the failure, and
the real damage/combat/effect owner restores its transaction rather than returning
a smaller successful result. This is bounded host representation with explicit
failure, not an arbitrary-precision gameplay guarantee. Other independent numeric
domains (for example generic draw modification and arbitrary stat algebra) are
not claimed by this correction.

Authored `damage_numeric_representation` scenarios cover exact boundaries,
double/multiply/add overflow, consumed one-shot rollback/retry, combat rollback,
life underflow, lifelink overflow, infect-counter and marked-damage overflow,
and a second action exceeding current-turn quantity representation. Native
scenarios cover incomplete-execution classification, wide threshold evidence,
projection aliases, zero floors and real event counter limits. All remain unrun.

Numeric receipt deduplication uses exact occurrence identity only. Sharing an
instruction provenance does not merge two real assignments. The same-parent
sibling regression pins MAX-1 prior damage plus two distinct one-point receipts,
while an actual alias of one receipt counts only once. Production damage
observations also require unique child provenance for the existing staged-history
owner; that producer correction is retained in the quantity lane.

## Joint source-review closure

The dependency chain includes quantity-owner ba6effbf5, its split/batch
correction, typed recipient sets 78a7b73ea, independent consequence totals
d0092f41d and unique child observations 41cf5bc26; this lane supplies completed
combat/result frames through 9ab573bb5, history idempotence 34a2c88a3, migrated
fixtures d9edcd3af, and checked numeric consumers e337af293+abc6fd3ac. Bounded
independent review found no remaining concrete blocker in this combined scope.
The final compiled/full-corpus/gameplay validation gate is still outstanding.
