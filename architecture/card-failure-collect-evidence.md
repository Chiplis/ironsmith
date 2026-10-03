# Collect Evidence shared action

Status: **UNVALIDATED** source implementation and authored regressions. No build,
compilation or test run was performed. The deferred full replay must measure all
proposed coverage and supported-card changes.

## Rules and candidate scope

The official [Murders at Karlov Manor release notes](https://magic.wizards.com/en/news/feature/murders-at-karlov-manor-release-notes)
and [mechanics article](https://magic.wizards.com/en/news/feature/murders-at-karlov-manor-mechanics)
define collecting evidence as exiling cards from the actor's graveyard with total
mana value at least the stated threshold. An unavailable collection cannot be
chosen. Zero is a valid collection, including no cards, and triggers collection
observers. Incinerator's X is chosen for the action; overpaying does not change it.

`fixtures/collect_evidence.json.fixture` retains all **14** frozen identities
containing this mechanic in `stack07-bc9e56e2`. **13** have a mechanic-root blocker;
Behind the Mask's recorded first blocker is an independent stat-setting tail.

**11 complete-card proposals:** Cryptex; Evidence Examiner; Forensic Researcher;
Hedge Whisperer; Incinerator of the Guilty; Izoni, Center of the Web; Memory Vampire;
Polygraph Orb; Sample Collector; Surveillance Monitor; Tenth District Hero.

**3 partials, excluded from complete-card coverage:**

- Behind the Mask: its conditional replacement base-power/toughness tail.
- Conspiracy Unraveler: an all-origin alternative-cost modifier must not grant
  permission to cast cards from otherwise inaccessible zones and cannot combine
  with a second alternative cost. No substitute hand-only or permissive casting
  grant was added.
- Kylox's Voltstrider: its linked casting permission also redirects the cast
  spell's graveyard destination to the bottom of its owner's library. That rider
  is outside the current action implementation and is not counted as complete.

Two existing supported controls, Lamplight Phoenix and Vitu-Ghazi Inspector, are
preserved separately in `fixtures/collect_evidence_controls.json.fixture`; they
are not new recovery claims.

## Implementation

- A serialized `CollectEvidenceEffect { amount: Value }` and typed compiler
  keyword action carry fixed, X and expression amounts. Normal effects,
  activation costs, and mandatory/optional additional casting costs share it.
- Shared activation-cost grammar owns complete `collect evidence N` components,
  including comma boundaries. The old suffix-only ad hoc cost reader is removed.
- Aggregate mana-value object selection and the normal exile executor remain
  authoritative. Card ownership, graveyard zone and current-object identity are
  validated before movement; replacements run through the normal zone pipeline.
- One appended `CollectEvidence` keyword-action enum value records the threshold,
  actor and source. Zero still has a successful/performed outcome, so “if you do”
  and keyword-action triggers both work without moved cards.
- Unbound action X is chosen at resolution from zero through available value;
  the announced X survives to reflexive effects and is independent of the total
  value exiled. Cost X hooks bound announcement and context-aware preflight checks
  its actual chosen amount. Casting preflight cannot count the cast card itself.
- Optional collection is not offered when its threshold is unavailable. Errors
  and pending choices restore action state and context; enclosing MayEffect
  checkpoints restore earlier children of an interrupted optional instruction.
- Amount/reference visitors, modal X replacement, payment capability checks,
  full “collect evidence” player triggers, runtime/wire type registries,
  source-card mapping and rendering are wired. Existing wire payloads are unchanged.
- The pre-existing atomic self-exile/evidence/return procedure keeps its distinct
  selected sets and adds the missing keyword event. Its renderer accepts both
  the prior three-step artifact and the event-bearing representation only when
  the event kind/amount matches the typed collection threshold.

## Authored deferred regressions

The public compiler/runtime target covers exact-card compilation and JSON typed
artifact transport for the 11 proposals; real Cryptex cost/mana activation and
counter placement; payer/opponent/zone boundaries; zero “if you do” plus actual
Surveillance Monitor event matching; unavailable optional collection; casting
source exclusion; announced-X preflight; pending optional-action rollback;
normal exile replacement; and real combat-trigger/reflexive Incinerator damage
using announced X despite overpayment. Separate supported controls exercise the
old linked-source procedure and optional additional casting costs.

Deferred commands:

`cargo test -p ironsmith-compiler-grammar --lib collect_evidence`

`cargo test -p ironsmith-compiler-grammar --lib numeric_keyword_actions_preserve_modal_header_x_binding`

`cargo test -p ironsmith-compiler-runtime --test collect_evidence`
