impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn build_action_view(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    index: usize,
    action: &LegalAction,
) -> ActionView {
    let (kind, object_id, ability_index, from_zone, mut to_zone) = action_drag_metadata(action);
    if let LegalAction::UsePregameAction {
        card_id,
        ability_index,
    } = action
        && matches!(
            pregame_action_kind(game, *card_id, *ability_index),
            Some(ironsmith::static_abilities::PregameActionKind::RevealFromOpeningHand(_))
        )
    {
        to_zone = None;
    }
    let source_visible = object_id
        .map(ObjectId::from_raw)
        .is_none_or(|id| object_visible_to_perspective(game, perspective, viewed_cards, id));
    ActionView {
        index,
        label: if source_visible {
            describe_action(game, action)
        } else {
            redacted_action_label(action)
        },
        kind: kind.to_string(),
        object_id: source_visible.then_some(object_id).flatten(),
        ability_index,
        from_zone: source_visible.then_some(from_zone).flatten(),
        to_zone: source_visible.then_some(to_zone).flatten(),
        action_ref: priority_action_ref(action),
    }
}

pub(super) fn build_untap_land_action_view(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    index: usize,
    stable_id: u64,
) -> Option<ActionView> {
    let object = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id).map(|obj| (*id, obj)))
        .find(|(_, obj)| obj.stable_id.0.0 == stable_id)?;
    let (object_id, object) = object;
    if !object.has_card_type(CardType::Land) || !game.is_tapped(object_id) {
        return None;
    }
    if !object_visible_to_perspective(game, perspective, viewed_cards, object_id) {
        return None;
    }

    Some(ActionView {
        index,
        label: format!("Untap {}", object.name),
        kind: "untap_land".to_string(),
        object_id: Some(object_id.0),
        ability_index: None,
        from_zone: Some(zone_name(Zone::Battlefield)),
        to_zone: Some(zone_name(Zone::Battlefield)),
        action_ref: PriorityActionRef::UntapLand { stable_id },
    })
}

