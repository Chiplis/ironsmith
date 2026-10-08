# Independent source review: speculative diagnostic ownership

Reviewed anchor: `39ad55d76d741a560d732de07536a11a74d5faa0` on `card-repair/lossy-probe-ownership`.
Baseline: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
Worktree: `/workspace/scratch/bc560e8d90ff/ironsmith-lossy-probe-ownership`.

## Result

No blocking source defect found. Suitable to retain as an unvalidated source proposal. No build, tests, parser execution, probes, corpus refresh, or code generation were performed. Read-only source inspection and `git diff --check` only; the latter produced no whitespace errors. Worktree was clean at the reviewed anchor. No source files were edited by this review.

## Production semantics

- `costs_replacements_and_permissions.rs:2701–2713`: the isolated call is solely an ownership probe. `Some` always causes this reader to decline, so no AST produced inside the capture is committed by this call site. `None` continues to the distinct conditional grammar. Discarding this report is therefore appropriate in both cases.
- `ironsmith-compiler-api/src/parse_loss.rs:73–82`: capture replaces the thread-local collector, collects only nested diagnostics, and restores the previous collector before returning or resuming a panic. Existing outer diagnostic ordering survives. The new `sized_addition?` retains the Result propagation shape; the current legacy reader itself converts its prefix/subject errors to `None`, unchanged by this patch.
- `costs_replacements_and_permissions.rs:2976–3079`: the legacy reader computes the subject before verifying the P/T predicate. `anthem_grant_lines.rs:2775–2819,2875` explains how an abandoned reader can emit suffix recovery. The actual suffix-record operation and full legacy owner are unchanged.
- `keyword_static/mod.rs:1885–1926`: registry candidates already isolate diagnostics and replay the selected report. The legacy owner remains registered at line 1531. A later actual legacy selection therefore keeps its real loss. No detector/gate relaxation or diagnostic-code filtering was added.
- `sentence_memo.rs:101–137`: cache hits replay stored loss; cache fills use observe, retaining their own loss copy. The change does not alter those APIs or stored reports. Source inspection found no memo bypass that would turn the speculative capture into permanent erasure of a later selected loss. Cold/warm execution remains deferred.
- The new conditional grammar's own parsing and losses after the probe are outside the new capture. Existing source-zone and attachment errors remain outside it as well.

## Five authored tests and API review

1. `copular_probe_loss_tests.rs:9–25`: adversarial legacy `None` plus suffix loss, followed by clean conditional decline. Establishes the exact ownership failure rather than simply checking an empty report.
2. Lines 28–40: nested outer-before/outer-after diagnostics and a later accepted suffix recovery; exact diagnostic order is asserted.
3. Lines 43–65: full pregame clauses through the enclosing static parser; asserts one typed `BeginOnBattlefield` owner, nonstarting-player and hand-exile fields, and counter presence/absence. The public `StaticAbility::pregame_action_kind` API exists; `PregameBeginOnBattlefieldSpec` fields match the assertions. This is complete pregame-clause coverage, not complete frozen whole-card coverage.
4. Lines 68–79: clean sized-animation probe deferral plus successful clean direct legacy ownership.
5. Lines 82–98: deliberately lossy sized-animation match is deferred cleanly by the probe, while direct committed legacy parsing must retain the suffix diagnostic. This specifically guards against genuine-loss laundering.

The test module is wired under `cfg(test)` at `costs_replacements_and_permissions.rs:8569–8571`. Imports, lexer return type, `ParseLossReport` APIs, AST variant, and pregame accessors are source-consistent. Compilation and execution remain unverified.

## Nonblocking coverage limitations

- The Gemstone assertion at test line 63 checks only that counters are nonempty. It does not prove exactly one luck counter. A stronger future assertion should compare the exact `(CounterType::Luck, 1)` vector before citing a verified luck-counter payload.
- The genuine-loss test calls the committed legacy reader directly. A future enclosing-dispatcher test would additionally guard selected-registry replay and ownership routing. Current production source preserves that routing, so this is a coverage gap rather than a discovered defect.
- No new cold/warm memo, panic-restoration, or explicit error-propagation test is included. Restoration and Result behavior are supported by source reasoning; execution evidence is still needed for a validated result.

## Evidence and credit boundary

The committed audit retains 94 exact-ID entries grouped into 18 authored-grammar families, with 17 opening-hand battlefield candidates singled out. Its README and audit explicitly separate these proposals from measured recoveries. No evidence in this patch establishes 17 recoveries, wholesale 94-card recovery, or whole-body semantic correctness. Keep the four preexisting lossy entries and other family-specific semantic risks out of any recovery claim pending direct and artifact-path validation of full frozen bodies.

Recommended next stage, only when execution is authorized: run the five focused tests, strengthen exact pregame payload coverage, exercise selected legacy loss through the full dispatcher, compare cold/warm paths, and validate full frozen 17-card bodies before any admission credit.
