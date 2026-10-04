# Replacement and prevention recovery boundary

Status: source-authored, UNVALIDATED. No build, compilation, test, browser scenario,
or corpus replay was run. This closes a known source omission through fail-closed
wire snapshots and exact replay/runtime branches; it is not executed evidence and
does not by itself promote any card in the source matrix.

## Exact known affected cohort (non-exhaustive)

Previously proposed temporary prevention, all ten:
Al-abara's Carpet; Azorius Ploy; Energy Arc; Ethereal Haze; Fleeting Flight;
Hidden Retreat; Pack Leader; Repel the Abominable; Shieldmage Elder; Soul Parry.
Their complete frozen bodies are in `fixtures/temporary_damage_prevention.json.fixture`.
The native prevention instructions call `effects/combat/prevention_helpers.rs::register_prevention_shield`,
which installs `PreventionEffectManager` state.

Previously proposed temporary damage multiplication, exactly three:
Blind Fury; Goblin Goliath; Quest for Pure Flame.
`RegisterDamageMultiplierEffect` delegates to `ApplyReplacementEffect` and
`add_until_end_of_turn_effect`. The other five identities in that fixture use
regenerated static replacements and are not implicated by this registration gap.

New partial timed redirection, seven:
Ascent of the Worthy; Karona's Zealot; Kjeldoran Royal Guard; Mirror Strike;
Shimian Night Stalker; Sivvi's Valor; Turn the Tables.
`RedirectAllDamageThisTurnToTargetEffect` installs cleanup/next-turn replacements.

These are twenty distinct identities. Static prevention, persistent life-gain,
static token/counter replacement and static damage multiplication are not
blanket-added to this inventory. Other runtime-created rules state may have its
own carrier gaps; this inventory is explicitly not exhaustive.

## Lossless-or-replay checkpoint contract

The former sync checkpoint omitted replacement/prevention managers, while import
constructed a fresh GameState. A signed public-board hash could not detect those
missing future effects. Runtime savepoints already retain the real managers.

Sync checkpoints now require explicit replacement and prevention completeness
carriers. Empty executable sets retain allocation counters, consumed-shield
metrics, turn anchors and validated manager metadata. Active executable
registrations, prevention shields, follow-up programs, replacement scopes,
nonzero prevention deferral depth and pending replacement decisions refuse
export. They are not replaced with null programs or neutral successful results.
Imports missing either carrier, forged unit programs or unencoded deferral work
are rejected transactionally before replacing the current game. Static ability
replacements remain regenerated from their compiled payloads.

`isReplayCheckpointBoundary` also rejects these states. More importantly, a
host can lie about an empty manager: a host signature authenticates its assertion
but does not prove that the actual accepted state contained no omitted effects.
`PublicAuditCheckpoint` does not commit to complete replacement/prevention state.
Verified resync therefore no longer captures, imports or trusts host gameplay
checkpoints, even when an older host supplies a signed checkpoint sequence. It
always replays every accepted action from the authenticated accepted genesis.
The checkpoint shortcut cannot be re-enabled until complete rules-state proof
exists. The cost is a longer replay on large transcripts, not missing effects.

## Verified replay and hidden metadata

The narrow metadata API and replay-only carrier are adapted from upstream
`2c6fc93258aab06df9b778d707f0a36293179771`. The exact carrier source is
`web/ui/src/lib/resync-checkpoint-carrier.js`, upstream blob
`22be5ab2b9e33fcc5a5e6a77480c453130689bd8`. This patch ports the relevant
metadata/replay hunks, not that entire upstream revision or unrelated payment
changes.

Verified replay-only always sends a signed null checkpoint and no checkpoint
sequence; it never downgrades to Trusted mode. The receiver still checks the pinned mode,
local transcript prefix, accepted match/genesis, signed roster and genesis,
shuffle/opening proofs, signed action log, current host/signature, envelope match
identity, action/final-state hashes and final public checkpoint hash. A supplied
legacy checkpoint cannot skip this full verified replay. Validation rollback uses
native savepoints; successful or failed replay releases its retained handle.

`getHiddenCardState` is a local committed-state metadata query. It cannot be
imported as a recovery checkpoint and grants no reveal permission. Hydration,
opening authorization, audit replay and command-identity rebinding use it without
invoking an unsupported executable-state serializer. Existing proof checks and
hidden-position/origin binding remain in their owning paths.

## Exact UI analysis fallback

Cross-worker priority, payment-options and target-preview capture can encounter
an unexportable active shield. Such failures now select a native runtime branch,
never a reduced checkpoint. Each branch call enters and leaves inside one
serialized worker queue task. Native exchange retains the GameState, pending
continuations, analysis jobs, trigger queues and gameplay ID cursors. The visible
runtime is restored before every yield, callback or next user action.

Priority and inspector queries retain one owned branch and use existing bounded
begin/step APIs, yielding between slices. Cancellation and disposal release the
branch; obsolete tokens cannot publish. Payment inventory executes a bounded
query on a branch; resource failures remain errors. Target previews create an
independent exact branch for each announced action, preserve casting-method
selection and restore state even on an exception. All routes recheck the exact
visible identity/session generation before capture or publication. Runtime
replacement no longer makes the cross-worker serializer a legality oracle.

An optional checkpoint backup is allowed to fail when a valid native restore
point already exists. A destroyed engine with no wire backup still requires
accepted-transcript recovery; it is not reported as restored.

## Authored evidence, all unrun

- Native Wasm scenarios reject active redirect/prevention export, retain both in
  a runtime savepoint, preserve empty-manager allocator gaps and exhausted-shield
  metrics, and reject missing/forged carriers without changing the live game.
- Metadata's upstream Earthbend scenario verifies a committed-zone identity read
  despite an executable continuous effect that cannot be exported.
- Carrier/envelope scenarios reject a replay-only checkpoint, claimed sequence,
  unsupported mode, unsigned envelope, changed match, action log and checkpoint.
- PeerJS reconnect/host-takeover scenario refuses checkpoint export while
  replaying signed accepted actions and retaining independent runtime savepoints.
- Local analysis scenarios cover priority/inspector slices, per-action target
  isolation, native gameplay ID cursors, cancellation during acquisition and
  after a yield, resource-error propagation, and optional wire-backup failure.

The final shared-code review and deferred native/Wasm/browser execution remain
mandatory. No measured recovery count changes are asserted here.


## Registered restriction follow-up

The same exact-local-versus-wire boundary also applies to
`EffectStore.restriction_effects`. `ApplyRestrictionEffect` records typed
restrictions with captured source/controller, reference snapshots, start and
expiry state. The existing checkpoint has no carrier for that collection. A
concrete earlier proposed identity is Display of Dominance, whose temporary
source-filtered targeting rule uses this owner; this is not an exhaustive
inventory of temporary restrictions.

The exporter and replay-boundary predicate now also reject any retained runtime
restriction. The wire carries an explicit empty-restriction field, required and
validated before import. Local priority/payment/target analysis uses the already
implemented exact runtime fallback, and Verified resync continues to replay from
genesis regardless of any host emptiness claim. A native savepoint retains the
restriction normally. An unrun native scenario and the missing-carrier import
scenario cover this additional owner. No gameplay prohibition is implemented by
this certificate: the actual native restriction continues to enforce the rule.