pub(super) fn action_drag_metadata(
    action: &LegalAction,
) -> (
    &'static str,
    Option<u64>,
    Option<usize>,
    Option<String>,
    Option<String>,
) {
    match action {
        LegalAction::PassPriority => ("pass_priority", None, None, None, None),
        LegalAction::KeepOpeningHand => ("pass_priority", None, None, None, None),
        LegalAction::TakeMulligan => ("take_mulligan", None, None, None, None),
        LegalAction::ContinuePregame => ("pass_priority", None, None, None, None),
        LegalAction::BeginGame => ("pass_priority", None, None, None, None),
        LegalAction::UsePregameAction { card_id, .. } => (
            "use_pregame_action",
            Some(card_id.0),
            None,
            Some(zone_name(Zone::Hand)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::PlayLand { land_id } => (
            "play_land",
            Some(land_id.0),
            None,
            Some(zone_name(Zone::Hand)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            ..
        } => (
            "cast_spell",
            Some(spell_id.0),
            None,
            Some(zone_name(*from_zone)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => (
            "activate_ability",
            Some(source.0),
            Some(*ability_index),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => (
            "activate_mana_ability",
            Some(source.0),
            Some(*ability_index),
            Some(zone_name(Zone::Battlefield)),
            None,
        ),
        LegalAction::TurnFaceUp {
            creature_id,
            method: _,
        } => (
            "turn_face_up",
            Some(creature_id.0),
            None,
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::SpecialAction(action) => match action {
            ironsmith::special_actions::SpecialAction::PlayLand { card_id } => (
                "special_action",
                Some(card_id.0),
                None,
                Some(zone_name(Zone::Hand)),
                Some(zone_name(Zone::Battlefield)),
            ),
            ironsmith::special_actions::SpecialAction::TurnFaceUp { permanent_id, .. } => (
                "special_action",
                Some(permanent_id.0),
                None,
                Some(zone_name(Zone::Battlefield)),
                Some(zone_name(Zone::Battlefield)),
            ),
            ironsmith::special_actions::SpecialAction::Suspend { card_id }
            | ironsmith::special_actions::SpecialAction::Foretell { card_id }
            | ironsmith::special_actions::SpecialAction::Plot { card_id } => (
                "special_action",
                Some(card_id.0),
                None,
                Some(zone_name(Zone::Hand)),
                Some(zone_name(Zone::Exile)),
            ),
            ironsmith::special_actions::SpecialAction::ActivateManaAbility {
                permanent_id,
                ability_index,
            } => (
                "special_action",
                Some(permanent_id.0),
                Some(*ability_index),
                Some(zone_name(Zone::Battlefield)),
                None,
            ),
            ironsmith::special_actions::SpecialAction::UnlockRoomDoor { room_id } => (
                "special_action",
                Some(room_id.0),
                None,
                Some(zone_name(Zone::Battlefield)),
                Some(zone_name(Zone::Battlefield)),
            ),
        },
    }
}

pub(super) fn zone_name(zone: Zone) -> String {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command",
        Zone::OutsideGame => "outside_game",
    }
    .to_string()
}

fn pregame_action_kind(
    game: &GameState,
    card_id: ObjectId,
    ability_index: usize,
) -> Option<ironsmith::static_abilities::PregameActionKind> {
    let ability = game.object(card_id)?.abilities.get(ability_index)?;
    let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    static_ability.pregame_action_kind()
}

pub(super) fn describe_action(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass priority".to_string(),
        LegalAction::KeepOpeningHand => "Keep hand".to_string(),
        LegalAction::TakeMulligan => "Mulligan".to_string(),
        LegalAction::ContinuePregame | LegalAction::BeginGame => "Pregame".to_string(),
        LegalAction::UsePregameAction {
            card_id,
            ability_index,
        } => match pregame_action_kind(game, *card_id, *ability_index) {
            Some(ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(_)) => {
                format!("Begin with {}", object_name(game, *card_id))
            }
            Some(ironsmith::static_abilities::PregameActionKind::RevealFromOpeningHand(_)) => {
                format!("Reveal {}", object_name(game, *card_id))
            }
            Some(
                ironsmith::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount,
            ) => format!("Use {}", object_name(game, *card_id)),
            Some(ironsmith::static_abilities::PregameActionKind::ChooseColor) => {
                format!("Choose a color for {}", object_name(game, *card_id))
            }
            None => format!("Use {}", object_name(game, *card_id)),
        },
        LegalAction::PlayLand { land_id } => {
            let name = game.object(*land_id).map_or_else(
                || object_name(game, *land_id),
                |object| {
                    ironsmith::decision::linked_other_face_land_definition(game, object)
                        .map(|def| def.card.name)
                        .unwrap_or_else(|| object.name.to_string())
                },
            );
            format!("Play {}", name)
        }
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => {
            let mut name = object_name(game, *spell_id);
            let mut qualifiers = Vec::new();

            match casting_method {
                ironsmith::alternative_cast::CastingMethod::Normal => {
                    if *from_zone != Zone::Hand {
                        qualifiers.push(format!("from {}", zone_display_name(*from_zone)));
                    }
                }
                ironsmith::alternative_cast::CastingMethod::FaceDown => {
                    qualifiers.push("face down".to_string());
                }
                ironsmith::alternative_cast::CastingMethod::SplitOtherHalf => {
                    if let Some(obj) = game.object(*spell_id)
                        && let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        )
                    {
                        name = other_def.card.name.clone();
                    }
                    qualifiers.push("other half".to_string());
                }
                ironsmith::alternative_cast::CastingMethod::Fuse => {
                    qualifiers.push("fuse".to_string());
                }
                ironsmith::alternative_cast::CastingMethod::Alternative(index) => {
                    let method = game.object(*spell_id).and_then(|obj| {
                        obj.alternative_casts
                            .get(*index)
                            .map(|method| (obj, method))
                    });
                    if let Some((
                        obj,
                        ironsmith::alternative_cast::AlternativeCastingMethod::Disturb { .. },
                    )) = method
                        && let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        )
                    {
                        name = other_def.card.name.clone();
                    }
                    let method_name = method
                        .map(|(_, m)| m.name().to_ascii_lowercase())
                        .unwrap_or_else(|| format!("alternative #{index}"));
                    qualifiers.push(method_name);
                }
                ironsmith::alternative_cast::CastingMethod::GrantedEscape { .. } => {
                    qualifiers.push("escape".to_string());
                }
                ironsmith::alternative_cast::CastingMethod::GrantedFlashback => {
                    qualifiers.push("flashback".to_string());
                }
                ironsmith::alternative_cast::CastingMethod::PlayFrom {
                    zone,
                    use_alternative,
                    ..
                } => {
                    if let Some(index) = use_alternative {
                        let alt = game
                            .object(*spell_id)
                            .and_then(|obj| {
                                ironsmith::decision::resolve_play_from_alternative_method(
                                    game,
                                    game.turn.priority_player.unwrap_or(obj.owner),
                                    obj,
                                    *zone,
                                    *index,
                                )
                            })
                            .map(|m| m.name().to_ascii_lowercase())
                            .unwrap_or_else(|| format!("alternative #{index}"));
                        qualifiers.push(alt);
                    }
                    qualifiers.push(format!("from {}", zone_display_name(*zone)));
                }
                ironsmith::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
                    zone,
                    use_alternative,
                    ..
                } => {
                    if let Some(other_def) = game.object(*spell_id).and_then(|obj| {
                        game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        )
                    }) {
                        name = other_def.card.name.clone();
                    }
                    let alt = game
                        .object(*spell_id)
                        .and_then(|obj| {
                            ironsmith::decision::resolve_play_from_alternative_method(
                                game,
                                game.turn.priority_player.unwrap_or(obj.owner),
                                obj,
                                *zone,
                                *use_alternative,
                            )
                        })
                        .map(|m| m.name().to_ascii_lowercase())
                        .unwrap_or_else(|| format!("alternative #{use_alternative}"));
                    qualifiers.push(alt);
                    qualifiers.push(format!("from {}", zone_display_name(*zone)));
                }
            }

            if qualifiers.is_empty() {
                format!("Cast {}", name)
            } else {
                format!("Cast {} ({})", name, qualifiers.join(", "))
            }
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            let name = object_name(game, *source);
            let ability_text = game
                .current_ability(*source, *ability_index)
                .and_then(|ability| {
                    stack_display_lines_from_abilities(std::slice::from_ref(&ability), false)
                        .into_iter()
                        .next()
                })
                .map(|text| normalize_action_text(&text));
            match ability_text {
                Some(text) => format!("Activate {}: {}", name, text),
                None => format!("Activate {} ability #{}", name, ability_index + 1),
            }
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            let name = object_name(game, *source);
            let ability_text = game
                .current_ability(*source, *ability_index)
                .and_then(|ability| {
                    stack_display_lines_from_abilities(std::slice::from_ref(&ability), false)
                        .into_iter()
                        .next()
                })
                .map(|text| normalize_action_text(&text));
            match ability_text {
                Some(text) => format!("Activate {}: {}", name, text),
                None => format!(
                    "Activate mana ability on {} (# {})",
                    name,
                    ability_index + 1
                ),
            }
        }
        LegalAction::TurnFaceUp {
            creature_id,
            method,
        } => {
            let cost_prefix =
                ironsmith::special_actions::turn_face_up_cost_display(game, *creature_id, *method)
                    .map(|cost| format!("{cost}: "))
                    .unwrap_or_default();
            format!(
                "{cost_prefix}Turn this face-down permanent face up. ({})",
                object_name(game, *creature_id)
            )
        }
        LegalAction::SpecialAction(action) => match action {
            ironsmith::special_actions::SpecialAction::PlayLand { card_id } => {
                format!("Play {}", object_name(game, *card_id))
            }
            ironsmith::special_actions::SpecialAction::TurnFaceUp {
                permanent_id,
                method,
            } => {
                let cost_prefix = ironsmith::special_actions::turn_face_up_cost_display(
                    game,
                    *permanent_id,
                    *method,
                )
                .map(|cost| format!("{cost}: "))
                .unwrap_or_default();
                format!(
                    "{cost_prefix}Turn this face-down permanent face up. ({})",
                    object_name(game, *permanent_id)
                )
            }
            ironsmith::special_actions::SpecialAction::Suspend { card_id } => {
                format!("Suspend {}", object_name(game, *card_id))
            }
            ironsmith::special_actions::SpecialAction::Foretell { card_id } => {
                format!("Foretell {}", object_name(game, *card_id))
            }
            ironsmith::special_actions::SpecialAction::Plot { card_id } => {
                format!("Plot {}", object_name(game, *card_id))
            }
            ironsmith::special_actions::SpecialAction::ActivateManaAbility {
                permanent_id, ..
            } => {
                format!(
                    "Activate mana ability on {}",
                    object_name(game, *permanent_id)
                )
            }
            ironsmith::special_actions::SpecialAction::UnlockRoomDoor { room_id } => {
                format!("Unlock {}", object_name(game, *room_id))
            }
        },
    }
}

pub(super) fn zone_display_name(zone: Zone) -> &'static str {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command zone",
        Zone::OutsideGame => "outside the game",
    }
}

