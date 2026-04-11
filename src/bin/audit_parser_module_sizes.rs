use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy)]
struct Budget {
    path: &'static str,
    max_lines: usize,
}

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let budgets = [
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/mod.rs",
            max_lines: 900,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/activated_sentence_parsers.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/keyword_activated_lines.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/activated_line_core.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/activation_costs.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/activation_restriction_clauses.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/keyword_action_costs.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/trigger_clause_core.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/trigger_subject_filters.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/families/activation_and_restrictions/choice_object_clauses.rs",
            max_lines: 2500,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/mod.rs",
            max_lines: 800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/player_relations.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/naming_and_reference.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/reference_tag_stage.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/spell_filters.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/predicate_phrases.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/front_end/grammar/filters/meld_and_special_subjects.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/mod.rs",
            max_lines: 900,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/choice_damage_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/registry.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/counter_marker_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/token_copy_control_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/combat_and_damage_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/delayed_step_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/sentence_primitives/mechanic_marker_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/mod.rs",
            max_lines: 700,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/resource_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/combat_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/zone_move_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/counter_stat_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/mod.rs",
            max_lines: 650,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/labeled_prefixes.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/sentences/effect_sentences/dispatch_inner/unsupported_shape_diagnostics.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/lowering/lower/mod.rs",
            max_lines: 500,
        },
        Budget {
            path: "src/cards/builders/compiler/lowering/lower/rewrite_text_helpers.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/lowering/lower/rewrite_sentence_grouping.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/lowering/lower/damage_and_cost_rewrites.rs",
            max_lines: 1200,
        },
        Budget {
            path: "src/cards/builders/compiler/lowering/lower/modal_and_level_lowering.rs",
            max_lines: 1200,
        },
    ];

    let mut failures = Vec::new();
    for budget in budgets {
        let path = repo_root.join(budget.path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        let line_count = source.lines().count();
        println!(
            "{}: {} lines (budget {})",
            budget.path, line_count, budget.max_lines
        );
        if line_count > budget.max_lines {
            failures.push((budget.path, line_count, budget.max_lines));
        }
    }

    if !failures.is_empty() {
        eprintln!("\nBudget failures:");
        for (path, line_count, max_lines) in failures {
            eprintln!("  {path}: {line_count} > {max_lines}");
        }
        std::process::exit(1);
    }
}
