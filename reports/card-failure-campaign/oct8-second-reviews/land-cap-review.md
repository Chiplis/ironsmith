# Independent source review: temporary additional-land caps

Reviewed commit c33c973f380ec80e38416f0d270376497b8e38fa against d65bd6564569a38132ae107cbe209af81c87f4f6 in ironsmith-temporary-land-caps on 2026-10-08. Source-only review. No builds, tests, executable compiler probes, corpus runs, code generation, formatting, source edits, or remote writes performed. Only source/data reads and this separate review artifact were produced.

## Verdict

No exact-source production blocker found. The three-line production change (two comments and one optional exact phrase parser) is narrowly consistent with the existing temporary permission owner and lowering. The authored gate APIs and Journey search representation are consistent with inspected source. This clears independent source inspection only: it does not establish that the new gates compile or pass, recover either exact ID, or qualify this surface under inherited artifact15.

## Frozen evidence

The retained baseline data at ../ironsmith-refresh-20261008-1dd81cd/reports/current-refresh-20261008/data/cards-current.json hashes to bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750. A read-only jq/sorted-JSON diff of the two exact oracle IDs against fixtures/temporary_additional_land_caps.json.fixture, removing only added `text`, was empty. Thus all preserved metadata, not only names or bodies, matches this source. The fixture-added full text retains original mana/type/body; the authored definitions helper asserts that concatenation. Summer Bloom is e5df4597-1647-4ac2-bdb3-a517598d1431, {1}{G}, Sorcery, three additional lands. Journey is 1c586d8a-9d1a-48a7-bb3e-9b2c0c329f8d, {2}{G}, Sorcery, the complete search/reveal/hand/shuffle first mode, two-additional-lands second mode, and Entwine {2}{G} with reminder text.

## Permission grammar and owner

- grammar/permission_facts/tagged_surface.rs:375-383 invokes probe_all, so the public typed fact parser requires total consumption.
- :864-902 consumes `play`, one optional exact `up to`, a mandatory nonempty count slice, exact additional land(s) this turn, sentence end, and a typed count whose used length must equal the entire count slice. Missing count, repeated introducer, malformed introducer, trailing command and recurring-duration forms retain their rejection boundaries.
- permission_helpers.rs:1801-1837 retains existing `you may` removal, typed count reparsing, implicit player, and EndOfTurn AST. Only the count tokens, without `up to`, reach that owner.
- effect_sentences/clause_dispatch/clause_dispatch_core/clause_readings/part_2.rs:43-78 explicitly interprets leading may plus recognized additional-land text as permission, avoiding a MayEffect resolution question.
- lowering_impl/compile_support/effect_dispatch/subject_verb_early.rs:2406-2418 preserves count, player-role resolution, and duration. There is no numeric resolution-choice effect or compulsory play action added.

## Runtime semantics

- engine effects/player/additional_land_plays.rs:28-47 resolves the affected player, records a Specific(player) restriction using the duration and source/controller, and refreshes derived restrictions. It does not play lands or reset used count.
- engine game_state/zones_and_characteristics.rs:5732-5740 starts each derived allowance at one; engine effect.rs:1409-1430 adds each applicable count with saturating addition. Multiple temporary grants stack while lands_played_this_turn is preserved.
- special_actions.rs:1330-1390 independently enforces active player, priority, unused allowance, main-phase/stack timing absent another applicable permission, and ordinary prohibitions. This grant cannot itself permit off-turn play.
- game_state.rs:7348-7355 removes current EndOfTurn restrictions. The authored expire helper explicitly refreshes afterward. It does not simulate an entire turn transition; retaining used count at cleanup is appropriately asserted separately.

## Journey complete body

The search representation expected by the test is correct for this body. subject_verb_middle.rs:3517-3536 uses SearchLibraryEffect only when count.max == Some(1), library-only, suitable destination, shuffle, and no aggregate/dynamic count. Journey has a maximum of two, so :3545-3635 constructs ChooseObjectsEffect with library binding, chooser and owner, optional-search mode, reveal, then a tagged per-object MoveToZoneEffect and ShuffleLibraryEffect. For destination Hand, shuffle follows moves. The authored recursive traversal can see the nested move. This is not a single-card SearchLibraryEffect API mismatch.

Search quantity parsing in grammar/effects/search_library.rs maps `up to` to optional count. ChooseObjects core fields used by the test exist (effect.rs:3536 onward). Runtime choose_objects_runtime.rs:1750-1795 marks selections public when reveal is required and asks the explicit hidden-zone choice; :2074-2083 publicly views nonempty revealed selections. Zero found cards still leaves the sequence's shuffle in place. The unchanged owner binding keeps searching and moving the controller's own basic lands; the opposite library and nonbasic witness guards are relevant.