pub(super) fn object_name(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|o| o.name.to_string())
        .unwrap_or_else(|| format!("Object#{}", id.0))
}

pub(super) fn hidden_object_label() -> String {
    "Hidden card".to_string()
}

pub(super) const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;

pub(super) fn redacted_choice_id(index: usize) -> u64 {
    JS_SAFE_INTEGER_MAX.saturating_sub(index as u64)
}

pub(super) fn decision_player_for_context(
    game: &GameState,
    decision: &DecisionContext,
) -> PlayerId {
    game.controlling_player_for(decision.player())
}

pub(super) fn decision_exposes_object_to_perspective(
    game: &GameState,
    decision: Option<&DecisionContext>,
    perspective: PlayerId,
    id: ObjectId,
) -> bool {
    let Some(decision) = decision else {
        return false;
    };
    if decision_player_for_context(game, decision) != perspective {
        return false;
    }

    match decision {
        DecisionContext::SelectObjects(objects) => {
            objects.candidates.iter().any(|obj| obj.id == id)
        }
        DecisionContext::SelectOptions(options) => options.options.iter().any(|opt| {
            opt.object_id.is_some_and(|object_id| object_id == id)
                || opt
                    .related_object_ids
                    .as_ref()
                    .is_some_and(|object_ids| object_ids.contains(&id))
        }),
        DecisionContext::Targets(targets) => targets.requirements.iter().any(|requirement| {
            requirement
                .legal_targets
                .iter()
                .any(|target| matches!(target, Target::Object(object_id) if *object_id == id))
        }),
        DecisionContext::Order(order) => order.items.iter().any(|(object_id, _)| *object_id == id),
        DecisionContext::Attackers(attackers) => attackers.attacker_options.iter().any(|option| {
            option.creature == id
                    || option.valid_targets.iter().any(|target| {
                        matches!(target, AttackTarget::Planeswalker(object_id) if *object_id == id)
                })
        }),
        DecisionContext::Blockers(blockers) => blockers.blocker_options.iter().any(|option| {
            option.attacker == id
                || option
                    .valid_blockers
                    .iter()
                    .any(|(blocker, _)| *blocker == id)
        }),
        DecisionContext::Partition(_)
        | DecisionContext::Modes(_)
        | DecisionContext::HybridChoice(_)
        | DecisionContext::TextInput(_)
        | DecisionContext::Boolean(_)
        | DecisionContext::Number(_)
        | DecisionContext::Priority(_)
        | DecisionContext::Distribute(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Counters(_)
        | DecisionContext::Proliferate(_) => false,
    }
}

pub(super) fn object_visible_to_perspective(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    id: ObjectId,
) -> bool {
    let Some(obj) = game.object(id) else {
        return false;
    };

    let visible_via_view_effect = viewed_cards.is_some_and(|view| {
        (view.public || view.viewer == perspective) && view.contains_object(game, id)
    });
    if obj.zone == Zone::Exile && game.is_face_down(id) {
        return game.can_player_look_at_face_down_exiled_card(id, perspective)
            || visible_via_view_effect;
    }

    if !obj.zone.is_hidden() || game.controlling_player_for(obj.owner) == perspective {
        return true;
    }

    visible_via_view_effect
}

pub(super) fn redacted_action_label(action: &LegalAction) -> String {
    match action {
        LegalAction::CastSpell { .. } => "Cast hidden spell".to_string(),
        LegalAction::PlayLand { .. } => "Play hidden land".to_string(),
        LegalAction::UsePregameAction { .. } => "Use hidden pregame action".to_string(),
        _ => "Hidden action".to_string(),
    }
}

pub(super) fn optional_cost_selection_metadata(
    game: &GameState,
    source: Option<ObjectId>,
    option_index: usize,
) -> (bool, Option<u32>) {
    let Some(source_id) = source else {
        return (false, None);
    };
    let Some(obj) = game.object(source_id) else {
        return (false, None);
    };
    let Some(optional_cost) = obj.optional_costs.get(option_index) else {
        return (false, None);
    };
    if optional_cost.repeatable {
        // Keep a practical cap for UI count inputs. Engine legality remains authoritative.
        (true, Some(32))
    } else {
        (false, Some(1))
    }
}

pub(super) fn priority_action_ref(action: &LegalAction) -> PriorityActionRef {
    match action {
        LegalAction::PassPriority => PriorityActionRef::PassPriority,
        LegalAction::KeepOpeningHand => PriorityActionRef::KeepOpeningHand,
        LegalAction::TakeMulligan => PriorityActionRef::TakeMulligan,
        LegalAction::ContinuePregame => PriorityActionRef::ContinuePregame,
        LegalAction::BeginGame => PriorityActionRef::BeginGame,
        LegalAction::UsePregameAction {
            card_id,
            ability_index,
        } => PriorityActionRef::UsePregameAction {
            card_id: card_id.0,
            ability_index: *ability_index,
        },
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => PriorityActionRef::CastSpell {
            spell_id: spell_id.0,
            from_zone: zone_name(*from_zone),
            casting_method: casting_method_ref(casting_method),
        },
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => PriorityActionRef::ActivateAbility {
            source: source.0,
            ability_index: *ability_index,
        },
        LegalAction::PlayLand { land_id } => PriorityActionRef::PlayLand { land_id: land_id.0 },
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => PriorityActionRef::ActivateManaAbility {
            source: source.0,
            ability_index: *ability_index,
        },
        LegalAction::TurnFaceUp {
            creature_id,
            method,
        } => PriorityActionRef::TurnFaceUp {
            creature_id: creature_id.0,
            method: method.description().to_string(),
        },
        LegalAction::SpecialAction(action) => PriorityActionRef::SpecialAction {
            action: special_action_ref(action),
        },
    }
}

pub(super) fn special_action_ref(
    action: &ironsmith::special_actions::SpecialAction,
) -> SpecialActionRef {
    match action {
        ironsmith::special_actions::SpecialAction::PlayLand { card_id } => {
            SpecialActionRef::PlayLand { card_id: card_id.0 }
        }
        ironsmith::special_actions::SpecialAction::TurnFaceUp {
            permanent_id,
            method,
        } => SpecialActionRef::TurnFaceUp {
            permanent_id: permanent_id.0,
            method: method.description().to_string(),
        },
        ironsmith::special_actions::SpecialAction::Suspend { card_id } => {
            SpecialActionRef::Suspend { card_id: card_id.0 }
        }
        ironsmith::special_actions::SpecialAction::Foretell { card_id } => {
            SpecialActionRef::Foretell { card_id: card_id.0 }
        }
        ironsmith::special_actions::SpecialAction::Plot { card_id } => {
            SpecialActionRef::Plot { card_id: card_id.0 }
        }
        ironsmith::special_actions::SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        } => SpecialActionRef::ActivateManaAbility {
            permanent_id: permanent_id.0,
            ability_index: *ability_index,
        },
        ironsmith::special_actions::SpecialAction::UnlockRoomDoor { room_id } => {
            SpecialActionRef::UnlockRoomDoor { room_id: room_id.0 }
        }
    }
}

