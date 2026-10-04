# Damage multiplier sources, recipients, and duration

Status: **UNVALIDATED.** Source review, fixture extraction, formatting and diff checks only. No builds, compiler execution, or tests were run under the implementation-first workflow.

## Exact candidate membership

Eight frozen stack07 failures are proposed complete by this bounded family:

1. Blind Fury
2. Curse of Bloodletting
3. Goblin Goliath
4. Goldnight Castigator
5. Inquisitor's Flail
6. Quest for Pure Flame
7. Sawhorn Nemesis
8. The Rollercrusher Ride

`fixtures/damage_multiplier_scopes.json.fixture` preserves the frozen full Oracle bodies and metadata. All eight previously failed at the `double that damage ... instead` body. Expected future status is strict, metadata-bearing, non-lossy compilation and a functional replacement program, not merely a recognized static label. None appears in the source coverage ledger at the branch base (2ee64c7c).

Impulsive Maneuvers remains partial: its coin-flip gated next-combat doubling/prevention program is separate. Goblin Charbelcher remains partial: its reveal-until-land count, Mountain-dependent multiplication, and library-bottom ordering are separate. Neither is counted here. The separate multisource simultaneous-damage and Mathemagics execution-budget gaps remain open.

## Shared repair

- Read the complete multiplier clause, keeping source, recipient, repeated-recipient anaphor, combat/noncombat qualifier, duration, and live `while` condition separate. A changed repeated recipient is rejected rather than silently treated as the original target. Existing player/permanent union anaphors stay valid.
- Add typed self, creature, attached creature, and chosen-player/permanent recipients. An Equipment uses an intrinsic exact attachment relation, so an unattached Flail cannot borrow another Equipment's host. Incoming `another creature` excludes this host, not the Equipment that owns the ability.
- Append a targetless semantic registration action and add the serializable `RegisterDamageMultiplierEffect`. Temporary instructions register the existing real Multiply replacement with the resolving ability's controller. They are multi-use through cleanup and survive source departure, including Quest's sacrifice payment. The grammar admits only the bounded class-of-source/class-of-recipient temporary forms, not resolution-local or attachment/choice references that would need separate binding.
- Reuse the existing ordered damage replacement engine and exact source live/LKI filter matching. No post-damage adjustment, suppression of loss flags, or replacement-order bypass is introduced.
- A static multiplier with a condition installs the condition on the damage matcher. Its controller, graveyard, and source ownership are checked when damage would happen. The conditional static renderer also retains `noncombat`; its existing special branch had rendered that qualifier as ordinary damage.
- Wire the new effect through lowering, reference traversal, the split artifact decoder and card-graph mapper, native encoding/materialization, model interpretation, and effect rendering. No existing serialized enum ordinals are shifted.

## Authored evidence, not executed

Normal targets:
- compiler-runtime integration `damage_multiplier_scopes`: full exact-card direct/artifact-JSON materialization; nine gameplay/artifact tests
- tools integration `damage_multiplier_scopes`: aggregate all eight metadata-bearing strict/non-lossy snapshots
- grammar unit tests: typed recipients, exact attachment complement, live conditional/noncombat flags, resolving duration/source domains, pre-existing union anaphors, and rejection of redirections or lost durations

Gameplay scenarios use public casts/activations/payment, entry choice, damage/prevention, attachment/controller/zone changes, and cleanup. They cover Goliath's actual opponent-count token entry and departed damage-source LKI; Quest's actual damage triggers/counters/sacrifice payment; Flail's paid equip and outgoing/incoming/self damage; Blind Fury's trample removal and expiry; Curse's enchanted player; Castigator's changing controller/self; Sawhorn's actual chosen player; Rollercrusher's announced X, two chosen entry targets, and live delirium; affected-player choice between prevention and doubling; and two multipliers applying once each.

## Existing numeric/runtime constraints

The shared damage pipeline represents packet amounts as `u32` and existing `EventModification::Multiply` uses `saturating_mul`. This patch does not broaden that numeric model or claim arbitrarily large damage is exact. Ordinary finite damage and the existing replacement/prevention ordering are the supported boundary. Future full-suite and full-corpus execution is still required to establish compilation, registration plumbing, and regression safety; these source proposals must not be reported as measured successes.
