# CF8 integration review — 2026-10-09

All twelve package heads are merged into `cf8/integration`. The main checkout is untouched. `merged-heads.json` records the exact inputs. Source proposals are not evidence of supported cards.

## Handoff mechanisms and semantic integration

- p01: the checkpoint warning describes a retired wire codec. Current runtime savepoints clone the complete GameState, including followed replacement objects. A regression restores both before casting and after casting before checking the eventual replacement. Public claim snapshots are not restoration checkpoints.
- p02: removed the Siren's Call-shaped token rewrite. Actual `Ignore this effect for each <filter>` tokens narrow a supported mass instruction through the existing exception machinery. A preceding typed controller antecedent resolves "that player"; the parser uses the full delayed-instruction grammar.
- p08/p07: exact-amount and half-damage prevention share NextTimeDamagePreventionPortion. The payment publishes its own amount. A typed independent-X flag distinguishes "pay any amount" from paying a spell's printed X. Liege's player-loop protocol collects payments before a simultaneous token batch; added runtime evidence checks every player's token count and batch identity.
- p09/p12: finished the uncommitted source-quality and delayed-death readers, then added full-card direct/artifact regressions. p06 and p11 had committed their latest source edits; no abandoned dirty source remained in their worktrees.
- Duplicate retarget restrictions use one Player/Object model. The merged parsed damage-multiplier and amount-change shapes use shared replacement machinery; additional model combinations are listed under remaining limits. Frozen object identities preserve other filter restrictions rather than replacing the entire filter.
- Null Chamber now lets its controller choose the opponent who names the second card in multiplayer, rather than automatically selecting the next opponent.
- Cycling self-triggers function from their pre-discard hand snapshot, even when discard is replaced by exile or a library move. The other-card battlefield arm remains gated separately. This implements the hand-origin rule without checking the current graveyard/exile location.
- Artist's Talent's regression compiles a real source artifact rather than depending on an absent generated frontend JSON file.

## Compatibility

Artifact format advances once, from 17 to 18. `architecture/cf8-integrated-schema.descriptor` fingerprints the integrated wire models. Previous descriptors retain their original bytes. Format 17 and its schema are explicitly rejected. The current golden is regenerated from the current library fixture. The real historical v3 fixture remains untouched; the previously referenced v5 fixture is unavailable in repository history, so historical-byte coverage uses v3 instead of fabricating a v5 file.

## Source-ledger tally and validation decision

The latest package ledgers contain 721 source-proposed rows, plus 19 dependant-proposed and 11 collateral rows. Deduplicating these statuses yields 746 proposed/collateral cards. There are 1,934 distinct ledger oracle IDs and 66 cards with differing statuses across packages; these are preserved in `ledger-tally.json`, not resolved by blindly trusting the optimistic status. Package summaries have stale round counts; latest ledger rows are authoritative only for source work.

This is enough source coverage to justify full corpus validation before landing. A compiler acceptance result does not prove runtime correctness. The full frozen-corpus comparison and broader test rerun are complete; results and remaining failures are recorded below.

## Deliberately unsupported

Stromgald Spy's persistent public-hand permission needs a coherent hidden-information protocol. That mechanism is not implemented or silently marked supported in this integration. Other blocked/dependant-blocked ledger entries remain explicitly unsupported pending independent mechanisms.

## Final validation

The compiler and corpus audit use frozen source `e3ce2aafbfb08d7060c4669f609ba72d127182b3` (tree `e72cf35525f44a866ad630ecfca4fafe145f5f5d`), with a clean source checkout. Subsequent commits that only add this report do not change that compiler provenance. `validation.json`, `corpus-run.json`, and `validated.snapshot.json.gz` preserve the evidence.

- **PASS:** `cargo check --workspace --all-targets --keep-going` on the final integrated source. The build log is retained.
- **PASS:** 40 focused tests: 34 static composition/runtime regressions (including 34 complete frozen cards), five p02 timing/exclusion tests, and one conditional-exclusion branch regression.
- **PASS:** 21 artifact/golden checks on the final merged source, with golden updates disabled.
- **PASS at an earlier integrated revision:** checkpoint restoration, single tagged identity filtering, and concrete-payload serialization regressions; the relevant mechanisms are unchanged in the final reader-only repairs. Their original logs are retained with that provenance distinction.
- **Broader rerun:** 142 previously failing targets from the earlier broad run, 220 passing tests, **158 failing tests across 116 targets**; exit 101. This is not a green test suite.
- **Full corpus:** all 32,209 canonical cards, using the exact baseline dataset SHA `9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c` and compiler binary SHA `f3b12054fccc10c9885653d17d3c6e2bf868d6b2a9f9c9cdec87fe8c35aa5913`. 31,126 cards meet the strict support criterion, versus 28,971 at baseline: a net **+2,155**. 1,083 remain failing/unsupported under that criterion. No compiler panic category was observed.
- **Baseline comparison:** 2,175 baseline failures now satisfy the criterion; 1,063 remain. **20 formerly supported cards now fail**. There are 157 comparison flags including score decreases or new semantic mismatches; reason counts overlap and are saved in `comparison.json`. The full campaign gate remains false because unresolved cards and comparison flags remain.
- **Proposed cohort:** **653/746** deduplicated proposed/collateral cards satisfy the strict criterion. The other proposals are not promoted as supported. Per-card results are in `proposed-card-results.json`.