pub(super) fn casting_method_ref(
    method: &ironsmith::alternative_cast::CastingMethod,
) -> CastingMethodRef {
    match method {
        ironsmith::alternative_cast::CastingMethod::Normal => CastingMethodRef::Normal,
        ironsmith::alternative_cast::CastingMethod::FaceDown => CastingMethodRef::FaceDown,
        ironsmith::alternative_cast::CastingMethod::SplitOtherHalf => {
            CastingMethodRef::SplitOtherHalf
        }
        ironsmith::alternative_cast::CastingMethod::Fuse => CastingMethodRef::Fuse,
        ironsmith::alternative_cast::CastingMethod::Alternative(index) => {
            CastingMethodRef::Alternative { index: *index }
        }
        ironsmith::alternative_cast::CastingMethod::GrantedEscape {
            source,
            exile_count,
        } => CastingMethodRef::GrantedEscape {
            source: source.0,
            exile_count: *exile_count,
        },
        ironsmith::alternative_cast::CastingMethod::GrantedFlashback => {
            CastingMethodRef::GrantedFlashback
        }
        ironsmith::alternative_cast::CastingMethod::PlayFrom {
            source,
            zone,
            use_alternative,
        } => CastingMethodRef::PlayFrom {
            source: source.0,
            zone: zone_name(*zone),
            use_alternative: *use_alternative,
        },
        ironsmith::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
            source,
            zone,
            use_alternative,
        } => CastingMethodRef::SplitOtherHalfPlayFrom {
            source: source.0,
            zone: zone_name(*zone),
            use_alternative: *use_alternative,
        },
    }
}

