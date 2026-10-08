# cf8 / p08-other — summary

175 cards. Source-only, nothing built or run (campaign policy). The prebuilt `compile_oracle_text` was used read-only for triage until it disappeared from the main checkout mid-session; later fixes (Sneak) and remaining-line checks for some cards were not probed.

| status | count |
|---|---|
| source-proposed | 34 |
| already-on-main | 1 (Loathsome Troll, cba78f342) |
| blocked | 81 |
| untriaged | 59 (singletons; the observed gap is recorded per card) |

## Clusters fixed (general fixes)

| cluster | cards | root cause / fix | files |
|---|---|---|---|
| bare-singular-object-noun-recipient | Settle the Score, Liliana's Scrounger, Cosmium Confluence, Wick, Astarion's Thirst | The counter placement consumes "a" and hands "planeswalker you control" to the target parser. Only `creature`/`permanent`/`card` were head-committed. Card-type, supertype, subtype and `commander` heads now parse through the same prefix grammar as a non-targeted, exactly-one choice. | grammar `target_semantics/reference.rs` |
| departed-object-controller-that-player | Pain Distributor, Sardian Avenger, Shriek | No player was inferred for zone-change, dies, or put-into-graveyard-from-battlefield triggers whose filter names a non-you controller. They now infer `AliasedControllerOf(triggering)` (LKI, CR 603.10a), as the entering-permanent rule does. | resolve `trigger_players.rs` |
| join-forces-validator | Minds Aglow, Collective Voyage, Alliance of Arms, Mana-Charged Dragon | The IteratedPlayer validator didn't traverse `CollectManaPaymentsEffect`. The payment head also now accepts the "each player starting with you may pay" order. | lowering `lowering_support.rs`; grammar `pair_procedure/collect_mana_payments.rs` |
| counter-kind-put-another | Maulfist Revolutionary, Skyship Plunderer, Powerful Broker | New put-only, all-kinds `ForEachCounterKindPutOrRemove` form ("give that permanent or player another counter of that kind"). The engine handles a chosen player target via the shared counter batch (CR 122.1). Text rendering added. | grammar counter shapes + family; semantic constructor; engine executor (+`matching_player_targets_for_spec` pub(crate)); text `late.rs` |
| granted-banding | Cooperation, Dire Wolves, Fortified Area | `runtime_static_ability_for_keyword_action` lacked a Banding arm (CR 702.22). | lowering `lowering_support.rs` |
| separate-line-instead-restatement | Slaying Fire, Summary Judgment, Fiery Impulse | A standalone (optionally labeled) "If <cond>, <action> instead." line wasn't joined to the preceding spell statement. It is now joined only when the condition has no "would" (CR 614.1a); the existing amount self-replacement then owns it. | grammar `followup_shapes.rs`, `statement_recognition.rs` |
| twice-that-much-damage | Chocobo Kick | The amount replacement accepts "twice that much" as `Scaled(prior amount, 2)`, plus a definite description of the targeted damage source. | grammar `followup_shapes/combat.rs`, `subject_verb_followups_combat.rs` |
| reflexive-fight | Back for More, Curse of the Werefox | The headless fight primitive claimed "when you do, it fights". It now yields when/if-led subjects to the reflexive rule (CR 603.12). | grammar `clause_primitives.rs` |
| static-reading-ownership | Thran Lens, Mycosynth Lattice, Vnwxt, Wings of Velis Vel | Non-equivalent registry ambiguities. The generic readings defer to the dedicated colorless, draw-doubling, and base-P/T readings. | `costs_replacements_and_permissions.rs` (two small additive hunks), `clause_readings.rs` |
| named-untap-step-tagged-set | Sleep | "That player's next untap step" for a tagged set binds to the declared target player, held as the last player antecedent. | lowering `subject_verb_middle.rs` |
| library-viewer-player-antecedent | Fortune's Favor, Atris | The reference pass didn't track "Target opponent looks at ..." as the player antecedent, though lowering did. The pile chooser therefore became IteratedPlayer. | resolve `reference_resolution.rs` |
| bare-sneak-keyword | Leonardo, Shark Shredder, Dark Leo & Shredder | A bare "Sneak {cost}" without reminder text was rejected. The engine already handles Sneak casts and permanent entry. Other lines were not re-probed. | grammar `alternative_cast_readings.rs` |

