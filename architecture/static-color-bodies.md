# Four frozen static-color bodies

Status: implementation and scenarios authored; all builds, compiler runs, tests,
formatters, engine/corpus/browser execution remain deferred. This document grants
no runtime or campaign coverage credit.

The compact fixture is `fixtures/static_color_bodies.json.fixture`. It retains
complete Oracle text, printing and Oracle IDs, printed metadata, computed source
colors and Commander identity, the frozen baseline error, and dataset provenance.
The decompressed source dataset SHA256 is
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
Baseline commit: `e8740178a7f7367ffa3147e7642607042079237c`.

## Typed implementation

- The complete nominal color-clause grammar accepts `colorless` and dispatches
  normalized source references beginning with `this`. Unsupported predicate tails
  remain failures. Literal colorless lowers to `SetColors(COLORLESS)`; the Devoid
  keyword retains its distinct existing `MakeColorless` identity.
- Unqualified subtype subjects denote battlefield permanents (CR 109.2).
  Ghostflame therefore includes noncreature Sliver permanents and all controllers,
  while Sliver cards and spells outside the battlefield remain unaffected.
  Source lifecycle, phasing and current subtype matching use the existing engine
  static-effect machinery.
- Unconditional source-only color definitions receive all-zone defaults at
  construction. Explicit functional-zone restrictions remain authoritative.
  Ordinary temporary color grants use the host's normal functional zone: stack
  for instants/sorceries and battlefield for other objects.
- CDA ordering reuses `AbilityOrigin::is_rules_text`, including copied or replaced
  text and excluding ordinary granted origins. Filters identify the source by
  object identity; presentation of that reference does not constrain its type.
- Compiler and native definition builders add colors from typed printed color
  CDAs to the existing `rules_text_color_identity` metadata. Mana-derived color
  and actual color indicators remain unchanged. Native Commander deck legality
  already consumes this metadata through `Card::color_identity`.

No card name, displayed text, label, or Debug representation selects execution.
No compiled-artifact shape, replay mechanism, or coverage ledger is changed.
The live temporary-origin model gains an optional acquisition timestamp, omitted
when absent; this is included in the coordinated compatibility review. Compiled
metadata and zone-default values change, so artifact/cache invalidation belongs
at the campaign's coordinated compatibility boundary.

## Authored witnesses (unrun)

- Local grammar: complete colorless/all-colors predicates; unsupported tails;
  source-only versus all-Sliver permanent filters.
- Core policy: all-zone source defaults, battlefield ordinary defaults,
  conditional exclusions, and a restricted source filter.
- Tools strict pipeline: all four complete frozen bodies, without supplying
  source `colors` or `color_identity` as color indicators.
- Compiler/runtime: independent direct compilation and serialized artifact
  roundtrip/materialization; canonical render and independent complete-body
  reparse with runtime scope checks; all nine zones; unchanged mana color versus current
  color and Commander identity; Ghostfire's real paid casting, stack color,
  player/creature damage and spell copy; Sphinx's current-color targeting and
  actual blocking with Flying; all-controller and noncreature Slivers, current
  subtype changes, controller change, source phasing and departure; older and
  later color setters, permanent copies, ability loss and text-box replacement;
  native printed versus ordinary granted origins; distinct genuine Devoid;
  renamed source statements and rejected tails.
- Native web-session deck validator: full frozen bodies rejected outside their
  Commander identity and accepted within it; a native typed color definition
  contributes without Oracle text or a color indicator.

These scenarios are source evidence only. They have not established passing
compilation, execution, or end-to-end replay results.

## Grant acquisition ownership

The source review found that direct static generation previously reused the host
object's timestamp for ordinary temporary grants. An older recipient granted a
color definition after a red-setting effect could therefore remain red.

Live `GameState` registration now allocates an immutable acquisition timestamp
from the existing per-game continuous-effect clock. `TemporaryAbilityOrigin`
retains it through native clones, identity comparisons, expiry and component
merges. Direct static effects use the later of host and acquisition timestamps
(CR 613.7a); regeneration never allocates a new timestamp. Acquisition order is retained
when a newer host timestamp ties multiple grants, including after component
merges reorder storage. The two live grant
owners are the temporary-through-turn and incarnation-grant paths. Pre-entry
object assembly registrations, including Blitz/Dash and selected-land entry
riders, retain their entering-object timestamp fallback.

Authored, unrun witnesses cover an older white host, a red setter, a later blue
ability grant, and a still-later green setter; refresh stability, independent
native clone chronology, identity/payload continuity, expiry and merge retention
and face-down host timestamp refresh are checked. Stack-to-battlefield grants
retain their identity while using the newer entry timestamp; later zone changes
remove the grant. Additional native producer witnesses grant the compiled color
abilities from Ghostfire, Transguild Courier and Sphinx of the Guildpact to an
older recipient through the public payload-grant API. No serialized gameplay
recovery path is introduced.

## Spell copies and ordinary grants

The same review found that `Object::spell_copy_of` retained finite temporary
grants while excluding indefinite ones. Both are applied effects, outside the
spell's copiable layer-one definition. Spell copies now start with an empty
temporary-grant store; their printed/copied definition and casting choices are
preserved through the existing owners. A direct/artifact Ghostfire witness
announces its complete spell, grants it blue until end of turn, copies it, and
expects a blue original plus a colorless copy with both damage programs resolving.
Native Dash/Blitz copy guards exercise printed riders and reconstruction from
copied alternative casting choices after the temporary store is cleared. These
additional scenarios remain unrun.
