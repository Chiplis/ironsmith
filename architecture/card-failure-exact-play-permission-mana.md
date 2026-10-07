# Exact play-permission mana prerequisite

Status: source implementation and authored scenarios, UNVALIDATED. No builds,
compiler probes, tests, formatters or engine/corpus execution. Base:
098c0aec81116bcbd754ee91ca06fef2bc38ebe3. This packet earns zero card credit.
Intellect Devourer, Rogue Class and Elder Brain still require their complete
frozen bodies and independent source review.

## Selection, announcement and payment

GrantSpec and PlayFromConstraints carry an omitted-default ManaSpendMode.
A non-normal plain play permission produces an appended native ExactPermission
casting method. It selects an index in the complete grant list and also retains
GrantPermissionIdentity, source, player, exact origin and chosen face/method.
The authoritative physical card supplies the proposed face, including printed
alternative costs, linked faces, prototype/bestow and face-down casting rules.
Marked reader enumeration starts with each proposed face, then finds its grants.
A permission matching only an Aura or prototype face can therefore supply that
cast; known noneligibility on another face does not abort the menu.
A caller cannot substitute arbitrary provisional characteristics. Nested exact
or price wrappers are rejected; an independent AlternativePrice route already
has its own exact origin_permission and preserves that selection.

The menu, prospective cast/payment queries, native announcement, selected costs,
CR 601.2e legality and real mana payment carry the same choice. Before moving
the card, announcement freezes PlayPermissionReceipt onto the stack object.
It retains the beneficiary, exact exile ObjectId, origin method and constraints.
Payment can remove a provider without changing the announced permission. The
receipt must agree with the cast-origin snapshot and ordinary captured grant
identity/constraints. The authoritative payment boundary returns IncompleteEvidence directly,
including calls without an execution meter and with sufficient ordinary mana.
Read-only boolean queries decline that payment. Zone
changes and copies clear the receipt. Other payers do not inherit its mode.

Native admission independently rejects a source-only PlayFrom command when
only marked readers exist, including a manually supplied printed alternative
price index. The legacy chooser sees only Normal-mode plain grants. A valid
legacy reader on the same host remains usable, with its own ordinary rules.
AnyColor cannot pay a colorless pip; AnyType can. The generic casting payment
APIs and interactive additional-mana costs choose the casting policy by their
explicit payment reason, so a non-casting payment does not acquire the rider.

## Tagged producers and compatibility with existing readers

GrantPlayTaggedEffect has an omitted-false permission_bound_mana marker.
Marked plain exile grants require an exact existing producer tag, do not follow
stable-card reincarnations, and place conversion on the grant's constraints.
They create no independent global mana permission. Known-empty tags remain
empty; missing tags fail explicitly. This packet admits the ordinary turn/end-
step and for-as-long-as-exiled durations, with no library-top, source-counter,
source-controlled, source-presence, source-next-exile or alternative-price
combination. No grammar emits the marker or new mode yet.

Existing unmarked tagged producers keep their admitted behavior. Their old
separate mana permissions now retain the exact grant occurrences they came
from. A newly exact cast excludes riders from unrelated tagged grants while
retaining genuinely independent global mana-as-though rules. This attribution
does not repair old source-only readers: their source/stable-card union
limitation remains as inventoried below. No previously supported route is
newly rejected merely because it lacks the new marker.

## Native recovery, public actions and the integrated version boundary

The receipt and native CastingMethod are retained by GameState/Object,
PendingCast, StackEntry and Clone. RuntimeSavepoint captures, copies, restores
and exchanges root game/priority state, pending decision games, native replay
checkpoints, inactive Grand Melee host lanes and suspended subgame hosts.
There is no executable public SyncCheckpoint importer at this source boundary;
public audit data remains a projection and must not be treated as native state.

The appended public exact_permission action contains a recursive public origin
plus source/index selection. It never serializes the native immutable identity
or receipt. Cached legal actions preserve their native selection after complete
public-reference comparison; deferred/hidden actions regenerate a legal menu
and compare the whole reference. Native announcement then revalidates identity.
Face-down disclosure suppression, public claim checks, labels and grouping
follow the underlying origin. Old action JSON shapes remain unchanged.