Modal announcement is not deferred until resolution: priority_cast.rs:1348-1369 emits ModesContext; priority_mana.rs:4997-5026 explicitly converts that to SelectOptionsContext and decide_options, matching the authored Decisions implementation. Optional-cost selection uses minimum zero and one nonrepeatable Entwine option here (priority_cast.rs:2778-2810). Paying Entwine sets chosen_modes to all authored indices in ascending order (priority_mana.rs:1935-1941), yielding search then grant. Mana-only payment bypasses a singleton next-cost ordering menu (priority_cast.rs:4088-4110). Therefore the test's assumption that a positive-minimum option menu is Journey's two modes is compatible with these exact fixture bodies; it would be brittle if reused for spells with different optional costs, alternate-price menus or nonmana cost ordering.

## Authored gate review

- Direct/artifact: compile_to_runtime_definition and compile_to_artifact(name, text, false) signatures and artifact tuple agree with runtime/lib.rs:660-685. Encode/decode equality, validation, materialization, loss capture, metadata, no-unimplemented and no-May assertions are coherent source contracts. Dependency names are present in the crate manifest.
- Effect traversal: ResolutionProgram::all_effects returns top-level segment effects (core resolution_model.rs:270-281), so recursive visit_child_effects does not inherently double-count every nested grant. Inspected traversal matches intended structural assertions.
- Baker: the compile input shape is consistent with adjacent existing authored gates; both complete fixture bodies enter compile_artifact with score None, validate/materialize, and reject unimplemented output. The gate does not alter admission.
- Runtime: Summer Bloom checks used-before zero/one, zero-to-three optional plays, additive stacked grants, opponent isolation, off-turn rejection, total-cap rejection, expiration, and preserved used counts. Journey checks either mode and entwine, total mana three/six, selected mode indices/order, zero/one/two selections, nonbasic and opponent-library isolation, reveal count, hand count, allowance, over-cap rejection and expiration through both routes.
- Negative: bounded grammar rejects missing/repeated/partial introducers, foreign count tokens, each-turn and trailing-command near matches, and global actor text at this bounded entry. Three malformed full bodies are asserted to error via direct and artifact routes. These are authored expectations, not observed rejection evidence.

## Limits and suggested later verification

1. No compilation or execution was authorized; all green assertions remain unobserved. Run the complete direct/artifact/baker/runtime/negative gates only in an authorized execution phase.
2. Runtime shuffle execution is not instrumented: structural shuffle/order assertions plus unchanged lowering support it, but hand counts alone do not prove a shuffle actually occurred. A future shuffle-event witness would strengthen the test.
3. Runtime reveal records maximum public viewed count, not exact revealed identities or timing; selected-card identity and reveal-before-move event assertions would strengthen this without being necessary to diagnose the narrow parser change.
4. Journey runtime exhausts land permissions but does not independently sample declining all new land plays, prior used counts, or stacked Journey grants. Summer Bloom and the shared grant executor cover those dimensions structurally/through authored contracts, not executed evidence.
5. The search structural test checks membership for Basic and exact land types, not every filter field. Three legal basic witnesses plus nonbasic/opponent controls do not exhaust every accidental narrowing possibility. The preserved source path shows no new narrowing.
6. The menu discrimination is acceptable for these fixed bodies only; future reuse should discriminate descriptions/stages explicitly.
7. The literal actor/controller, timing, used-count and cleanup behavior comes from unchanged implementation; no new global each-player permission, enclosing Nahiri grammar, collateral family or supplemental-face recovery is established.
8. Only seven files differ from base; inherited artifact15 descriptors are untouched. A later exact-source boundary/admission update is required before promoting this source surface. Measured recoveries remain zero.

Repository status was clean at review end. The review artifact is outside the checkout.

## Follow-up independent review: 2f9b5c1cf0bcc44661d9d64e678c0e2c48bf1552

Reviewed against c33c973f380ec80e38416f0d270376497b8e38fa on 2026-10-08. Exact diff changes only the runtime test file and its source-review report. No production code, fixture data, admission, or artifact boundary changed. HEAD was the requested follow-up and checkout status remained clean. No builds/tests/probes/corpus/codegen or source edits were performed.

Verdict: no source blocker found in the follow-up. It directly addresses the earlier count-only reveal and unobserved runtime shuffle limitations with meaningful authored observations. This is source clearance, not a passing execution claim.

