# Historical proposals vs Oct8 current measurements

Audit only. No source or ledger edits, execution, or remote writes. Exact ID cohorts, source-row evidence and input hashes are in `audit-historical-source-proposals.json`.

## Counts and scope

- Historical proposed: 1,291 original-failure IDs / 1,296 compile entries. Exact ID hash matches the inherited ledger.
- Oct8 measured: 1,269 compile-gate-supported; 22 unresolved (16 lossy, 5 parser failure, 1 semantic-output failure). Of 1,269 supported, 53 have heuristic flags, leaving 1,216 without flags. Neither subset is freshly verified whole-card source eligibility or gameplay correctness.
- Historical 40: all 40 compile-gate-supported, no heuristic flags. Remain excluded from the original 3,193 remaining-ID denominator and additional-source numerator.
- Historical majority stays 1,597 / 3,193. Historical numerator remains 1,291, historical shortfall 306. The fresh measurements do not establish a current-source numerator; neither 1,269 nor 1,216 may substitute for it.
- 30 retained reholds: 29 unresolved; God-Eternal Kefnet now compile-gate-supported but heuristic-flagged. Its source hold is not automatically lifted.
- 25 previously restored holds: 24 compile-gate-supported (7 flagged), Druid of the Emerald Grove remains parser-failed. All prior 55 holds combined: 25 supported (8 flagged), 30 unresolved.
- NEXT02 12 held candidates: only Orcish Farmer supported (unflagged); 11 unresolved. NEXT03 8 held candidates: Fourteenth Doctor and Revivify supported/flagged, The Master, Formed Anew supported/unflagged; 5 unresolved. Keep source holds pending actual substantive review.
- Historical 70 partial IDs: 27 supported (6 flagged), 43 unresolved. The distinct 13 excluded new-fixture partials: 7 supported (2 flagged), 6 unresolved. These overlapping inventories must not be summed or automatically admitted.
- Old current-residual 146 proposals: 139 now supported (33 flagged), 7 unresolved. Original-failure portion 67: 61 supported (19 flagged), 6 unresolved. Regression portion 79: 78 supported (14 flagged), 1 lossy. All regression-only IDs retain zero original-majority credit.
- Old 11 supported semantic corrections: 10 supported, Leyline of Singularity now lossy; old 12 supported evidence-only bodies remain supported/unflagged. These are overlapping analytical scopes, not additive counts.

## Exact 22 and targeted source-review links

- Claim Jumper (18f0cd0b-3e4f-4637-a62e-75dd1b2f3fce): parser_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: repeat-process-boundaries. Fixtures: fixtures/card-failure-campaign/repeat-process-boundaries.json.
- Baru, Wurmspeaker (1cf27d78-d6ae-4fa2-9c8d-c277913f821c): lossy_compilation; diagnostic family: Activated/equip cost reduction quantity or condition. Historical families: typed-activation-modifiers. Fixtures: fixtures/typed_activation_modifiers.json.fixture.
- Contamination (4013b4c2-c9ed-4d14-90f0-97214ec1fded): lossy_compilation; diagnostic family: Mana replacement conditional. Historical families: mana-output-replacements. Fixtures: fixtures/mana_output_replacements.json.fixture.
- Strength-Testing Hammer (4548d471-e78a-473e-84d4-cd2af64c9d28): lossy_compilation; diagnostic family: Dynamic damage/statistic/attack-tax quantity. Historical families: extrema-quantities. Fixtures: fixtures/extrema_quantities.json.fixture.
- Enduring Renewal (47a080c4-ff04-4f52-aca0-2b8e4f4d931e): lossy_compilation; diagnostic family: Draw-replacement conditional. Historical families: draw-replacement-programs. Fixtures: fixtures/draw_replacement_programs.json.fixture.
- Survey Mechan (53745903-de8f-42cf-a949-842e5831de30): lossy_compilation; diagnostic family: Activated/equip cost reduction quantity or condition. Historical families: typed-activation-modifiers. Fixtures: fixtures/typed_activation_modifiers.json.fixture.
- Bring the Ending (57b37690-0fb6-4e78-9210-4a529a6ade7c): lossy_compilation; diagnostic family: Postposed effect or static condition. Historical families: conditional-self-replacement-programs. Fixtures: fixtures/conditional_self_replacement.json.fixture.
- Dragonfly Swarm (83abf1d0-04e4-49e5-bf63-59f77829ffd1): parser_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: intervening-predicate-cohort. Fixtures: fixtures/intervening_predicate_cohort.json.fixture.
- The Ur-Dragon (87b22b09-4f6d-4bc5-9cfc-663e4c7c6981): semantic_output_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: residual-static-condition-cohort. Fixtures: fixtures/residual_static_condition_cohort.json.fixture.
- Druid of the Emerald Grove (acf54a85-0e9e-43fb-99d9-c223c02f13c4): parser_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: complete-die-result-programs, numeric-result-tables. Fixtures: fixtures/die_result_programs.json.fixture, fixtures/die_result_programs.json.fixture.
- The Rollercrusher Ride (aed1f0cd-8df8-415f-afd6-ccfd12234334): lossy_compilation; diagnostic family: Dynamic damage/statistic/attack-tax quantity. Historical families: damage-multiplier-scopes. Fixtures: fixtures/damage_multiplier_scopes.json.fixture.
- Sewer Crocodile (b01e9ff2-865a-401e-a839-568cd4a0451d): lossy_compilation; diagnostic family: Activated/equip cost reduction quantity or condition. Historical families: typed-activation-modifiers. Fixtures: fixtures/typed_activation_modifiers.json.fixture.
- Displaced Dinosaurs (bc0daba2-c51c-44b0-a4a3-806f2acd9aa6): lossy_compilation; diagnostic family: Entry counter/characteristic replacement. Historical families: chosen-type-damage-and-entry-characteristics. Fixtures: fixtures/chosen_entry_static_bodies.json.fixture.
- Leyline of Singularity (bc9f159b-984d-4b9f-8904-b38b5cb79636): lossy_compilation; diagnostic family: Opening-hand battlefield pregame (17; highest-confidence coherent witness family). Historical families: copular-characteristic-statics. Fixtures: fixtures/copular_characteristic_statics.json.fixture.
- Leyline of Hope (c7ee79c3-a273-45b4-b3a4-548d9b2b883c): lossy_compilation; diagnostic family: Opening-hand battlefield pregame (17; highest-confidence coherent witness family). Historical families: persistent-life-gain-replacements. Fixtures: fixtures/life_gain_replacements.json.fixture.
- Hawkeye, Young Avenger (c8f8b4ed-8455-45cd-a24c-e2f42cd5e5b1): lossy_compilation; diagnostic family: Dynamic damage/statistic/attack-tax quantity. Historical families: additive-damage-replacements. Fixtures: fixtures/additive_damage_replacements.json.fixture.
- Omnath, Locus of Creation (cd133d30-51ff-4114-a7d7-029345f0f0d7): lossy_compilation; diagnostic family: Ability-resolution ordinal conditional. Historical families: source-context-and-ward. Fixtures: fixtures/lossy_metadata.json.fixture.
- Infernal Darkness (dd821fca-79a0-48a9-b1cf-5a496bc04f65): lossy_compilation; diagnostic family: Mana replacement conditional. Historical families: mana-output-replacements. Fixtures: fixtures/mana_output_replacements.json.fixture.
- Ritual of Subdual (e0587218-206e-41ed-af3c-f06b7a668e90): lossy_compilation; diagnostic family: Mana replacement conditional. Historical families: mana-output-replacements. Fixtures: fixtures/mana_output_replacements.json.fixture.
- Belt of Giant Strength (eb70939f-c4ce-4224-ab74-acc24771d7f0): lossy_compilation; diagnostic family: Activated/equip cost reduction quantity or condition. Historical families: typed-activation-modifiers. Fixtures: fixtures/typed_activation_modifiers.json.fixture.
- Ethersworn Shieldmage (ee988017-fc7e-4d8a-8f5c-0a7e57a8d050): parser_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: temporary-prevention-bindings. Fixtures: fixtures/temporary_prevention_bindings.json.fixture.
- Walltop Sentries (fb3a0910-a582-41ef-b5b9-3cda1f1cd5ce): parser_failure; diagnostic family: not in lossy audit; inspect exact diagnostic in JSON. Historical families: intervening-predicate-cohort. Fixtures: fixtures/intervening_predicate_cohort.json.fixture.

