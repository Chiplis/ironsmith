#!/usr/bin/env python3
"""Static ownership review queue; scan signals are not completion evidence."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "crates/ironsmith-engine/src"
OUTPUT = ROOT / "reports/audits/semantic-action-ownership-2026-10-05.json"


def mask(source):
    pattern = r'(?s)/\*.*?\*/|//[^\n]*|r(#+)?".*?"\1|"(?:\\.|[^"\\])*"'
    return re.sub(pattern, lambda match: re.sub(r"[^\n]", " ", match[0]), source)


def production(source):
    masked = mask(source)
    pattern = (
        r'#\[cfg\((?:test|ironsmith_runtime_parser_tests)\)\]\s*'
        r'(?:#\[[^\]]*\]\s*)*(?:pub\s+)?mod\s+\w+\s*\{'
        r'|#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?fn\s+\w+[^;{]*\{'
    )
    for match in list(re.finditer(pattern, masked)):
        start = masked.find("{", match.start(), match.end())
        if start < 0:
            continue  # An enclosing test module was already masked.
        end, depth = start + 1, 1
        while end < len(masked) and depth:
            depth += (masked[end] == "{") - (masked[end] == "}")
            end += 1
        masked = masked[:match.start()] + re.sub(r"[^\n]", " ", masked[match.start():end]) + masked[end:]
    return masked


def scalar_projection_facade(source, masked, impl_start, impl_end):
    """Recognize only a whole scalar body delegating and projecting its packet.

    This does not validate the retained executor or any caller. A scalar body
    with additional statements stays in the direct-call review queue.
    """
    match = re.search(r"\bfn\s+execute\s*\(", masked[impl_start:impl_end])
    if match is None:
        return None
    start = impl_start + match.start()
    opening = masked.find("{", start, impl_end)
    if opening < 0:
        return None
    end, depth = opening + 1, 1
    while end < impl_end and depth:
        depth += (masked[end] == "{") - (masked[end] == "}")
        end += 1
    body = re.sub(r"\s+", "", masked[opening:end])
    pattern = (
        r"\{(?:return)?self\.execute_with_outputs\(game,ctx,?\)"
        r"\.map\((?:crate::effects::)?CompletedEffectOutputs::into_outcome,?\);?\}"
    )
    if depth or not re.fullmatch(pattern, body):
        return None
    return {
        "start": start,
        "end": end,
        "line": source.count("\n", 0, start) + 1,
        "source_sha256": hashlib.sha256(source[start:end].encode()).hexdigest(),
        "delegated_to": "execute_with_outputs",
        "projection": "CompletedEffectOutputs::into_outcome",
    }


patterns = {
    "child_gateway": r"execute_effect(?:_with_outputs)?\(|\.execute_child(?:_with_outputs)?\(",
    "keyword_completion": r"\b(?:complete_keyword_action(?:_with_result|_with_outputs)?|publish_keyword_action_completion(?:_receipt|_with_result|_with_outputs)?)\(",
    "direct_executor_call": r"\.(?:execute|execute_with_outputs)\(\s*game\s*,",
    "movement_receipt": r"apply_zone_change|finish_zone_change_receipts|take_zone_change_results|PreparedZoneMove",
    "replacement_processing": r"process_trait_event|commit_prepared|prepare_.*scoped",
    "state_commit": r"game\.(?:move_object\w*|shuffle\w*library\w*|move_library_card\w*|set_player_library_order\w*|tap|untap|add_\w*counters\w*|remove_\w*counters\w*|gain_life|lose_life)\(",
    "public_reveal": r"CardRevealedEvent::|mark_hidden_cards_publicly_revealed",
    "private_view": r"view_cards\(",
    "simultaneous": r"simultaneous|Simultaneous",
}
executors, files = [], {}
for path in sorted(BASE.rglob("*.rs")):
    if "tests" in path.stem or "tests" in path.parts:
        continue
    source = path.read_text()
    masked = production(source)
    matches = list(re.finditer(r"\bimpl\s+(?:crate::effects::)?EffectExecutor\s+for\s+([\w:]+)", masked))
    if not matches:
        continue
    relative = str(path.relative_to(ROOT))
    files[relative] = hashlib.sha256(source.encode()).hexdigest()
    for match in matches:
        start = masked.find("{", match.end())
        end, depth = start + 1, 1
        while end < len(masked) and depth:
            depth += (masked[end] == "{") - (masked[end] == "}")
            end += 1
        body = masked[start:end]
        facade = scalar_projection_facade(source, masked, start, end)
        signals = {
            name: [
                source.count("\n", 0, start + hit.start()) + 1
                for hit in re.finditer(pattern, body)
                if not (
                    name == "direct_executor_call"
                    and facade is not None
                    and facade["start"] <= start + hit.start() < facade["end"]
                )
            ]
            for name, pattern in patterns.items()
        }
        executors.append({
            "effect": match[1], "path": relative,
            "line": source.count("\n", 0, match.start()) + 1,
            "status": "Full semantic ownership review pending",
            "signals": signals,
            "scalar_projection_facade": (
                {key: value for key, value in facade.items() if key not in ("start", "end")}
                if facade is not None else None
            ),
        })

# Review low-level commits in helpers as well as executors. These are narrow
# source-boundary checks, not proofs of action semantics or runtime behavior.
owner_rules = {
    "reflexive_occurrence": (r"next_reflexive_trigger_id\s*\+=", "composition/reflexive_trigger.rs"),
    "completed_action_receipt": (r"\.mark_action_completed\(", "action_observation.rs"),
    "optional_once_per_turn_acceptance": (r"game\.record_do_this_action\(", "composition/may.rs"),
    "marked_damage_removal": (r"game\.(?:set_damage_marked|clear_damage)\(", "damage/remove_marked_damage.rs"),
    "life_total_change": (r"game\.(?:gain_life|lose_life)\(", "life/life_change.rs"),
    "shuffle": (r"game\.shuffle\w*library\w*\(", "cards/shuffle_library.rs"),
    "library_position": (r"game\.(?:move_library_card\w*|set_player_library_order\w*)\(", "cards/library_arrangement.rs"),
    "player_counter_removal": (r"game\.remove_player_counters_with_source\(", "counters/remove_counters.rs"),
    "object_counter_removal": (r"game\.remove_counters\(", "counters/remove_counters.rs"),
    "public_reveal_observation": (r"CardRevealedEvent::new\(", "cards/reveal.rs"),
    "permanent_designation": (r"game\.set_(?:monstrous|renowned)\(", "permanents/designation.rs"),
    "permanent_tap": (r"game\.tap\(", "permanents/tap.rs"),
    "land_play_observation": (r"LandPlayedEvent::new\(", "zones/land_play.rs"),
    "land_play_history": (r"player\.record_land_play\(", "zones/land_play.rs"),
    "battlefield_entry_observation": (r"EnterBattlefieldEvent::(?:new|tapped)\(", "zones/entry_observation.rs"),
    "numerical_die_completion": (r"DieRolledEvent::new_with_natural_result\(", "player/die_roll_transaction.rs"),
}
# A constructor can represent a prospective query rather than publication.
# Only exact source-reviewed function bodies are exempted. A changed body
# returns to the candidate queue until it receives a fresh semantic review.
prospective_review_path = ROOT / "reports/audits/ownership-work/entry-prospective-query-boundary-review-2026-10-08.json"
prospective_reviews = (
    json.loads(prospective_review_path.read_text())["entries"]
    if prospective_review_path.exists() else []
)


def prospective_entry_query(path, source, masked, position):
    relative = str(path.relative_to(ROOT))
    for review in prospective_reviews:
        if review["path"] != relative:
            continue
        marker = "fn " + review["function"] + "("
        start = masked.find(marker)
        if start < 0:
            continue
        opening = masked.find("{", start)
        if opening < 0:
            continue
        end, depth = opening + 1, 1
        while end < len(masked) and depth:
            depth += (masked[end] == "{") - (masked[end] == "}")
            end += 1
        if start <= position < end and hashlib.sha256(source[start:end].encode()).hexdigest() == review["source_sha256"]:
            return {
                "classification": "prospective_query",
                "function": review["function"],
                "review": str(prospective_review_path.relative_to(ROOT)),
                "function_sha256": review["source_sha256"],
                "reason": review["reason"],
            }
    return None


prospective_entry_query_sites = []
scanned_effect_sources = {}
commit_sites = {name: [] for name in owner_rules}
for path in sorted((BASE / "effects").rglob("*.rs")):
    if "tests" in path.stem or "tests" in path.parts:
        continue
    source = path.read_text()
    masked = production(source)
    relative = str(path.relative_to(BASE / "effects"))
    scanned_effect_sources[str(path.relative_to(ROOT))] = hashlib.sha256(source.encode()).hexdigest()
    for name, (pattern, owner) in owner_rules.items():
        for match in re.finditer(pattern, masked):
            if name == "battlefield_entry_observation":
                query = prospective_entry_query(path, source, masked, match.start())
                if query is not None:
                    prospective_entry_query_sites.append({
                        "path": str(path.relative_to(ROOT)),
                        "line": source.count("\n", 0, match.start()) + 1,
                        **query,
                    })
                    continue
            commit_sites[name].append({
                "path": str(path.relative_to(ROOT)),
                "line": source.count("\n", 0, match.start()) + 1,
                "is_declared_owner": relative == owner,
            })


# Root actions can bypass effects even when the effect-only screen is clean.
# Benchmark setup is not a game action. This narrow screen covers explicit
# game.tap calls, not self calls, aliases, function pointers or full semantics.
root_tap_commit_sites = []
for path in sorted(BASE.rglob("*.rs")):
    if "tests" in path.stem or "tests" in path.parts or path.name == "bench_support.rs":
        continue
    source = path.read_text()
    for match in re.finditer(r"\bgame\.tap\(", production(source)):
        root_tap_commit_sites.append({
            "path": str(path.relative_to(ROOT)),
            "line": source.count("\n", 0, match.start()) + 1,
            "is_declared_owner": path == BASE / "effects/permanents/tap.rs",
        })

engine_source_fingerprint = hashlib.sha256("".join(
    str(path.relative_to(ROOT)) + hashlib.sha256(path.read_bytes()).hexdigest()
    for path in sorted(BASE.parent.rglob("*.rs"))
).encode()).hexdigest()
validation_path = OUTPUT.with_name("semantic-action-ownership-validation-2026-10-05.json")
checked_validation = json.loads(validation_path.read_text()) if validation_path.exists() else {}
previous_report = json.loads(OUTPUT.read_text()) if OUTPUT.exists() else {}

report = {
    "objective": "One owner per semantic action; compound effects preserve timing, simultaneity, costs and action identity.",
    "complete": False,
    "validation": {
        "reference": "semantic-action-ownership-validation-2026-10-05.json",
        "matches_checked_engine_source": (
            checked_validation.get("current_source_certified_for_compilation") is True
            and checked_validation.get("engine_source_fingerprint") == engine_source_fingerprint
        ),
        "engine_source_fingerprint": engine_source_fingerprint,
        "fingerprint_scope": "All Rust files under crates/ironsmith-engine; repository-relative path plus file SHA256 in sorted order.",
        "previous_checks": checked_validation.get("checks", []),
        "pending_checks": checked_validation.get("pending_checks", []),
        "gameplay_verified": False,
        "limitations": "Compile checks and static ownership signals do not prove full semantic completion.",
    },
    "method": "Static concrete EffectExecutor inventory with whole-body scalar projection facade classification and exact-source reviewed prospective entry queries. All effect-source files are fingerprinted, including helpers. Generic implementations, aliases, callbacks, native paths and semantic timing still require review. Signals are candidates, not defect verdicts.",
    "requirements": [
        "Identify each action's proposal, replacement, commit, observations and completion owner.",
        "Compound adapters delegate semantic commits and retain complete child outcomes.",
        "Capture original identities and snapshots before deferred additions.",
        "Preserve cost validation, original-result projection and semantic action events.",
        "Preserve batching, decision suspension, rollback and context restoration.",
        "Verify every executor and helper family; compiler success alone does not prove completion.",
    ],
    "current_progress": previous_report.get("current_progress", []),
    "next_reviews": previous_report.get("next_reviews", []),
    "intentional_direct_dispatch": previous_report.get("intentional_direct_dispatch", []),
    "root_tap_commit_sites": root_tap_commit_sites,
    "ownership_signals": {
        name: {
            "declared_owner": "crates/ironsmith-engine/src/effects/" + owner,
            "commit_sites": commit_sites[name],
            "outside_owner": [site for site in commit_sites[name] if not site["is_declared_owner"]],
            "scope": "Effect source only; completion timing, proposals and callers still require semantic review.",
        } for name, (_, owner) in owner_rules.items()
    },
    "prospective_entry_query_sites": prospective_entry_query_sites,
    "scanned_effect_sources_sha256": scanned_effect_sources,
    "executor_count": len(executors), "executors": executors, "source_sha256": files,
}
OUTPUT.write_text(json.dumps(report, indent=2) + "\n")
print(f"{len(executors)} concrete executor implementations inventoried across {len(files)} files; completion remains unproven.")
