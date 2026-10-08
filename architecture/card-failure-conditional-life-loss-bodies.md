# Conditional life-loss complete bodies

Source-only review at ff30f190c72b80b9c212d30056d402e94d4f2c86. All authored
scenarios are UNRUN. No build, test, probe, formatter, corpus replay, codegen,
remote write, or accounting change is authorized in this work.

Only Tezzeret's Simulacrum (dd886509-7455-4c8f-976f-0f5fafcb97be) and Necra
Sanctuary (2586a59d-8501-4c22-9d69-f4bf91de7024) are in scope. Exact full Oracle
bodies and printed metadata were copied from cards-20261003.json.xz into
fixtures/conditional_life_loss_bodies.json.fixture. Both IDs occur in the retained
unadmitted-measured-recoveries list. Historical measured compilation success is
not source/gameplay admission; no new measured recovery or residual change is
claimed. Other conditional-life siblings remain held independently.

Source review and authored evidence are complete for this packet. Existing
production owners are retained; only frozen fixtures, regression scenarios and
this report are added. Runtime validation remains deferred.

## Source review findings

No production owner change is needed on the inspected source paths. In
`effect_sentences/chain_carry/chain_carry_reference.rs`,
`subject_verb_player_action_player_mut` explicitly includes LoseLife alongside
Draw/GainLife; `explicit_player_for_carry` uses that player action projection.
`carried_player_from_effect` in the subject-verb reference followups retains
explicit player/opponent subjects, including nested sentence wrappers.
`post_rule_future_zone_and_self_replacement` obtains the prior player, binds
`that player` in the true arm, and places the original action in the false arm.
There is no need to execute the 1-life result to obtain the replacement target.

The semantic-line conditional-self-replacement classifier accepts a complete
leading-if consequence with the terminal replacement marker. Existing document
attachment keeps this sentence with the prior ability; the targeted authored
complete-body tests assert exactly one ability, one self-replacement, and the
independent full rendered rules. Necra's outer trigger retains its own
intervening-if, separate from the program's amount replacement. The runtime
uses existing resolution-program gating and the life-loss original owner.

## Authored evidence (all UNRUN)

`crates/ironsmith-compiler-runtime/tests/conditional_life_loss_bodies.rs` contains:

- Strict direct compilation independent of strict artifact compilation; validated
  artifact JSON round trip and materialization; separate native runtime-definition
  encoding, JSON round trip, and materialization. Every route runs each scenario.
- Independent exact full executable-body expectations and printed mana cost,
  types, subtype, P/T, colors, color identity, single-ability and single-replacement
  assertions. No compared route is treated as its own independent body oracle.
- Real paid Simulacrum activation with pending announcement state clone/continuation,
  one opponent target (self excluded, third player selectable), actual tap,
  activation history/source receipt, no immediate reactivation, and no loss from
  summoning-sick or foreign-controlled source admission.
- One-versus-three life loss, exactly one non-damage life-loss event, correct
  selected opponent, own versus opposing Tezzeret, graveyard/phased/non-planeswalker
  subtype witnesses, wrong planeswalker subtype, late arrival/departure/control
  change, paid stack recovery, and source control change/departure after payment.
- Real upkeep dispatch and target selection for Necra, self target allowed,
  other player's upkeep excluded, absent/green-only/white-only/both colors,
  one multicolored permanent, opponent-only colors and split controllers.
- Separate resolution rechecks of outer OR and inner AND after permanent departure,
  control change or color change, a newly qualifying second color, source departure,
  and native state recovery. Outer failure causes zero loss; inner failure leaves
  exactly the original one life rather than adding or bypassing the outer gate.
- Illegal sole player target resolves without loss or retargeting, preserving
  already-paid tap. External life-loss replacement with an earlier gain followed
  by a pending choice or token-budget error rolls the complete resolution back,
  preserves the paid activation and one-shot replacement, and retries through the
  native stack owner with precisely one replacement result.

Recovery coverage is explicitly native GameState/PriorityLoopState cloning and
runtime-definition codec recovery. It is not a claim of an untested network or
full-game serialization protocol. No test was executed, so all source expectations
remain subject to the deferred validation gate.

## Scope and accounting

These two exact original-gate identities are the only proposed source closeouts.
No fixture accounting, ledger, residual gate or measured result was edited.
Ilysian Caryatid, Anoint with Affliction, The Destined Black Mage, Thunderdrum
Soloist, all other conditional-life siblings and all other held bodies receive
no credit from this packet. No historical measured successes were reclassified.

## Independent source-review correction

The initial helper restored TurnRunner::Upkeep while leaving the generic game's
FirstMain phase intact. That cannot emit the ordinary upkeep event: the dispatch
owner requires Beginning/Upkeep together. The helper now restores the genuine
Beginning/Untap boundary, lets TurnRunner advance into Upkeep, and asserts the
resulting Beginning/Upkeep state and UpkeepPriority runner state before stacking
triggers. Positive Necra assertions require one actual trigger; negative cases
remain coupled to the same genuine dispatch path. This is a test setup correction,
not a production-owner change. Execution remains UNRUN.