pub(super) fn resolve_priority_action(
    priority: &ironsmith::decisions::context::PriorityContext,
    action_index: Option<usize>,
    action_ref: Option<&PriorityActionRef>,
) -> Option<LegalAction> {
    if let Some(action_ref) = action_ref {
        return priority
            .actions
            .iter()
            .find(|action| priority_action_ref(action) == *action_ref)
            .cloned();
    }
    action_index.and_then(|index| priority.actions.get(index).cloned())
}

/// Derive a short structured reason label from a DecisionContext.
pub(super) fn decision_reason(ctx: &DecisionContext) -> Option<String> {
    match ctx {
        DecisionContext::Boolean(b) => {
            let d = b.description.to_lowercase();
            if d.contains("ward") {
                Some("Ward".into())
            } else if d.contains("miracle") {
                Some("Miracle".into())
            } else if d.contains("madness") {
                Some("Madness".into())
            } else if d.contains("new targets") {
                Some("Retarget".into())
            } else if d.starts_with("you may") || d.starts_with("may ") {
                Some("May ability".into())
            } else {
                None
            }
        }
        DecisionContext::Number(n) => {
            if n.is_x_value {
                Some("X value".into())
            } else {
                Some("Choose number".into())
            }
        }
        DecisionContext::TextInput(_) => Some("Text entry".into()),
        DecisionContext::SelectOptions(o) => {
            let d = o.description.to_lowercase();
            if d.contains("replacement") {
                Some("Replacement effect".into())
            } else if d.contains("choose the next cost to pay") {
                Some("Next cost".into())
            } else if d.contains("optional cost") {
                Some("Additional costs".into())
            } else {
                None
            }
        }
        DecisionContext::Modes(_) => Some("Modal choice".into()),
        DecisionContext::HybridChoice(_) => Some("Mana payment".into()),
        DecisionContext::Order(o) => {
            let d = o.description.to_lowercase();
            if d.contains("blocker") {
                Some("Order blockers".into())
            } else if d.contains("attacker") {
                Some("Order attackers".into())
            } else if d.contains("trigger") {
                Some("Order triggers".into())
            } else {
                Some("Ordering".into())
            }
        }
        DecisionContext::Distribute(_) => Some("Distribute".into()),
        DecisionContext::Colors(_) => Some("Choose color".into()),
        DecisionContext::Counters(_) => Some("Remove counters".into()),
        DecisionContext::Partition(p) => {
            let d = p.description.to_lowercase();
            if d.starts_with("surveil") {
                Some("Surveil".into())
            } else {
                Some("Scry".into())
            }
        }
        DecisionContext::Proliferate(_) => Some("Proliferate".into()),
        DecisionContext::SelectObjects(o) => {
            let d = o.description.to_lowercase();
            if d.contains("sacrifice") {
                Some("Sacrifice".into())
            } else if d.contains("discard") {
                Some("Discard".into())
            } else if d.contains("exile") {
                Some("Exile".into())
            } else if d.contains("search") {
                Some("Search library".into())
            } else if d.contains("legend rule") {
                Some("Legend rule".into())
            } else if d.contains("destroy") {
                Some("Destroy".into())
            } else if d.contains("return") {
                Some("Return".into())
            } else {
                None
            }
        }
        DecisionContext::Targets(_) => Some("Choose targets".into()),
        DecisionContext::Priority(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_) => None,
    }
}

pub(super) fn normalize_action_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn target_choice_view(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    decision: Option<&DecisionContext>,
    index: usize,
    target: &Target,
) -> TargetChoiceView {
    match target {
        Target::Player(pid) => TargetChoiceView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        Target::Object(id) => {
            let visible = object_visible_to_perspective(game, perspective, viewed_cards, *id)
                || decision_exposes_object_to_perspective(game, decision, perspective, *id);
            TargetChoiceView::Object {
                object: if visible {
                    id.0
                } else {
                    redacted_choice_id(index)
                },
                name: if visible {
                    object_name(game, *id)
                } else {
                    hidden_object_label()
                },
            }
        }
    }
}