These counts measure strict compilation without reported unsupported content or parse loss. They do not establish correct gameplay for every accepted card. Semantic similarity is reported independently. The exact corpus input is retained as `frozen-cards.json.gz`; decompression reproduces the recorded dataset hash. The baseline is the tracked `fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz`.

| Package | Proposed/collateral cards | Strict-supported |
|---|---:|---:|
| p01-lossy-semantic-markers | 64 | 57 |
| p02-linefamily-a | 62 | 57 |
| p03-linefamily-b | 54 | 50 |
| p04-noverb-a | 73 | 62 |
| p05-noverb-b | 64 | 56 |
| p06-predicates | 79 | 72 |
| p07-other | 54 | 49 |
| p08-other | 71 | 58 |
| p09-other | 59 | 49 |
| p10-other | 39 | 37 |
| p11-other | 62 | 57 |
| p12-other | 70 | 54 |

Package rows overlap; the deduplicated cohort is the count above. Comparison reason counts: new_semantic_mismatch: 45, previously_supported_card_failed: 20, similarity_score_decreased: 137.

## Integration corrections found during review

The package ledgers were treated as proposals, not proof. Review and validation found shared-reader precedence and ownership bugs: complete static compositions lost sibling clauses; attached-object readers dropped quoted triggers, activations, or statics; a keyword grant reader swallowed an unrelated quoted grant; and a plural source-exiled casting permission fell through to an immediate casting instruction. The integrated fixes use existing typed readers and lowering paths rather than card-name hooks. Full frozen-card fixtures cover 34 affected cards. The full-corpus comparison also caught a composition ownership regression: commas in keyword lists, conditions, and single X definitions were being claimed before their complete existing readers. The composed reader now proves an omitted-subject sibling before committing; complete quoted tails keep priority over narrower attached probes. Gameplay checks preserve both conditional stat branches, owner-scoped graveyard X counts, and graveyard-only keyword sharing. Gameplay checks exercise attachment changes, controller-scoped Skeleton counts, Aura/Equipment attachment counts, and Rona’s persistent source-linked casting permission through activation, resolution, legal-action enumeration, and source departure.

The final p02 handoff is included in merge ancestry. Its trailing-exclusion parser covers shared mass instructions, and a regression verifies that both branches of a conditional are narrowed rather than just the first branch. The earlier, narrower line-family implementation is removed.

Other integration repairs preserve announced activation-time values through sacrifice costs, responses, and checkpoint cloning; keep the selected land face through entry preparation and commit; and serialize concrete effect payloads before type erasure to avoid an erased-serde panic for omitted optional enum fields. The serialization repair keeps the existing JSON shape. It adds a JSON value allocation that has not been benchmarked.

## Reading the test results

The broader rerun deliberately retains red tests. Some complete-card tests fail because another line on that card remains unsupported; some assertions expect an older internal representation; others require gameplay investigation. Parser acceptance alone does not resolve those categories. For example, Detention Vortex retains a separate sorcery-speed condition alongside opponent-only timing, whereas its existing test expects a combined timing enum. That enum mismatch is not evidence by itself that the timing restriction is lost. Flaring Flame-Kin’s source-line static group also differs from the older per-piece shape assertion; a focused gameplay regression verifies the enchanted condition.

Gameplay assertions also remain red, including Gond Gate’s controller-scoped entry behavior and Zaffai’s free-cast legal actions. Those require investigation and must not be dismissed as representation-only failures.

All individual target counts, failing test names, and failure outputs are saved in `broader-test-results.json` and its compressed log. Those results are a validation gate, not a green-suite claim. The corpus comparison records newly failing previously supported cards separately from semantic-score or rendered-text changes. A score decrease can warrant review without demonstrating an incorrect game action.

## Remaining limits

Stromgald Spy remains unsupported: persistent public-hand permission needs a coherent hidden-information protocol. This integration does not introduce a mental-poker mechanism or silently count it as supported. Blocked and dependant-blocked ledger entries remain blocked.

Damage-multiplier registration freezes single tagged-object identities and referenced players while retaining other filter restrictions. Its current freezer does not capture a general multi-object tagged set or every tagged relation in the original resolution context. That broader context retention needs a targeted runtime scenario before it can be claimed supported; the single-identity regression does not establish it.

The damage-registration model also admits combinations the current grammar does not emit: a minimum threshold with the factor-only path, and a noncombat restriction with an amount override. The executor does not retain those respective fields in those combinations. They remain unsupported model combinations; this review does not identify a current parsed card relying on them.

