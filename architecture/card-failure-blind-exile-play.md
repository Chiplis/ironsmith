> Historical source-owner record, preserved for the additive prepared-main port.
> Current corrections and compatibility inventory: [prepared-main blind exile port](card-failure-blind-exile-prepared-port.md).

# Blind exile play: native contract increment

Source-authored and UNRUN. This increment has zero card credit. Rogue Class
remains partial until native, WASM/transport, disclosure, replay and full-body
review all clear. No build, compiler invocation, compiler probe, test, formatter,
engine execution, corpus execution, or baseline regeneration was performed.

Base: `e4e9a2823f1b2614362cdecb58c94a8f05f6f8e5`, after the typed Class owner.
This commit is the native API contract for the dependent adapter work. New enum
arms require the WASM/UI partner's exhaustive mappings before union admission;
this increment is not an independently publishable application boundary.

## Rules and failed announcements

[CR 406.3a](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf)
puts the face-up transition immediately before announcing an unqualified play
from face-down exile. CR 406.3b and 601.3f require inspection for permissions
qualified by spell characteristics. An all-card permission does not grant
private inspection. CR 733.1 reverses an illegal or uncompletable play and its
payments; CR 733.2 leaves priority with the player. These rules do not authorize
leaking the hidden face through prospective menus or treating an illegal spell
or land play as successful merely because its identity was opened.

The native transaction therefore first validates only exact opaque card/grant
authority, opens the face publicly, then determines available ordinary play
methods. If no spell or land play is available, no play is announced, no mana,
land allowance or permission use is consumed, and the player keeps priority.
An exact pre-opening game/trigger-queue receipt reverses the physical attempt
on that terminal unavailable path, including restoring face-down status. Only
the publicly learned identity of the exact exile incarnation is retained for
all players. No successful cast/play or synthetic rules event is manufactured.
A technical failure after opening retains its after-opening continuation and
native authority for recovery. An offered valid
play choice is mandatory; payment cancellation cannot turn opening into an
optional private preview. Native rollback returns to that public choice, not to
the unknown face. The adapter must retain public disclosure even when a failed
command restores an earlier accepted prefix, using its existing commitment and
retry discipline. Physical rollback and learned/public knowledge are separate owners. CR406.3a's
exception for a card being cast face down is not implemented by forcing a public
opening: this mandatory-opening menu excludes FaceDownPlayFrom. Already inspected
cards retain their existing no-opening face-down route. Unseen face-down casting has a separate uniform intent and explicit public rule declaration, described below; it never uses this public-opening menu.

## Native API

- `LegalAction::OpenExiledCardForPlay { card_id, incarnation, permission: GrantSelection }` is
  appended. It contains no land/spell, face, cost, alternative-method, or title
  field. Its presentation is simply “Play exiled card”.
- `alternative_cast::blind_play::{requires_opening, selections}` is public.
  Selection indices refer to the unqualified list, never the ordinary list
  filtered by the secret face. The native identity, source and index all bind.
- The unqualified grant collector accepts exact card targets, an unrestricted
  exile filter and the existing unmarked source-pool membership relation. It
  rejects face-name/characteristic qualifications before any hidden-face match.
  Existing legacy source pools keep their prior membership semantics; they do
  not receive a new typed definition/acquisition pair.
- Unknown face-down exile cards produce only these opaque actions. Ordinary
  spell/land/alternative proposals and unknown in-exile activated abilities are
  omitted. Priority spell announcement and direct land admission also require
  opening, independently of presentation. Existing resolving-effect authority
  remains separate from priority admission.
- `PriorityLoopState.pending_exile_play` stores the post-opening choice.
  `opened_exile_play` retains `PendingExilePlay { card_id, incarnation, player, permission,
  actions }` through the ordinary cast/land transaction. Both are native Clone
  carriers, including native root and lane savepoints.
- `PriorityResponse::ExilePlayChoice(usize)` consumes the existing generic
  SelectOptions response. Exact permission identity remains selected while
  normal/alternative spell faces or a land face are chosen after disclosure.
  A land consumes that same permission's budget, follow-up and entry riders.
- `has_opened_exile_play_receipt()` exposes the unreconstructable native receipt
  guard for wire/replay admission. Raw public snapshots are not native recovery.

## Dependent adapter obligations

The public action refs carry opaque `card_id`, the frozen public hidden-card incarnation, and the existing source/index grant selection. Match the complete ref, retaining the native
identity; do not accept an ordinal from a different filtered face menu. Its drag,
action kind and label must stay independent of the card's hidden characteristics.
The transport must verify/open that exact card publicly before executing the
accepted play-opening command. The native helper rejects a still-unmaterialized
placeholder rather than inventing a face.

Prefer `opened_exile_play`'s original card/player as the disclosure transaction
subject: ordinary casting changes the object's ID. Extend disclosure views to
this exact exiled source, including failed commands, and preserve them in native
root/inactive-lane savepoints and retry/recovery. General option commands resume
the native choice. No cancellation, stale reference or forged grant may open an
unrelated card. The new action requires the coordinator's upcoming current-peer
protocol gate; the published artifact7/checkpoint3/protocol20 boundary is not
reused as evidence. Historical signature verification stays separate.

Authored native witnesses cover equal pre-opening actions across land/spell,
prices, mana and land allowances (including a preceding face-qualified grant),
wrong authority with no disclosure, unavailable plays with no resource use,
land-entry failure/recovery after opening, exact spell payment, and retained
opening state. The full Rogue fixture also exercises the new-controller opaque action, ordinary
spell/land payment, unavailable openings with no consumed play, and wrong-card
or stale-grant non-disclosure on both direct and artifact materialization routes.
Adapter witnesses remain dependent work. All remain unrun.

