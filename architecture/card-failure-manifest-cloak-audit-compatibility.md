# Manifest and cloak audit compatibility

This source reconstruction introduces public audit checkpoint version 3 and live
audit protocol 19. It was authored against pinned source
`69a946ec767deda59927d63f08dd17fabff470f3` after the previous workspace disappeared.
It does not establish that a previous implementation or commit was retained.

## Signed state

Every public object exports both `manifested` and `cloaked` as booleans, including
false values. Each field reads its exact game-state provenance. Cloak must not be
reported as manifest merely because both mechanics allow the same face-up action.
The known-object branch of hidden-zone commitments includes the same two fields.
The opaque hidden-card metadata branch continues to commit its existing public
position/identity anchors without exposing a private card name.

Checkpoint version 2 evidence used the old manifest representation. Adding a
default cloak flag, inferring one from ward, or replacing its version would alter
the evidence. The canonical checkpoint hash normalizer and hash domain therefore
remain unchanged. A v2 payload is hashed exactly as the historical payload shape;
v3's explicit fields are independently part of its hash.

## Compatibility boundary

- Live peer handshakes continue to compare protocol versions strictly through
  `PROTOCOL_VERSION = CURRENT_AUDIT_PROTOCOL_VERSION`, now 19.
- Historical protocols 14, 16, 17, and 18 remain supported by signed-transcript
  verification with `requireEngineReplay: false` and no replay callback. Signed
  genesis, action chains, commitments, and exported checkpoint hashes are still
  checked. This does not assert historical rules replay with today's engine.
- Passing a replay callback requests current-engine replay even when
  `requireEngineReplay` is false. Both transcript and match must carry numeric
  protocol 19. Old, missing, string-coerced, or mismatched versions fail before
  callbacks, savepoints, match initialization, hidden-card hydration, actions, or
  disclosure checks can use engine state.
- Full replay, step initialization, per-action replay, and engine disclosure
  checking require version 3 checkpoint exports. UI preparation and start also
  validate protocol before setting up the replay runtime.
- A per-action call requires a successful initialized session. The session is
  installed only after initialization, checkpoint comparison, and state export
  succeed; it is invalidated when an action fails. Full replay restores the
  caller through native runtime savepoints and restores prior replay history only
  when runtime restoration succeeds.

## Authored scenarios and verification status

The added Rust scenarios distinguish ordinary face-down, manifested, and cloaked
public-object evidence, and distinguish their known hidden-object commitment
roots. JavaScript scenarios cover protocol rejection before engine access,
explicit v3 exports, failed initialization, per-action session authorization,
native restoration, historical signature-only verification, unchanged canonical
v2 checkpoint hashing, and tampered historical checkpoint rejection. Existing
origin, private-epoch, disclosure, and restoration fixtures use explicit current
protocol and checkpoint versions. The browser hydration scenario now initializes
its replay session before calling per-action replay.

All scenarios are authored and unrun. No builds, compilers, test runners,
formatters, probes, or corpus execution were used for this reconstruction.