This semantic action capability requires a current-peer/replay gate even though
old public action bytes still hash and verify identically. The coordinator will
publish one independently reviewed atomic boundary with intrinsic mana/text
provenance: provisional artifact 7 and audit 20, checkpoint 3 unchanged. The
version patch is owned separately by that prerequisite; this packet must not
be published with the old audit 19 current-peer gate. Historical signature
verification is distinct from admitting an old peer or replaying it under new
semantics. No claim is made that a default field alone supplies that gate.

Authored, unrun scenarios cover same-host distinct modes; menu/payment parity;
forged source-only commands with and without printed prices; legacy reader
compatibility; unrelated tagged versus independent global mana rules; exact
incarnation, stale index/identity, wrong face and known-empty/missing tags; selected Bestow and linked-face
Prototype, excluded-face menu recovery and forged opposite-face admission;
provider removal during a pending cast; real mana plan confirmation; missing
receipts; typed direct/artifact/default carrier preservation; root, pending,
replay, inactive-lane and suspended-host native recovery; old JSON/hash shape,
public selector tampering, deferred reconstruction and face-down privacy.
They are source scenarios, not execution evidence or complete-card coverage.

## Bounded accounting and legacy-owner inventory

fixtures/exact_play_permission_mana_inventory.json.fixture records a source-only
join of the immutable frozen card bodies with source-coverage.json at
96f0adead, retaining that ledger's SHA256 and the frozen dataset identity.
The lexical superset includes all 203 frozen bodies containing casting and
“mana of any” or “mana as though”; it deliberately includes production and
reminder-text false positives. Only four currently counted proposals appear:
Berta, Wise Extrapolator (715f0832-696c-4111-b418-d9b6da403159), Discreet Retreat
(0158634a-08a3-4eb8-8216-bf85161fd168), Progenitor's Icon
(747f6d0f-3c10-4fef-b7ae-f578c8964ab5), and Ronin, Shadow Stalker
(8ceb1acf-5ff3-4f1a-af04-9f07ad454526). Their complete bodies create mana; none
contains a tagged-exile casting conversion clause. This inventory identifies
no additional counted proposal requiring a hold. Gale's Redirection already
has partial_not_counted status, independently of this new gap.

The actual legacy owner is GrantPlayTaggedEffect::execute in
engine/effects/player/grant_play_tagged.rs. It records a separate
ManaSpendPermission::CastingSpellsWithStableIds, whose GameState consumer
requires exile or a spell cast from exile but checks neither the selected
GrantPermissionIdentity nor the particular exile incarnation. Its permanent
exile/control durations use u32::MAX for that independent mana permission.
Consequently a different grant, or a later exile incarnation of the same
physical card, can inherit conversion while that mana permission is retained.
This is not a claim of conversion for an ordinary cast from hand.

Source tracing reaches this owner from the UntilEndOfTurn and
ForAsLongAsExiled branches of compile_support/effect_dispatch/subject_verb_middle.rs
and the ForAsLongAsYouControlSource branch beside them. Frozen examples include
Stolen Strategy (6807167a-e290-4259-940e-3cbbfa75d0a1), Gonti, Lord of Luxury
(d5be0ff6-39d9-4f9f-a028-e62dc38463f2), Covetous Urge
(6f52008f-3365-4cf9-9cae-8e761d657902), Psychic Intrusion
(fdb10291-b5e0-4da8-a103-8e7ac7f8177b), and Taster of Wares
(020aae46-cdba-4c37-9c54-283727c09994). Existing Stolen Strategy and Gonti
full-body scenarios in engine/cards/builders/tests/shard_00.rs and shard_20.rs
also identify those routes; the permission grammar's exact persistent/control
clauses and lowering establish the other examples. These are source traces,
not newly executed compiler results. They are absent from this failed-card
ledger and must be treated as baseline-supported regression risks. Their old
unmarked routes cannot be deliberately rejected by the new admission owner.
