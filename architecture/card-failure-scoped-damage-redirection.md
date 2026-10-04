# Scoped all-damage redirection

Status: source-authored, UNVALIDATED. No build, compilation, test, or corpus replay
has been run for this batch. The cohort contains nine source-proposed identities
and seven partial timed identities with a known resync gap. No measured recovery
or runtime closure is claimed.

## Exact frozen cohort

The full Oracle bodies and identities are retained in
`fixtures/scoped_damage_redirection.json.fixture`:

- Empyrial Archangel, Harsh Judgment, Martyrs of Korlis
- Pariah, Pariah's Shield, Protector of the Crown, Treacherous Link
- Veteran Bodyguard, Weathered Bodyguards
The following seven timed identities remain partial, excluded from source-complete
coverage until the checkpoint/replay gap below closes:

- Ascent of the Worthy, Karona's Zealot, Kjeldoran Royal Guard
- Mirror Strike, Shimian Night Stalker, Sivvi's Valor, Turn the Tables

With Great Power and Heroic Sacrifice are adjacent secondary-body candidates,
not included in this count. Optional redirection, bounded next-N/next-time shared
damage shields, and clauses choosing a source need their own complete decision
or simultaneous-allocation model and are not broadened into this grammar.

## Typed model and execution

The new persistent payload retains source and recipient filters, a combat flag,
a live untapped-source gate, and distinct destinations: the ability source, its
currently attached permanent, or the original damaged permanent's controller.
The old source-and-other-permanents shape retains its existing representation.
The existing Harsh Judgment source-controller replacement remains its own shape.

The all-damage instruction model gains an optional typed scope. This distinguishes
an announced source target from a live source filter, an announced protected target
from a live protected set, and fixed controller/source destinations from event-source
controller destinations. It retains an explicit cleanup or next-controller-turn
lifetime. Legacy artifacts default to their existing shape; materialization and
rendering preserve the complete new scope. Multiple independent target slots are
explicitly rejected here rather than silently using only one target.

The runtime uses actual Redirect replacement actions and the existing CR 616
ordering/application-identity driver. It does not rewrite the damage source,
amount, combat classification, or unpreventability flag and emits no prevention
event. Source characteristics use a live object when available, otherwise the
matching event-source LKI, including phase-out. A captured source target keeps
its exact incarnation. A static restriction host must still be active; that host
lifetime is separate from the damage source's LKI.

Both original and replacement recipients are checked at application. An absent,
phased-out, or no-longer-damageable permanent cannot redirect its damage elsewhere;
a missing/invalid destination likewise leaves the proposed recipient unchanged.
Player departure is checked. Attached destinations and recipient controllers are
read live. Shroud and hexproof do not prohibit a non-targeting redirection.

These distinctions follow [CR 109.2 and 614.9](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf).
Redirection can change the recipient of unpreventable damage; it is not prevention.

## Declaration boundary

The former `attacking && !blocked` test called creatures unblocked too early.
Combat state now records completion of the whole blocker declaration, including
cost payment and zero-blocker declarations. Validation-only candidate states do
not set that boundary. The ordinary and shared-player turn owners set it after
all declarations finish. A new combat resets it.

Sync checkpoints retain this bit. Older checkpoints with saved runner state can
recover the boundary from that authoritative state, not from a phase-name guess.
Without either marker, completion is not assumed. Synthetic post-declaration
fixtures explicitly mark their state. This preserves the CR 509.1h distinction
between attacking, blocked, and unblocked while blocking costs are being paid.

## Open recovery boundary

Native runtime savepoints clone the replacement manager and retain these timed
registrations. The current Wasm sync checkpoint has no replacement/prevention
manager wire encoding; `try_build_sync_checkpoint` can export a state without
those registrations. An exported/reimported checkpoint must not be treated as a
lossless recovery of a timed redirect. This inherited gap also affects earlier
runtime-created prevention/replacement shields. A verified replay-only recovery
path or complete typed encoding, plus fail-closed checkpoint handling, needs
separate source review. The seven timed identities above remain partial until
that path is closed. The new combat-declaration field is encoded, but that alone
does not solve replacement-manager recovery.

## Authored evidence, all unrun

- Four grammar scenarios cover persistent scopes, all-damage duration/source
  shapes, and rejection of trailing/optional/shared-budget clauses.
- Fifteen runtime scenarios compile full frozen bodies and both direct/restored
  artifacts, then exercise static and attached controller changes, phase/leave,
  artifact source LKI, untapped gates, combat-only and post-declaration scopes,
  announced targets, real paid Royal Guard/Night Stalker activations, Karona's
  actual face-up trigger, and all three Ascent chapters.
- Actual damage checks preserve the original source's lifelink and distinguish
  redirection from prevention, including an application-identity cycle.
- Native zero-blocker/extra-combat and Wasm checkpoint scenarios retain the
  declaration boundary through current and legacy-shaped checkpoints.

The source review and these authored expectations do not replace the deferred
full build, regression comparison, corpus replay, or live multiplayer checks.
