# Parser postparse repair inventory

PR-30 classifies every semantic mutation that formerly ran after a typed
effect or line had already been recognized. Recognition now owns authored
syntax, reference resolution owns symbol identity, semantic validation owns
invalid combinations, and normalization owns representation only.

| Former repair site | Former responsibility | Owning phase after PR-30 | Disposition |
| --- | --- | --- | --- |
| `reconcile_dynamic_zone_change_group_token_creation`, `dynamic_zone_change_group_token_creation_from_authored_trigger`, `dynamic_static_ability_count_token_creation_from_authored_trigger`, `authored_dynamic_token_creation_from_trigger` | Reparse intact source and replace a parsed trigger body | Typed effect/trigger grammar | Deleted; the selected grammar result is authoritative |
| `reconcile_authored_correlated_trigger_programs`, `replace_triggered_effects`, `replace_trigger_spec`, `spell_or_activated_ability_x_cost_trigger_spec` repair call sites | Detect named multi-sentence shapes and replace effects or trigger kinds | Typed trigger grammar and document programs | Repair dispatcher and replacement helpers deleted; the reusable trigger-spec constructor remains grammar-owned |
| `reconcile_open_attraction_reminder`, `restore_copy_static_variant_source_display` | Recover presentation facts from raw text after parsing | Provenance recognition | Deleted; provenance must be emitted with the recognized node |
| `apply_spell_cast_mana_source_filter`, `apply_spell_cast_single_target_source_exclusion`, `apply_source_spell_cast_trigger_spec` and their recursive setters | Add filters or replace a trigger after line parsing | Trigger grammar and lexical reference resolution | Deleted |
| `bind_protected_battle_iteration_in_effects`, `bind_protected_battle_iteration_in_effect`, `bind_protected_battle_iteration_in_runtime`, `apply_protected_battle_iteration_surface` | Invent iterator bindings and rewrite runtime trigger effects | Scoped reference resolution | Deleted |
| `grammar_proven_named_explore_surface`, `reconcile_named_explore_source_surface` | Reparse source and retarget an Explore action | Typed source-reference grammar | Deleted |
| `exact_destroy_no_regeneration_statement`, `exact_hidden_partition_permission_statement`, `exact_historical_target_return_statement` | Give named statement recipes postparse precedence | Composable clause/document grammar | Deleted |
| `transport_delayed_copy_retarget_in_line` | Move a retarget effect into a delayed program after parsing | Explicit control-flow grammar | Deleted |
| `exact_dynamic_exile_permission_bundle`, `exact_looked_hand_optional_cast_bundle`, `exact_target_same_name_graveyard_may_cast_bundle`, `exact_atomic_return_as_aura_bundle`, `is_exact_correlated_trigger_effect_bundle`, `is_authored_dynamic_exile_permission_bundle`, `is_authored_look_hand_optional_cast_bundle` | Reparse or lexically probe complete trigger recipes | Typed permissions, document programs, and scoped references | Deleted |
| `preserve_triggered_effect_surfaces` | Reparse a body, compare semantic trees, and replace the chosen tree with source wrappers | Lossless document/sentence recognition | Deleted; source-boundary nodes are created during the initial parse |
| `activation_cost_sets_x_from_counter_removal`, `bind_event_amount_to_cost_x`, `bind_event_amounts_to_cost_x_in_effect`, `bind_event_amounts_to_cost_x` | Rewrite effect values from an already materialized activation cost | Typed values and activated-ability grammar | Deleted |
| `bind_activated_x_definition_to_mana_cost` | Replace an already parsed mana cost from a trailing X definition | Typed cost grammar | Deleted |
| `rewrite_self_replacements_as_conditionals`, `normalize_mana_replacement_effects` | Change replacement semantics after effect recognition | Explicit control-flow grammar | Deleted |
| `bind_typed_where_x_references`, `typed_where_x_binding`, `replace_bound_x_in_value`, `replace_bound_x_in_predicate` | Invent numeric bindings during AST normalization | Typed value/reference resolution | Deleted |
| `bind_removed_counter_damage_fanout`, `bind_until_next_turn_permissions_to_prior_exiled_collection` | Infer result and permission references during normalization | Typed grammar and scoped reference resolution | Deleted from normalization |
| `correlate_delegated_subsets_with_prior_target_collections`, `bind_choice_filter_to_collection`, `delegated_subset_collection_tag`, `bind_source_exile_to_collection_difference`, `bind_source_counter_to_latest_exiled_object`, `bind_other_target_to_collection_difference`, `append_to_conditional_false_branch` | Invent collection/subset/complement relationships | Selection grammar and scoped reference resolution | Deleted from normalization |
| `bind_all_players_subtype_choices_to_destroy_exclusion`, `bind_all_players_subtype_choices_to_return_inclusion`, `bind_quantified_choice_collections_to_destroy_followups`, `bind_explicit_chosen_object_followups`, `correlate_conditional_quantified_choice_followups`, `correlate_split_for_each_player_choice_complements`, `bind_counted_set_followups` and their tag/filter helpers | Retag selections and rewrite later consumers | Choice grammar and scoped reference resolution | Deleted from normalization |
| `normalize_singular_source_exiled_move`, `rewrite_repeat_process`, `rewrite_repeat_process_may`, `rewrite_repeat_process_once`, `rewrite_return_as_aura` | Change object movement or control-flow meaning during normalization | Object-action/control-flow grammar | Deleted from normalization |
| `is_noop_effect` and empty-grant pruning | Remove an executable AST variant based on inferred meaning | Semantic validation | Deleted from normalization |
| `normalize_nested_effects` | Recursively invoke the semantic repair chain | Canonical semantic visitor | Replaced by `for_each_nested_effect_vec_mut` recursion |

The remaining `normalize_effects_ast` operation is intentionally limited to
recursing over canonical effect vectors, flattening associative `Sequence`
nodes, and dropping empty `Sequence` wrappers. It consumes no tokens, text,
tags, or runtime objects and reaches a fixed point after one pass.