### API and callback validity

- StableId is publicly re-exported through engine ids.rs. HiddenInfoOperation is public at game_state.rs:400, with the exact LibraryShuffle field names and types destructured by the test (:412-420).
- set_random_seed, random_seed, irreversible_random_count, crypto_audit_checkpoint, and crypto_audit_operations_since are public at game_state.rs:6078-6104. The audit accessor returns an owned Vec of cloned operations, so iterating/filtering then destructuring its first element by reference is consistent with the API. The counters are u64 as expected by u64::from(searches).
- ViewCardsContext's viewer/subject/zone/public fields are public (decisions/context.rs:385-398).
- effects/helpers.rs:52-117 groups public reveal candidates by owner and zone in an ordered map, preserves each group's input order, and calls every player by ascending player index. This fixture has one A/library group, hence precisely A then B is appropriate. Empty selected groups have no callback. The helper records the public tag after both viewers.
- choose_objects_runtime.rs:2074-2083 invokes that public reveal before returning the selected tagged objects to the following move effects. Private initial library/candidate views do not enter the new callback assertions. SelectionRevealPolicy is frontend replay policy; it does not independently add a duplicate public view callback in this local DecisionMaker path.
- Runtime selection normalization sorts ordinary chosen IDs (choose_objects_runtime.rs:889-904, :1845-1860), so arbitrary user-submitted selection order is not a universal promise. The fixture selects the first one/two candidates from its unshuffled, incrementally created library; library enumeration preserves insertion order (:744-763), and creation appends (game_state.rs:8300). Thus selected and normalized orders coincide for this fixed matrix. A later randomized/reversed-choice extension should assert sets or normalized order unless ordering itself is the intended contract.
- Ordinary move handling creates a new ObjectId while retaining the prior object's stable_id (zones_and_characteristics.rs:1385-1390); find_object_by_stable_id is public and validates the current index at turns_and_tracking.rs:3329-3335. Tracking stable identity through library-to-hand moves is correct.

### Independence and strength of the shuffle witness

The reference is cloned before actual resolution and seeded there; it does not copy the actual post-shuffle order. RuntimeCacheState::clone explicitly copies RNG state and random-operation count into distinct Cells, transcript queues into new RefCells, and audit history into a new RefCell (game_state.rs:1316-1344). Advancing the clone therefore cannot advance or fabricate the actual game's audit/RNG state.

The only post-resolution datum used in reference setup is the set of IDs submitted by the test DecisionMaker, an intentional input choice, not a result inferred from actual shuffle output. `remaining` preserves the pre-resolution library order and removes precisely that submitted set. Actual audit input must equal it, actual output must equal both native seeded reference and final library, and the actual operation must have A as owner and exact random before/after counts. Final actual random count and state additionally prohibit unnoticed extra random operations. The grant-only branch asserts no shuffle, unchanged own/opponent libraries, and no random-count advance. Zero-result search still requires a typed shuffle audit record.

shuffle_player_library (game_state.rs:6230-6316) records randomness, takes the pre-order, randomizes the specified player's library, then records its actual post-order and counts. The reference's direct library assignment leaves selected objects in its object store, but the native shuffle only uses that player's ordered library vector here; there are no hidden placeholders, verified epochs, transcript overrides or fixtures that would make stale unused objects affect the permutation. This reference is appropriate for the local deterministic matrix.

The oracle shares the existing native shuffle implementation deliberately. It independently verifies Journey's routing to that owner, exact shuffle boundary/input, one-call accounting and resulting native permutation; it does not independently certify the shuffle algorithm, statistical fairness, cryptographic replay behavior or a defect shared by native shuffle and its audit implementation. This is a limitation of scope, not a circular use of actual expected output.

### Identity and ordering assertions

The reveal callback now witnesses exact selected ObjectIds, A's ownership/library zone, empty hand and absence of any shuffle since resolution began. Post-resolution hand count plus selected stable-ID membership and every selected stable ID resolving to Hand establishes the exact selected hand set for the unique IDs supplied by the fixture. Remaining original objects must still be in Library; shuffle before_order/after_order comparison establishes exact ordered membership. The opponent's full ordered library is checked unchanged. These checks are materially stronger than maximum reveal or hand counts alone.

The previous report's suggested runtime shuffle/reveal strengthening is now addressed at the authored-test level. Earlier limits on execution, recovery credit, fixed-body announcement assumptions, broader adversarial/hidden replay coverage and artifact15 boundary remain unchanged. No measured recovery or executed whole-body pass is established.
