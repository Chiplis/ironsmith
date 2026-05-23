Card: Ripples of Potential
Date: 2026-05-23

Status
- strict parse now succeeds
- latest live similarity from `compile_oracle_text --compare-text`: 0.8333

What was fixed
- Added a reusable parser bundle for:
  - "Proliferate, then choose any number of permanents you control ..."
  - "Those permanents phase out."
- Added proliferate lowering support for auto-tagging when object auto-tag flow is enabled.
- Added runtime proliferate affected-object reporting so tagged composition can consume proliferated permanent IDs.

Remaining blocker to >= 0.99
- The engine still lacks a stable reusable compiler/runtime path to reference exactly "objects that had a counter put on them this way" from proliferate inside the same spell line while also preserving the later explicit player choice set for phasing.
- Existing `it`-tag flow is overwritten by subsequent choice tagging and does not expose a stable card-text-level handle for this specific "this way" subset chain in a generic manner.

Why this is beyond single-card parser-only scope
- Correctly modeling this family requires a reusable capability to carry forward proliferate's chosen/affected permanent subset as a first-class referenceable set through subsequent effect selection clauses.
- A robust solution likely needs either:
  - explicit AST/lowering support for tagging keyword-action affected objects with stable follow-up references, or
  - generalized "this way" object-set threading for multi-step imperative bundles.

Next recommended engine work
- Implement a generic affected-object-set reference primitive for keyword actions (starting with proliferate) that can be consumed by subsequent choose/filter clauses in the same effect program.