pub(super) fn attack_target_view(game: &GameState, target: &AttackTarget) -> AttackTargetView {
    match target {
        AttackTarget::Player(pid) => AttackTargetView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        AttackTarget::Planeswalker(id) => AttackTargetView::Planeswalker {
            object: id.0,
            name: object_name(game, *id),
        },
    }
}

pub(super) fn attack_target_from_input(input: &AttackTargetInput) -> AttackTarget {
    match input {
        AttackTargetInput::Player { player } => AttackTarget::Player(PlayerId::from_index(*player)),
        AttackTargetInput::Planeswalker { object } => {
            AttackTarget::Planeswalker(ObjectId::from_raw(*object))
        }
    }
}

pub(super) fn colors_for_context(
    ctx: &ironsmith::decisions::context::ColorsContext,
) -> Vec<ironsmith::color::Color> {
    if let Some(available) = &ctx.available_colors {
        if !available.is_empty() {
            return available.clone();
        }
    }
    ironsmith::color::Color::ALL.to_vec()
}

pub(super) fn color_name(color: ironsmith::color::Color) -> &'static str {
    match color {
        ironsmith::color::Color::White => "White",
        ironsmith::color::Color::Blue => "Blue",
        ironsmith::color::Color::Black => "Black",
        ironsmith::color::Color::Red => "Red",
        ironsmith::color::Color::Green => "Green",
    }
}

pub(super) fn unique_indices(indices: &[usize]) -> Vec<usize> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for &index in indices {
        if seen.insert(index) {
            unique.push(index);
        }
    }
    unique
}

pub(super) fn unique_object_ids(ids: &[u64]) -> Vec<u64> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for &id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }
    unique
}

pub(super) fn decision_context_kind(ctx: &DecisionContext) -> &'static str {
    match ctx {
        DecisionContext::Boolean(_) => "boolean",
        DecisionContext::Number(_) => "number",
        DecisionContext::TextInput(_) => "text_input",
        DecisionContext::SelectObjects(_) => "select_objects",
        DecisionContext::SelectOptions(_) => "select_options",
        DecisionContext::Modes(_) => "modes",
        DecisionContext::HybridChoice(_) => "hybrid_choice",
        DecisionContext::Order(_) => "order",
        DecisionContext::Attackers(_) => "attackers",
        DecisionContext::Blockers(_) => "blockers",
        DecisionContext::Distribute(_) => "distribute",
        DecisionContext::Colors(_) => "colors",
        DecisionContext::Counters(_) => "counters",
        DecisionContext::Partition(_) => "partition",
        DecisionContext::Proliferate(_) => "proliferate",
        DecisionContext::Priority(_) => "priority",
        DecisionContext::Targets(_) => "targets",
    }
}

pub(super) fn replay_decision_requires_root_reexecution(ctx: &DecisionContext) -> bool {
    matches!(
        ctx,
        DecisionContext::Boolean(_)
            | DecisionContext::TextInput(_)
            | DecisionContext::SelectOptions(_)
            | DecisionContext::Order(_)
            | DecisionContext::Distribute(_)
            | DecisionContext::Colors(_)
            | DecisionContext::Counters(_)
            | DecisionContext::Partition(_)
            | DecisionContext::Proliferate(_)
    )
}

pub(super) fn validate_attacker_declarations(
    attackers: &ironsmith::decisions::context::AttackersContext,
    declarations: &[AttackerDeclarationInput],
) -> Result<Vec<AttackerDeclaration>, JsValue> {
    let options: HashMap<u64, &ironsmith::decisions::context::AttackerOptionContext> = attackers
        .attacker_options
        .iter()
        .map(|option| (option.creature.0, option))
        .collect();
    let mut declared_creatures = HashSet::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.creature) else {
            return Err(JsValue::from_str(&format!(
                "invalid attacker creature id: {}",
                declaration.creature
            )));
        };
        if !declared_creatures.insert(declaration.creature) {
            return Err(JsValue::from_str(&format!(
                "attacker declared twice: {}",
                declaration.creature
            )));
        }

        let target = attack_target_from_input(&declaration.target);
        if !option.valid_targets.contains(&target) {
            return Err(JsValue::from_str(&format!(
                "invalid attack target for creature {}",
                declaration.creature
            )));
        }

        converted.push(AttackerDeclaration {
            creature: ObjectId::from_raw(declaration.creature),
            target,
        });
    }

    for option in &attackers.attacker_options {
        if option.must_attack && !declared_creatures.contains(&option.creature.0) {
            return Err(JsValue::from_str(&format!(
                "{} must attack if able",
                option.creature_name
            )));
        }
    }

    Ok(converted)
}

