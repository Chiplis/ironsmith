# Scoped Waterbend payments (source reconstruction, unvalidated)

Frozen failure membership remains the e8740178 baseline and the
2026-10-03 corpus. The payment family contains exactly these six baseline
failures: Benevolent River Spirit, Crashing Wave, Foggy Swamp Visions,
Water Whip, Waterbender's Restoration and Waterbending Lesson. Katara,
Water Tribe's Hope is an already-supported regression/migration scenario.
Avatar Aang, Hama and Secret of Bloodbending have other body failures and
are outside this count.

This is a new reconstruction from e722bc5bd. Lost commits dacb4dc87 and
dabeafd91 are not present locally and their old reviews are not evidence
for this work. Whole-body scenarios and fresh independent review remain
required before any of the six can be added to proposed source coverage.
No builds, tests, compiler probes, formatters or corpus executions have run.

The owner is ManaCost's typed WaterbendPaymentScope. It retains original
obligations separately from an algebraic capacity: additions sum independently
priced scopes, each generic rewrite limits the existing capacity, and bound
X survives every subsequent rewrite. Ordinary generic additions never restore
a previously reduced Waterbend share. Zero retains an obligation, so payment
must be accepted and produces an explicit Waterbend completion event even
when the accepted amount is zero.

The shared payment planner selects exact Waterbend resources alongside other
alternatives and mana activations. Eligible resources are current untapped,
non-phased-out artifacts or creatures controlled by the payer. A resource is
spent at most once and exact choices are revalidated after mana activations.
Completion is emitted once per obligation after the whole mana/life price
succeeds. Mandatory spell additions join the spell's price. Fixed and X
activations, optional additions, Ward and UnlessPays use the same owner.

Direct Cost::mana/ManaPaymentCost payment now encloses manual activations in
one resource scope and one checkpoint. Cancellation, pending decisions and
errors restore sources, mana, queued receipts and speculative disclosures;
resource-limit errors retain their typed cause. Incoming execution and
replacement context is passed through unchanged.

The pinned upstream Manabrew protocol 2.0.0 lacks a Waterbend resource kind.
The local version-3 protocol sources are the exact pinned upstream crate plus
the typed discriminant and explicit package metadata. Upstream source SHA,
license, and bounded modifications are recorded in vendor/manabrew-protocol/
UPSTREAM.md. The adapter exports/checks version 3 at match setup and uses
Waterbend for use/release actions; it does not assign meaning to action labels.

The restored full-body fixture and 27 authored native/public regression groups
are in `crates/ironsmith-tools/tests/waterbend_payment_bodies.rs`. Each full-card
scenario independently compiles unchanged source through direct and serialized
artifact/materialization routes. Older fixed-card shape/reporting fixtures now
use the same scoped owner. Additional core algebra, free-X, overflow, resource
transaction, Manabrew schema/use/release and native UI contracts are authored.

Fresh independent source review cleared the six complete frozen bodies and
Katara migration through `0f91bf03a` on 2026-10-05, after re-reading the final
composition, constrained-X coexistence and Assist deltas. This is source
clearance only. The frozen coverage matrix is intentionally unchanged for the
coordinator's deduplicated integration; no measured recovery is claimed.

## Caller and dependency boundary

Only ironsmith-web-session declares the Manabrew protocol dependency. Its
WASM bindings, prompt validation and output DTOs all use that same path
package; protocol::PROTOCOL_VERSION derives from its package major (3).
The npm split facade forwards the match configuration unchanged and rejects
mismatches before its artifact registration step. The npm usage example now
checks the version and supplies protocolVersion: 3 explicitly.

The standard browser worker, peer-lobby setup, native audit replay and trusted
relay replay call startMatch rather than startManabrewMatch. Their independent
native/audit protocol is unchanged. The optional Manabrew adapter requires a
host and peers that implement version 3's typed Waterbend UseResource and
ReleaseResource actions. Version 2 and absent versions are explicitly
rejected, not relabeled as compatible. This is source contract evidence, not
an executed end-to-end interoperability result.

Fresh source review corrections append the retained scope after existing
ManaCost fields, separate ancestor activation exclusions from tap reservations,
and reserve a later independent {T} in direct and pending payment owners.
Free-choice effect X and activation/spell X inventories count Waterbend
resources at a small positive X before exact checked planning; multiple X
symbols cannot overflow the optimistic inventory probe. Arithmetic failure in
a retained obligation or capacity is IncompleteEvidence at the checked planner
boundary and again at receipt emission, never accepted zero or a skipped event.

Planned activation execution now forwards the complete ancestor activation
exclusion chain and the same alternative tap reservations used in planner
simulation/preparation. Manual inventory, planned execution and the foreground
mana-ability continuation keep these domains separate. Direct cost entry points
validate retained Waterbend quantities before expanding X into pips.

A final public-algebra review found independently bound, still-unexpanded X
components could otherwise be expanded using one later request X. Each bound
component is now expanded before composition; a separate retained raw-pip
binding tracks only the remaining X pips. Original obligations and constrained-X
allocation metadata retain their own declarations. Bound+unbound (either side),
differently bound components, later binding/composition and reduction-followed-
by-addition contracts are authored without execution.

## Final source-review handoff

Production/test series starts at `9ff335f9d` on base `a07ead1cb` and ends at
`0f91bf03a`. The final documentation-only checkpoint records that review.
There are 95 changed paths, 31 of them the pinned protocol source, license
and attribution. All commits use per-command Codex identity. No publication
or external repository write was performed by this worker.

Frozen proposed identities:

- Benevolent River Spirit: `12d91743-1ccb-4f2b-b2a9-1c2b34f7c8eb`
- Crashing Wave: `c8d3936f-4c08-4ffe-83c9-7ed26fb8f9cd`
- Foggy Swamp Visions: `16af0a22-833e-47c6-ac27-002702194595`
- Water Whip: `2015f455-988e-444c-93df-23fab179ef87`
- Waterbender's Restoration: `285046f6-b3c4-4eb7-8712-9dffebabc762`
- Waterbending Lesson: `3d28886c-46bf-4d2e-a257-cb9016cc3573`

Katara, Water Tribe's Hope (`234fb291-0b62-4092-9071-81311c71bd53`) stays
outside that count. The later permitted validation phase must build the final
integrated stack, execute authored native/artifact/schema/UI contracts, replay
the unchanged frozen corpus and investigate every supported-card regression.
