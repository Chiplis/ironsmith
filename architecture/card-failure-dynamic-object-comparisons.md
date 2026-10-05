# Dynamic object-comparison family

Status: **UNVALIDATED**, implementation-first source proposals. No compilation, builds, or tests executed.

## Reserved exact seven-card inventory

1. Beguiler of Wills
2. Vedalken Shackles
3. Fell the Mighty
4. Mirko, Obsessive Theorist
5. Winter Soldier, Reborn Avenger
6. Nihiloor
7. Sovereign Okinec Ahau

Custodi Peacekeeper is excluded: its draft-history quantity is a separate mechanic.

## First checkpoint: two proposed complete cards

Beguiler of Wills and Vedalken Shackles use existing typed Count expressions inside a LessThanOrEqual power target filter. A legacy dispatch guard rejected the combination of power, number, and you-control before that parser could run. The guard is removed; unknown operands still fail in the complete target-filter parser.

The bound is target legality: it is checked when the target is announced and again on resolution. It is not a condition that ends control later. Beguiler's control remains indefinite after it resolves. Shackles instead ends when its own tapped-state duration ends; neither later Island count nor a change in the artifact's controller is that duration.

The ordinary GainControl executor now emits ControlChanged and breaks a soulbond pair only after an actual controller transition. Previously, a false initial ForAsLongAs duration could produce neither a control effect nor a controller change, but still emit that event and break the pair. This correction checks the actual post-application controller.

Exact complete source fixtures: `fixtures/dynamic_control_bounds.json.fixture`. Authored normal compiler-runtime target `dynamic_control_bounds` has five tests with direct and JSON-materialized definitions: typed filters, positive/negative power announcement boundaries, resolution response changes, source-controller changes, lasting control, true/false initial tapped durations, no revival after retap, and the printed optional untap choice. Tools target covers both metadata-bearing strict/non-lossy cards. Grammar tests verify typed bounds and reject an unknown operand. Engine regression pins the false-duration receipt/soulbond behavior. All are unrun.

## Remaining work, no additional coverage claim yet

- Fell the Mighty is closed by the second checkpoint below.
- Mirko / Winter Soldier: builder-aware source possessives (including elided power), graveyard targeting and complete entry/finality conditions.
- Nihiloor: body-tapped object identity must take precedence over a tap-cost object. Its power must remain live for the same incarnation and use exact departure LKI; a frozen FirstPower result metric would be incorrect. Reflexive target announcement and source-control duration need whole-body scenarios.
- Sovereign Okinec Ahau: current-versus-base-power selection and the corresponding per-object difference. The existing base-power filter path reads raw base fields; a correct rules base-power value must include characteristic-defining and set-base-P/T layers, excluding later modifiers/counters. Do not count merely recognizing the phrase.

## Second checkpoint: Fell the Mighty (three of seven proposed complete)

The shared quantity reader preserves the explicit targeted creature in PowerOf / ToughnessOf. The non-targeted set-filter prelude exposes that simple numeric operand as a real target declaration. The destroyed set is still a non-targeted set. The standard stack target gate prevents resolution if the single reference target becomes illegal; a returned incarnation is not the original target. All destruction candidates are selected before the existing simultaneous destruction transaction begins.

The comparison adapter now reads a tagged/targeted reference's calculated live P/T for the same exact object ID and zone. After departure, it first checks the existing exact-object-ID zone-change receipt (including staged simultaneous events), then the supplied LKI. It never follows a stable physical-card ID through a blink. Pending-stack tags are also refreshed at actual departure; the receipt lookup covers independently retained/active-resolution tags that were captured earlier. Source fallback is unchanged.

Exact full-card fixture: `fixtures/target_characteristic_comparisons.json.fixture`. Four authored compiler-runtime tests cover the typed target prelude, response pumps (including negative power), actual casts with blink/shroud invalidation, and tagged live/departure comparisons after blink with a differently sized returned incarnation. Grammar and normal tools tests are also authored, unrun. This checkpoint does not claim multiple independent characteristic targets or base-power support.

A fifth authored runtime regression retains a selection tag outside the stack, pumps then blinks that object, and makes a second departure with different power. Comparison must use the original incarnation's actual departure receipt, not the earlier selection or later card incarnation.

## Third checkpoint: Mirko and Winter Soldier (five of seven proposed complete)

The shared comparison reader accepts an elided power/toughness axis only for a typed source possessive. Existing builder-aware name normalization supplies that self reference; arbitrary names and mana-value-axis ellipses still fail. The following zone phrase stays outside the operand. Winter Soldier's full possessive already uses the same explicit PowerOf source primitive after that normalization.

Winter Soldier exposed an independent bounded tail: the entry-counter grammar previously hardcoded Creature for “if a creature enters this way.” It now carries a parsed card-type or subtype filter through the existing typed result/conditional fusion. A Hero predicate becomes an object-filtered BattlefieldEntryCounterSpec on the return producer, before entry replacements and triggers. It does not put a later counter on every returned creature.

Exact metadata/source fixtures: `fixtures/source_comparison_returns.json.fixture`. Five authored compiler-runtime scenarios (direct and JSON) pin both target filters and fused entry counters, Mirko's actual surveil counter trigger and optional end-step return, response power changes, source-controller changes, source departure/blink LKI, finality exile, and Winter Soldier's actual attack with Hero/non-Hero entry filtering plus intrinsic entry counters and a size-based entry witness. Normal tools and typed grammar regressions are authored, unrun. Nihiloor and Sovereign Okinec Ahau remain explicit partials.

## Fourth checkpoint: Nihiloor (six of seven proposed complete)

“The tapped creature's power” is a distinct compiler alias, bound to the preceding local tap result. Imported tap-cost tags remain a fallback (including their actual namespace/index); a local body tap supersedes them, and unrelated later object memory does not. A missing tap producer fails closed. The final runtime quantity remains PowerOf the exact tagged object, preserving live power and departure LKI rather than freezing the tap-time number.

A player-loop tap followed by a reflexive trigger that explicitly reads that tapped-object alias is transported inside the player iteration. Each successful tap queues its own reflexive ability with that iteration's player, exact object, and result; targets are announced only for those later abilities. Declining the optional tap produces no trigger. Existing source-control duration machinery enforces the initial condition and expiry.

The secondary attack ability exposed two bounded reference/recognition gaps. An explicit singular a/an blocks the plural probe from mistaking the terminal verb “owns” for a plural attacking group. For a singular creature you control that an opponent owns, the inferred “that player” is the tagged attacker's owner. It is neither the defender nor Nihiloor's later controller.

Exact fixture: `fixtures/tapped_object_comparisons.json.fixture`. Four authored direct/artifact runtime scenarios cover typed artifacts, three-player per-opponent tap/target isolation, response shrink versus exact departure/blink LKI, declined taps, initial/expiring source-control durations, and two stolen attackers producing two owner-directed life-drain triggers. Added resolve tests preserve imported costs while proving local-body precedence and missing-producer rejection; a normalization test pins reflexive loop placement, and a grammar test pins the singular probe. All unrun. Sovereign Okinec Ahau remains partial at the true base-power boundary and per-object difference.
