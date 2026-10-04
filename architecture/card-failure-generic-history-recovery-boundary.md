# Generic current-turn history recovery boundary

Status: source-authored, UNVALIDATED. No builds, tests, probes, browser scenarios,
or replay executions were run.

## Omission and safe ownership

The authoritative wire checkpoint reconstructs a fresh TurnHistory without its
committed or staged generic event records. Empty is therefore not an equivalent
state when damage, searches, entries, destruction causes, countering causes,
or combat blocking occurred. These facts may be queried later in the same
turn by a card that was not present when the history was recorded.

Wire export now refuses any nonempty generic event history, including every
independent Grand Melee lane. Both committed and staged records participate.
A required `genericTurnHistoryEmpty: true` carrier accompanies actual empty
main/lane histories. Import rejects missing or false carriers for every lane
before constructing/replacing the game, and the owning restore helpers validate
them independently. Replay-checkpoint admission shares the same guard. No
historical receipt is serialized with an incomplete characteristic snapshot,
private card identity, or invented neutral result.

Native RuntimeSavepoint retains the actual GameState and rich receipts. Full
accepted-transcript replay reconstructs all completed and staged observations.
Wire refusal changes recovery-anchor availability, not ordinary gameplay.

## Known direct consumers

- The 13 proposed current-turn damage-history cards in
  `fixtures/damage_history_quantities.json.fixture` (Impact Resonance remains
  separately partial because of its damage-occurrence identity boundary).
- Archive Trap: its controller's completed searches.
- Baloth Cage Trap, Permafrost Trap, Whiplash Trap: completed entry observations.
- Cobra Trap: qualifying opponent destruction cause.
- Summoning Trap: cast-spell countering cause.
- Fyndhorn Druid: the exact creature's blocking history.

This is a shared boundary, not a card-name gate or an exhaustive consumer list.

## Actual recovery and genesis paths reviewed

- `web/ui/src/hooks/peer-lobby/validation.js` initializes through `startMatch`
  with the accepted players, deck/hidden manifests, commanders, seed and opening
  hand inputs. The signed initial hash comes from `exportPublicAuditCheckpoint`,
  not `exportSyncCheckpoint`.
- `web/ui/src/lib/audit-replay.js::startAuditTranscriptReplayWithGame` likewise
  calls `startMatch(replayMatchConfig(match))`, then checks the initial public
  hash. Opening hands and pregame entries may therefore create real event
  records without blocking genesis production or replay initialization.
- `crypto-resync.js` selects `replayOnly = trusted || verified`; Verified
  recovery replays from the accepted signed genesis, and Trusted recovery uses
  its accepted configuration/transcript or a validated suffix on the existing
  lossless state. Neither requires a gameplay wire snapshot.
- `engine-restore-point.js` prefers native runtime savepoints. An optional wire
  backup may fail while the real native restore handle remains available.
- Analysis capture's existing refusal handler uses native branches for priority,
  inspector, payment and target previews; branch exchange is restored in
  `finally`, with existing identity/epoch publication guards.
- Opening hydration/authorization uses `getHiddenCardState`, a committed
  metadata getter independent of executable checkpoint export.

The public-audit and hidden-ledger owners do not invoke the history-export
refusal. The new Grand Melee completeness field is omitted from public audit
serialization, preserving its previous digest shape.

## Authored, unrun regressions

Wasm source tests cover actual damage and exact native-savepoint receipts,
legacy/false main carrier rejection without live-state mutation, a nonfocused
Grand Melee lane holding damage, unknown/false lane rejection, and real opening
draw/pregame zone entry followed by successful public-audit and metadata reads.

## Unrepresented queued and stacked programs

The same boundary also requires `pendingAbilityProgramsEmpty: true` on the
root checkpoint. Export refuses every ability stack entry in the main stack or
any Grand Melee stack, regardless of trigger event kind. It refuses main/lane
TriggerQueue entries and undelivered AbilityTriggered notifications, the
GameState deferred event and matched-entry owners, and pending reflexive
programs. New narrow complete-emptiness queries include the private notification
and reflexive owners without exposing their executable contents. Active or
suspended subgames also refuse, because their parent frames have no wire owner.

Import independently rejects populated ability entries in both wire stacks,
even if an empty-program claim was supplied. Missing/false claims reject before
reset. The existing first-target completeness carrier, captured-announcement
checks, payment/cancellation guards and delayed-registration codecs remain in
their respective owners. The new guard is additive.

Additional authored Wasm cases cover generic cast-trigger matches, entries
removed while their notifications remain, main and inactive host queues,
deferred entries/events, reflexive context after its parallel entry is removed,
native restore, missing program claims and forged populated root/lane stacks.

The day/night lifecycle producer also owns pending `as transforms` work in
TurnStore, deferring its completed transformation event until those programs
finish. A nonempty `pending_day_night_as_transforms` list in the main turn or
any Grand Melee lane is now an explicit unrepresented program owner. An authored
native-savepoint/inactive-lane case pins this boundary; it is unrun.