Tests (authored, unrun) are in `crates/ironsmith-compiler-runtime/tests/`:
- `bare_object_noun_counter_recipients.rs`
- `departed_object_controller_player.rs`
- `join_forces_collective_scope.rs`
- `counter_kind_put_another.rs`
- `granted_banding.rs`
- `separate_line_instead_restatement.rs`
- `twice_that_much_damage.rs`
- `reflexive_fight.rs`
- `static_reading_ownership.rs`
- `named_untap_step_tagged_set.rs`

They share the helper `cf8_p08/support.rs`. Fortune's Favor and Atris rely on the existing `binary_card_piles.rs`; the join-forces sorceries also rely on `join_forces.rs`.

## Risk notes
- **Bare noun head** (shared hot spot): the fallback fires only on NoMatch, for a singular type, supertype, subtype or `commander` head, and only when `parse_object_filter` succeeds.
- **Zone-change "that player"**: already-compiling cards with such triggers now resolve to the departed object's controller instead of IteratedPlayer. That is the intended change, but compiled output shifts.
- **Instead-restatement join**: it applies to any statement followed by "If ..., ... instead." whose condition has no "would".
- **Engine player branch**: it uses the `execute_counter_batch_with_outputs` gateway and adds no world-checkpoint wrapper. Check it against the incoming transaction refactor.
- **Mana-Charged Dragon**: depends on the stage-84 CollectManaPayments X scope inside a triggered pump.

## Blocked, grouped by missing mechanic
- **Opponent chooses the mode at cast time** (resolution-time chooser only): Fatal Lore, Library of Lat-Nam, Misfortune.
- **Single-player "pay any amount of mana" with a per-player amount-paid value**: Errant Minion, Power Leak, Karn Living Legacy, Leyline Tyrant, Liege of the Hollows.
- **Dynamic life cost bound to the trigger object**: Lorcan, Madame Null.
- **Past-tense "that creature was"** after an intervening source action: Taborax, Venom.
- **Villainous-choice faced-player binding on triggered paths**: The Dalek Emperor, Damocles Base, Ensnared by the Mara.
- **"either of them" re-binding**: Call of the Death-Dweller, Wicked Slumber.
- **Conditional "has K and is a T"**: Hero of Bretagard, Skyknight Squire, Ezio.
- **"up to that many targets" / per-result metrics**: Cephalid Constable, Froghemoth, Coveted Falcon.
- **Filtered metric memory producer**: Convert to Slime, The Hunger Tide Rises.
- **Per-attacker sacrifice attack tax**: Flooded Woodlands, Reclamation.
- **Non-terminal die-roll owner / row-bound X**: Druid of the Emerald Grove, The Deck of Many Things, Wand of Wonder.
- **Compound recipient sets**: Sewers of Estark, Cerebral Eruption, Corpse Explosion, Faith's Shield, Kitsune Palliator.
- **Amount-modifying damage replacements**: Lashknife Barrier, Benevolent Unicorn, Neriv.
- **Chosen-color additional-mana triggers**: Gauntlet of Power, Caged Sun.
- **Selection phrases**: Sword of Hearth and Home, Sorrow's Path, Legion's End, Ultimate Nullification, Split the Party, Ulamog, Dwarven Catapult, Crashing Wave.
- **Unresolved "it"**: Desolation, Elite Arcanist, Instill Furor, Ixhel, Nascent Metamorph, Replicating Ring, Snort, Talion's Messenger, Tavern Brawler, Teo.
- **Dynamic mana/roll amounts**: Danse Macabre, Drain Power, Elemental Resonance, Gale's Redirection, Radiant Lotus, Quag Feast.
- **Single-card mechanics** (each card's ledger entry records its gap): Animation Module, Crystalline Giant, Tasha, Ashiok, Lara Croft, Fblthp, Sarkhan, Gimli, Fear of Sleep Paralysis, Piracy, Ethersworn Shieldmage, Voidpouncer, Equipoise, Ambassador Blorpityblorpboop, Rakdos the Showstopper, Blast-Furnace Hellkite, Desperate Plea.
- **Untriaged** (59 singletons) carry their observed gap in the ledger.

## Cross-package conflict risks
These shared hot spots are likely touched by other packages too:
- `target_semantics/reference.rs`
- `lowering_support.rs`
- `trigger_players.rs`
- `reference_resolution.rs`
- `statement_recognition.rs`
- `costs_replacements_and_permissions.rs`

All edits are small additive hunks.