pub(super) fn validate_blocker_declarations(
    blockers: &ironsmith::decisions::context::BlockersContext,
    declarations: &[BlockerDeclarationInput],
) -> Result<Vec<BlockerDeclaration>, JsValue> {
    let options: HashMap<u64, &ironsmith::decisions::context::BlockerOptionContext> = blockers
        .blocker_options
        .iter()
        .map(|option| (option.attacker.0, option))
        .collect();

    // Compute per-blocker max assignments: the number of distinct attacker options
    // that list this blocker as valid (i.e. how many attackers it can block).
    let mut blocker_max_assignments: HashMap<u64, usize> = HashMap::new();
    for option in &blockers.blocker_options {
        for (blocker_id, _) in &option.valid_blockers {
            *blocker_max_assignments.entry(blocker_id.0).or_insert(0) += 1;
        }
    }

    let mut blocker_assignment_count: HashMap<u64, usize> = HashMap::new();
    let mut blocker_attacker_pairs: HashSet<(u64, u64)> = HashSet::new();
    let mut counts_by_attacker: HashMap<u64, usize> = HashMap::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.blocking) else {
            return Err(JsValue::from_str(&format!(
                "invalid blocking attacker id: {}",
                declaration.blocking
            )));
        };
        if !option
            .valid_blockers
            .iter()
            .any(|(id, _)| id.0 == declaration.blocker)
        {
            return Err(JsValue::from_str(&format!(
                "invalid blocker {} for attacker {}",
                declaration.blocker, declaration.blocking
            )));
        }
        // Reject duplicate (blocker, attacker) pairs.
        if !blocker_attacker_pairs.insert((declaration.blocker, declaration.blocking)) {
            return Err(JsValue::from_str(&format!(
                "blocker {} already assigned to attacker {}",
                declaration.blocker, declaration.blocking
            )));
        }
        // Check per-blocker assignment limit.
        let count = blocker_assignment_count
            .entry(declaration.blocker)
            .or_insert(0);
        *count += 1;
        let max = blocker_max_assignments
            .get(&declaration.blocker)
            .copied()
            .unwrap_or(1);
        if *count > max {
            return Err(JsValue::from_str(&format!(
                "blocker {} cannot block more than {} attacker(s)",
                declaration.blocker, max
            )));
        }
        *counts_by_attacker.entry(declaration.blocking).or_insert(0) += 1;
        converted.push(BlockerDeclaration {
            blocker: ObjectId::from_raw(declaration.blocker),
            blocking: ObjectId::from_raw(declaration.blocking),
        });
    }

    for option in &blockers.blocker_options {
        let assigned = counts_by_attacker
            .get(&option.attacker.0)
            .copied()
            .unwrap_or(0);
        // "Minimum blockers" applies only when the attacker is blocked at all.
        // Example: menace means if blocked, it must be by 2+, but not blocked is legal.
        if assigned > 0 && assigned < option.min_blockers {
            return Err(JsValue::from_str(&format!(
                "{} requires at least {} blocker(s)",
                option.attacker_name, option.min_blockers
            )));
        }
    }

    Ok(converted)
}

pub(super) fn validate_option_selection(
    min: usize,
    max: Option<usize>,
    selected: &[usize],
    legal_indices: &[usize],
) -> Result<(), JsValue> {
    if selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} option(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} option(s)"
        )));
    }
    for selected_index in selected {
        if !legal_indices.contains(selected_index) {
            return Err(JsValue::from_str(&format!(
                "option index {selected_index} is not legal"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_object_selection(
    min: usize,
    max: Option<usize>,
    allow_partial_completion: bool,
    selected: &[u64],
    legal_ids: &[u64],
) -> Result<(), JsValue> {
    if !allow_partial_completion && selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} object(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} object(s)"
        )));
    }
    for object_id in selected {
        if !legal_ids.contains(object_id) {
            return Err(JsValue::from_str(&format!(
                "object id {object_id} is not legal"
            )));
        }
    }
    Ok(())
}

fn hidden_ref_matches_object(game: &GameState, id: ObjectId, hidden_ref: &HiddenObjectRef) -> bool {
    let Some(object) = game.object(id) else {
        return false;
    };
    let hidden = game.hidden_card_info(id);
    let owner = hidden.map(|info| info.owner).unwrap_or(object.owner);
    if hidden_ref.owner.is_some_and(|expected| owner.0 != expected) {
        return false;
    }
    if hidden_ref
        .zone
        .as_ref()
        .is_some_and(|expected| zone_name(object.zone) != *expected)
    {
        return false;
    }
    if let Some(expected_slot) = hidden_ref.slot {
        if hidden.is_none_or(|info| info.slot != expected_slot) {
            return false;
        }
    }
    if let Some(expected_slot) = hidden_ref.public_slot {
        if hidden.is_none_or(|info| info.public_slot != Some(expected_slot)) {
            return false;
        }
    }
    if let Some(expected_commitment) = hidden_ref.commitment.as_deref() {
        if hidden.is_none_or(|info| {
            info.commitment != expected_commitment
                && info.public_commitment.as_deref() != Some(expected_commitment)
        }) {
            return false;
        }
    }
    if let Some(expected_commitment) = hidden_ref.public_commitment.as_deref() {
        if hidden.is_none_or(|info| {
            info.commitment != expected_commitment
                && info.public_commitment.as_deref() != Some(expected_commitment)
        }) {
            return false;
        }
    }
    true
}

fn unique_legal_candidate_by_stable_id(
    game: &GameState,
    ctx: &ironsmith::decisions::context::SelectObjectsContext,
    stable_id: u64,
) -> Result<ObjectId, JsValue> {
    let mut matches = ctx
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .filter_map(|candidate| {
            game.object(candidate.id)
                .is_some_and(|object| object.stable_id.0.0 == stable_id)
                .then_some(candidate.id)
        });
    let Some(first) = matches.next() else {
        return Err(JsValue::from_str(&format!(
            "stable object id {stable_id} is not legal"
        )));
    };
    if matches.next().is_some() {
        return Err(JsValue::from_str(&format!(
            "stable object id {stable_id} matches multiple legal candidates"
        )));
    }
    Ok(first)
}

