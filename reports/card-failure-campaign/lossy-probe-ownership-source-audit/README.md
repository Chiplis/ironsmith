# Speculative copular probe diagnostic ownership — source proposal

Baseline: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
Status: SOURCE-ONLY / UNVALIDATED. No added source-credit, measured recoveries,
builds, tests, parser probes, corpus runs, or code generation.

The new conditional copular reader asks the legacy sized-animation reader
whether it owns a body. That check never commits its AST. The legacy reader
can record suffix recovery while reading a subject, then decline the body
when the predicate is not a size. Capture only that speculative call's losses;
leave its Result/error behavior and the eventual owner's diagnostics intact.

Production change is confined to that call in
`crates/ironsmith-compiler-grammar/src/keyword_static/costs_replacements_and_permissions.rs`.
Five source-authored tests in `copular_probe_loss_tests.rs` cover failed-probe
loss isolation, nesting, retained real suffix loss, typed pregame ownership,
and delegation to the established sized-animation owner. They have NOT run.

Preserved evidence: 94 exact IDs, partitioned into 18 disjoint authored-grammar
families; 17 opening-hand battlefield candidates are the strongest coherent
witness family. The 88 newly gate-failing strict-lossy entries are NOT asserted
to be new gameplay regressions. Four losses predate this baseline and two
entries previously failed compilation. The diagnostic route does not prove
all other whole-body semantics correct.

- `lossy-source-audit.md`: source call chain, runtime ownership, whole-body
  obligations, compatibility and deferred validation requirements.
- `lossy-source-audit-families.md`: complete exact-ID family membership.
- `lossy-source-audit-exact-ids.json`: full unchanged-input Oracle/compiled
  bodies, loss diagnostics, prior categories and source evidence hashes.

The strict loss detector, capture/replay API, gate, runtime, and chosen-type
and conditional-untap implementation files are unchanged by this proposal.