No changes from this integration were merged into the main checkout, pushed, deployed, or submitted as a pull request. The existing package worktrees retain their local build-cache configuration. The integration checkout’s original local build-cache configuration is restored after validation and is not committed.

## Previously supported cards now failing

These compiler/diagnostic regressions require review before landing. The table records diagnostics; it does not assume the old acceptance was correct.

| Card | Current diagnostic |
|---|---|
| Brenard, Ginger Sculptor | lossy_compilation: suffix_object_filter_recovery: parsed 'it if you do create a token that's a copy of that creature except' as suffix of 'whenever another nontoken creature you control dies you may exile it if you do create a token that… |
| Bride's Gown | parser does not yet support line family: 'Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.' [rule-path=unsupported-line-fam… |
| Celestial Reunion | unsupported predicate (predicate: 'this spell's additional cost was paid and revealed card is chosen type') [rule-path=leading-if-conditional > sentence-reading] [rule-path=statement-line > statement-probe]; oracle-only fallback also fai… |
| Dihada, Binder of Wills | counter-cost quantity requires the matching counter kind and exact paid scope; oracle-only fallback also failed: counter-cost quantity requires the matching counter kind and exact paid scope |
| Essence Reliquary | object filter has an unsupported controller qualifier 'attached to it' [rule-path=return-clause > statement-reading] [rule-path=composable-typed-statements > after-direct-registry-reading]; oracle-only fallback also failed: object filter… |
| Flame Discharge | damage amount replacement requires retained cast-time control evidence [rule-path=damage-amount-replacement] [rule-path=statement-line > statement-probe]; oracle-only fallback also failed: damage amount replacement requires retained cast… |
| Groom's Finery | parser does not yet support line family: 'Equipped creature gets +2/+0. It gets an additional +0/+2 and has deathtouch as long as an Equipment named Bride's Gown is attached to a creature you control.' [rule-path=unsupported-line-family]… |
| Hofri Ghostforge | lossy_compilation: suffix_object_filter_recovery: parsed 'it if you do create a token that's a copy of that creature except' as suffix of 'whenever another nontoken creature you control dies exile it if you do create a token that's a cop… |
| Invoke Despair | missing restriction tail in negated restriction clause (clause: 'an enchantment can't') [rule-path=parse_cant_clauses] [rule-path=static-ability-line > granted-ability-component-registry-reading] [rule-path=coordinated-and-segments > cha… |
| Orzhov Charm | object filter has an unsupported controller qualifier 'attached to it' [rule-path=return-clause > statement-reading] [rule-path=composable-typed-statements > after-direct-registry-reading]; oracle-only fallback also failed: object filter… |
| Rhino's Rampage | missing prior effect for when clause; oracle-only fallback also failed: missing prior effect for when clause |
| Sakashima the Impostor | unsupported complete enters-as-copy name exception tail [rule-path=enter-as-copy-as-enters-line > static-compound-line-registry-reading] [rule-path=static-line]; oracle-only fallback also failed: unsupported complete enters-as-copy name … |
| Sinister Concierge | unsupported complete negated restriction clause (clause: 'each card exiled this way that doesnt have suspend gains suspend') [rule-path=statement-line > triggered-line]; oracle-only fallback also failed: unsupported complete negated rest… |
| Spectra Ward | parser does not yet support line family: 'Enchanted creature gets +2/+2 and has protection from each color. This effect doesn't remove Auras. (It can't be blocked, targeted, or dealt damage by anything that's white, blue, black, red, or … |
| Spiteful Repossession | comparison reference requires an earlier explicit value comparison; oracle-only fallback also failed: comparison reference requires an earlier explicit value comparison |
| Stormwild Capridor | could not find verb in effect clause (clause: 'prevent that damage'; known verbs: add, move, deal, draw, counter, destroy, exile, untap, scry, discard, transform, convert, regenerate, mill, get, reveal, look, lose, gain, put, sacrifice, … |
| Stunning Strike | parser does not yet support line family: 'As long as enchanted creature isn't legendary, it doesn't untap during its controller's untap step.' [rule-path=unsupported-line-family]; oracle-only fallback also failed: parser does not yet sup… |
| The Prydwen, Steel Flagship | lossy_compilation: suffix_object_filter_recovery: parsed 'control enters create a 2/2 white human knight creature token with this token' as suffix of 'whenever another nontoken artifact you control enters create a 2/2 white human knight … |
| Trade Route Envoy | pending filtered effect metric requires a prior memory-producing effect; oracle-only fallback also failed: pending filtered effect metric requires a prior memory-producing effect |
| Vivien on the Hunt | card selection has no resolved source zone or referenced collection; refusing an implicit battlefield selection; oracle-only fallback also failed: card selection has no resolved source zone or referenced collection; refusing an implicit … |
