# Basic-land characteristic changes

Status: UNVALIDATED source proposal. Six exact frozen full cards are proposed:
Elsewhere Flask, Terraformer, Navigator's Compass, Tundra Kavu, Gaea's Liege,
and Graceful Antelope. The exact fixture also retains Orcish Farmer as partial.
No builds, compiler probes or tests ran.

## Native rule boundary

CR305.7 (pinned September2026 rules) preserves card types, supertypes and unrelated
subtype families when basic land types are set. It removes rules-text and copy
abilities, supplies intrinsic mana for the new land type, and preserves abilities
granted by other effects. The former resolving basic-land effect incorrectly
used SetAbilities at ordinary layer6 timestamp order, potentially erasing earlier
external grants. An appended native RemoveLandRulesTextAbilities modification
now uses the same pre-grant ordering and intrinsic-mana restoration as the
existing static land-type rules owner. It maps through native retained state,
all characteristic evaluators, dependency analysis and relevant derived caches.
It is not a fabricated static-ability origin or a display label.

SetSubtypes already replaces only the stated subtype family. The new native
owner retains it and uses the explicit land-rules-text loss only when replacing.
In-addition mode instead adds the new subtype and preserves old land types and
all existing abilities. Intrinsic basic-land mana is derived by the normal layer
boundary. A restricted alternatives list offers precisely its basic types;
malformed kinds/selections fail explicitly. New core fields have serde defaults.

## Complete grammar and ownership

The strict basic-land template reader keeps `Plains or Island` as one resolution-
time choice rather than giving both types, and keeps `in addition` as a typed
flag. AST, lowering and rendering carry both facts. A single fixed alternative
needs no spurious prompt.

The existing complete choice/become pair reader already handled basic land types,
but its procedure-opening guard admitted only creature-type choices. The guard
now also admits the exact basic-land phrase, so `Choose a basic land type. Each
land you control becomes that type` remains one choosing owner over the entire
resolution set. Sacrificing the source as a cost does not prevent this choice.

A strict unquoted suffix `until this <source> leaves the battlefield` retains the
existing exact source-lifetime duration. Quoted inner text and incomplete tails
are rejected. No target-controller next-untap duration is approximated.

## Authored scenarios, all unrun

Three native properties cover prior/later external grants, printed ability loss,
old/new intrinsic mana, unrelated creature subtypes/card types/supertypes,
addition mode, exact alternative options and invalid input. Full direct/artifact
six-card tests cover real sacrifice/tap/mana payments, one shared choice, fixed
recipient sets excluding later lands, complete ETB life/draw, cleanup, opponent
targets, Gaea's attacking/nonattacking Forest quantities, source departure before
resolution, exact-incarnation blink, and phasing not ending a leaves duration.
Strict tools payload and grammar properties are also authored.

Orcish Farmer remains partial: `until its controller's next untap step` needs an
explicit beginning boundary with the correct target-controller ownership. The
existing ControllersNextUntapStep also serves restrictions lasting during that
step; this patch does not conflate the two. Snow and chosen-color families remain
separate. Deferred validation must include prior supported basic-land cards and
native/Wasm retained-state behavior for the appended scalar modification.

## Bounded review corrections

The appended native variant also invalidates prohibition tracking and participates
in grouped-effect discovery. Native option answers must contain exactly one
in-range index; empty, duplicate and multi-selection answers fail before changes.

The review identified pre-layer registered grants (including mana-spend haste)
that were not ordinary later continuous grants. Land rules-text removal now
retains their exact paired origins, rebuilds its static cache and adds intrinsic
mana. Copy/text-base replacements likewise retain those independent occurrences;
copied text and copy exceptions keep their early Effect origins and are removed
by land-type replacement. Ordinary layer6 RemoveAllAbilities/SetAbilities still
clear these grants. The shared dependency evaluator uses the same removal owner.
Additional unrun scenarios cover direct registered origins, removal of a printed
cant restriction with restoration after cleanup, copied text versus a registered
grant, and subsequent ordinary ability loss.