No previously counted body was identified on this blind controller route:
Nightveil, Elder and Intellect produce face-up cards; Kheru and Colfenor include
current-reader inspection; Bane has an inspector and no play permission; Intet
fixes both inspection and play to its resolving controller. The only marked
resolving mana-grant production is the reviewed hand-exile/draw composition
(Elder, face up). The marked AnyColor static grammar currently covers Intellect
(face up) and the pending Rogue scope. No additional prior hold is inferred.


## No-reveal declaration and incarnation increment

The second appended native action is `CastExiledCardFaceDown { card_id,
incarnation: Option<u64>, permission: GrantSelection }`. Both intents are offered
uniformly for each unqualified authority, without inspecting its land/spell type,
printed casting rules, cost, affordability, or alternate face. A tracked card
without known incarnation history produces explicit incomplete evidence when
there is an eligible authority. Untracked native cards retain exact ObjectId
admission and cannot obtain cross-peer remapping from an absent witness.

The new action creates `pending_exile_face_down: PendingExileFaceDownCast` with
original card, incarnation, player, grant, public kind list, and optional frozen
`declared_kind`. The typed `SelectOptionsContext.exile_face_down_choice` prompt
offers Morph, Megamorph, Disguise, active public effect-permission sources, and
Cancel. It does not query which printed rule the secret card actually has.
`PriorityResponse::ExileFaceDownChoice(usize)` is its native response. The marker
is false on unrelated/nested choices. Cancel clears only this attempted cast and
provisional declaration without payment, opening, inspection, or gameplay events.

The admitted declaration captures a native exact authority in GameState; merely
registering a public kind claim cannot recreate it. Tracked known definitions
and placeholders both use the existing public claim and later authenticated
`CastFaceDown(kind)` or filter obligation. The chosen kind determines the generic
face-down face/cost, including disguise ward. The actual caster supplies an effect
permission, separately from the selected play-from grant. The ordinary exact
FaceDownPlayFrom owner captures mana constraints and performs payment. Untracked
native cards, lacking a cryptographic identity obligation, check only the explicitly
declared rule after intent. No route grants inspection or calls set_face_up.

An unaccepted failed declaration restores game, trigger queue and the pre-command kind state. It introduces no new local-only kind lock; an already accepted declaration keeps its prior kind during later payment recovery. `declared_exile_face_down` preserves the original
card/incarnation/player/grant/kind across the stack ObjectId transition through
ordinary completion. Native root/lane clones preserve both states. A rolled-back
payment returns to the same declaration prompt; Cancel remains available. The
GameState declaration is retired with its old object/claim on a zone change or
opaque-library retirement. Public snapshots cannot reconstruct these receipts.

Direct unseen-exile commands from the older source/claim-only shape cannot bypass
this intent or its frozen witness. Inspected cards and the existing hand/library
claim routes retain their ordinary owners. The adapter must migrate unseen entry
to the new intent and preserve original signed bytes, actor and deadline while
validating/remapping the frozen incarnation before any token/material release.
This union requires its own coordinated current-peer gate; version numbers remain
the coordinator's decision, separate from historical signature verification.

The shared face-down overlay now hides printed additional/optional costs and
stores them with its other restore fields, including learned identity hydration
and enters-as-copy restoration. Prospective targeting uses the chosen face's
intentional empty program instead of falling back to the secret printed program.
These corrections prevent known and placeholder tracked cards from disagreeing
through hidden printed costs/targets. All new declaration/payment/claim/rollback,
full Rogue direct/artifact, identity and restoration scenarios are source-authored
and unrun. Rogue remains uncounted pending native/adapter union review.


## Native review corrections

Pre-stack face construction, printed alternative lookup, exact permission receipt
capture and mana-cost lookup now admit the same public declaration before reading
the unseen card. The shared check uses only the actor, exact card incarnation,
public kind and immutable unqualified grant receipt; it never calls a face query
or recurses through `resolve_method`. The full method additionally retains the
selected reader identity/source before its ordinary face-local index is resolved.
Existing stack receipts remain validated against their captured casting origin.
A raw hidden kind claim is insufficient for direct legality or payment-policy
queries as well as announcement. Source-authored controls cover both unknown and
materialized tracked definitions, ordinary and face-down exact methods, valid
admitted declarations and captured stack receipts.

No-reveal intents also retain the exact original TriggerQueue in their native
continuation. Root payment cancellation restores this queue with the gameplay
checkpoint, while a nested mana-ability cancellation keeps its enclosing owner.
Final declaration cancellation preserves the original queue and clears the
receipt. The unrun regression manually activates a land through the accepted
payment response, verifies a new nonmana tap-for-mana trigger is queued, cancels
payment and the declaration, and proves the pre-existing trigger alone survives.
Native cloning retains this queue receipt through recovery; absent root evidence
is an explicit error. No adapter retained-kind or signed-attempt behavior changes.


An unaccepted no-reveal failure also restores the pre-declaration native kind.
It cannot add an unpublished lock that accepted-prefix replay would lack. The
source-authored recovery comparison applies the same later Morph declaration to
both a failed Disguise attempt's savepoint and the clean prefix, then compares
successful cast/payment and authenticated obligations. Successful suspended
responses retain the kind carried by their captured continuation, and previously
accepted kinds remain fixed across payment rollback. The adapter must likewise
restore the per-command savepoint without transferring an unaccepted failed kind.
