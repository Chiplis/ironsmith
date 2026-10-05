# Recovery after the cloud workspace replacement

On 2026-10-05, the campaign resumed after a preview-limit interruption. The previous cloud checkout and unpublished Git objects were absent. No implementation or validation ran during the interruption.

The verified main commit is `ced8df84fa9e2103c04dfdf54e98e3332ad9a6d0`. Its second parent is the exact published stage-54 tip, `986baa2ecd78763c27720d6820eaf08563c4c174`. All 54 published campaign stages are in main's ancestry. GitHub currently marks PR 766 merged while the remaining 53 stacked PRs still show open against their previous stack branches; those statuses do not mean their code is absent from main. New changes start from reconciled main and must preserve its integration fixes.

The recovered published source matrix contains 942 proposed unique cards (944 entries). The last unpublished local checkpoint, `150298d1a`, had 961 proposals; its additional 19 identities require reconstruction and fresh source review before their coverage is restored. They are five consumable next-play timing cards, six scoped Waterbend cards, three granted Flashback cards, and five dynamic bolster/mobilize cards. Their prior review findings are design evidence, not proof that the replacement checkout contains the fixes.

The frozen baseline and validation policy remain unchanged: 40 measured compile recoveries and 3,193 unresolved baseline identities at stage 07. Builds, compilation, tests and corpus replay remain deferred until source plausibly covers all or a majority of the remaining baseline. The user's integration commit reports its own native/WASM/UI checks and incomplete behavioral validation; those reports do not replace the campaign's frozen full-corpus measurement.

Partial work remains explicit: Font, nested/simultaneous draw replacements and cycling share unresolved replacement-program continuation boundaries; complete source-choice reference inventories still need queue/resolving owner coverage. Parked prerequisites and defensive rejection paths do not count as whole-card fixes.

Publish new reviewed logical source batches promptly as cumulative draft PRs. Never duplicate merged stage commits, merge PRs, or claim unexecuted validation.