## Minimal safe additive current overlay

Preserve both historical ledgers unchanged as immutable evidence. Add a separately dated current overlay keyed by Oracle ID and measured commit/tree, with exact input hashes and historical row references. Do not overwrite old current_measurement fields in a way that makes old review claims appear Oct8-reviewed.

Suggested fields: oracle_id; historical_scope (original_failure, historical40, original_supported_regression); historical_proposal_status; historical_review_commits_and_evidence; measured_commit/tree/snapshot_hash; measured_entry_outcomes; heuristic_flags; source_revalidation_status (pending_owner_impact_review, carried_with_current_owner_review, revalidated_full_body, held); current_review_tree; affected_owner_paths_and_hashes; independent_review_evidence; gate_credit_status (historical_only, current_source_admitted, excluded); exclusion_reason; execution_validation_status=UNRUN.

Current aggregate counters must derive from exact-ID unions, separately reporting historical 1,291, the 1,269/22 measurement split, and current-source-admitted IDs only after documented review. Until reconciliation establishes the active union, report current_source_eligible_count=null with pending status, never zero or an unqualified 1,291. The unchanged gate remains closed; satisfying a source number is not itself an execution authorization.

Do not discard inherited proposals wholesale: prioritize the 22 unresolved and 53 flagged candidates, then assess owner-impact drift for the remaining 1,216. Retain old full-body evidence and allow bounded independent owner/compatibility review to carry unaffected identities explicitly. A compile pass or heuristic absence cannot stand in for this review. Flagged status alone also does not demote an independently justified source proposal; inspect the signal and current body.

The old source admission records pin 225d50dd; the measured tree is 1dd81cd. This checkout cannot resolve the old commit object, so this audit does not claim a complete old-to-new owner diff. The historical ledger itself requires actual-new-base affected-owner and combined-semantic review; nearby current source audit documents compositional architecture changes. Therefore blanket active carry-forward is not justified by these inputs.

For 16 lossy proposals, correlate each exact audit family and diagnostic with the shared diagnostic-ownership correction, inspect preserved fixture plus complete currently composed semantics, and independently clear the combined tree. Do not infer that every lossy flag is a harmless ownership leak. Six non-lossy unresolved proposals require their specific parser/marker blockers and full bodies reviewed. Retain 53 heuristic signals as distinct review flags.

The five upcoming original-failure proposals (Anzrag, Glorfindel, Catoblepas, Summer Bloom, Journey) can be additive only after exact IDs, original eligibility, prior unaddressed status, current source body, and independent combined review are verified. Titania and the prior six originally-supported parser regressions have zero original-failure gate increment. No future candidate was credited by this audit.