fn unique_legal_candidate_by_hidden_ref(
    game: &GameState,
    ctx: &ironsmith::decisions::context::SelectObjectsContext,
    hidden_ref: &HiddenObjectRef,
) -> Result<ObjectId, JsValue> {
    let mut matches = ctx
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .filter(|candidate| hidden_ref_matches_object(game, candidate.id, hidden_ref))
        .map(|candidate| candidate.id);
    let Some(first) = matches.next() else {
        let truncate = |raw: &str| raw.chars().take(12).collect::<String>();
        let candidate_summary: Vec<String> = ctx
            .candidates
            .iter()
            .map(|candidate| {
                let hidden = game.hidden_card_info(candidate.id);
                format!(
                    "id={} legal={} zone={} slot={} pslot={} c={} pc={}",
                    candidate.id.0,
                    candidate.legal,
                    game.object(candidate.id)
                        .map(|object| zone_name(object.zone))
                        .unwrap_or_else(|| "gone".to_string()),
                    hidden.map(|info| info.slot as i32).unwrap_or(-1),
                    hidden
                        .and_then(|info| info.public_slot)
                        .map(|slot| slot as i32)
                        .unwrap_or(-1),
                    hidden
                        .map(|info| truncate(&info.commitment))
                        .unwrap_or_default(),
                    hidden
                        .and_then(|info| info.public_commitment.as_deref())
                        .map(truncate)
                        .unwrap_or_default(),
                )
            })
            .collect();
        return Err(JsValue::from_str(&format!(
            "hidden object reference is not legal (ref owner={:?} zone={:?} slot={:?} pslot={:?} c={} pc={}; candidates: [{}])",
            hidden_ref.owner,
            hidden_ref.zone,
            hidden_ref.slot,
            hidden_ref.public_slot,
            hidden_ref
                .commitment
                .as_deref()
                .map(truncate)
                .unwrap_or_default(),
            hidden_ref
                .public_commitment
                .as_deref()
                .map(truncate)
                .unwrap_or_default(),
            candidate_summary.join("; "),
        )));
    };
    if matches.next().is_some() {
        return Err(JsValue::from_str(
            "hidden object reference matches multiple legal candidates",
        ));
    }
    Ok(first)
}

pub(super) fn normalize_select_object_choice_ids(
    game: &GameState,
    ctx: &ironsmith::decisions::context::SelectObjectsContext,
    selected: &[u64],
    stable_ids: &[Option<u64>],
    hidden_refs: &[Option<HiddenObjectRef>],
) -> Result<Vec<u64>, JsValue> {
    selected
        .iter()
        .enumerate()
        .map(|(choice_index, selected_id)| {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.legal && candidate.id.0 == *selected_id)
            {
                return Ok(*selected_id);
            }

            if let Some(mapped_id) = ctx
                .candidates
                .iter()
                .enumerate()
                .find(|(index, candidate)| {
                    candidate.legal && redacted_choice_id(*index) == *selected_id
                })
                .map(|(_, candidate)| candidate.id.0)
            {
                return Ok(mapped_id);
            }

            if let Some(Some(stable_id)) = stable_ids.get(choice_index) {
                return unique_legal_candidate_by_stable_id(game, ctx, *stable_id).map(|id| id.0);
            }

            if let Some(Some(hidden_ref)) = hidden_refs.get(choice_index) {
                return unique_legal_candidate_by_hidden_ref(game, ctx, hidden_ref).map(|id| id.0);
            }

            Ok(*selected_id)
        })
        .collect()
}

/// Convert and validate target inputs against the requirements in a TargetsContext.
///
/// Validates that:
/// - Each selected target is legal in at least one requirement
/// - The flattened target list can be assigned to the requirements in order
pub(super) fn convert_and_validate_targets(
    ctx: &ironsmith::decisions::context::TargetsContext,
    inputs: Vec<TargetInput>,
) -> Result<Vec<Target>, String> {
    let converted: Vec<Target> = inputs
        .into_iter()
        .map(|target| match target {
            TargetInput::Player { player } => Target::Player(PlayerId::from_index(player)),
            TargetInput::Object { object } => Target::Object(ObjectId::from_raw(object)),
        })
        .collect();

    // Build the set of all legal targets across all requirements.
    let all_legal: HashSet<Target> = ctx
        .requirements
        .iter()
        .flat_map(|req| req.legal_targets.iter().copied())
        .collect();

    // Validate every chosen target is legal somewhere.
    for target in &converted {
        if !all_legal.contains(target) {
            return Err(format!(
                "target {} is not a legal choice",
                match target {
                    Target::Player(p) => format!("player {}", p.0),
                    Target::Object(o) => format!("object {}", o.0),
                }
            ));
        }
    }

    if !validate_flat_target_assignment(&ctx.requirements, &converted) {
        return Err("targets do not satisfy the targeting requirements in order".to_string());
    }

    Ok(converted)
}
