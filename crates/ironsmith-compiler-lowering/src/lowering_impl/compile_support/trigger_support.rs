use crate::cards::builders::{
    CardTextError, DamageBySpec, EffectAst, ReferenceImports, TagKey, TriggerSpec,
};
use crate::effect::{Effect, EventValueSpec};
use crate::filter::ObjectRef;
use crate::model::ast::TriggerIntroSurfaceAst;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::Trigger;

use super::LoweredEffects;

fn after_exact_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value.starts_with(prefix).then(|| &value[prefix.len()..])
}

fn one_or_more_subject_description(filter: &crate::target::ObjectFilter) -> String {
    fn has_subtype_named(filter: &crate::target::ObjectFilter, word: &str) -> bool {
        filter
            .subtypes
            .iter()
            .any(|subtype| subtype.to_string().eq_ignore_ascii_case(word))
            || filter
                .any_of
                .iter()
                .any(|branch| has_subtype_named(branch, word))
    }

    let pluralize_word = |word: &str| {
        let lower = word.to_ascii_lowercase();
        let plural = match lower.as_str() {
            "elf" => "elves".to_string(),
            "dwarf" => "dwarves".to_string(),
            "wolf" => "wolves".to_string(),
            "werewolf" => "werewolves".to_string(),
            "mouse" => "mice".to_string(),
            "plains" | "urzas" | "myr" | "merfolk" | "equipment" => word.to_string(),
            _ if lower.ends_with('y')
                && lower.len() > 1
                && !matches!(
                    lower.as_bytes().get(lower.len() - 2).copied(),
                    Some(b'a' | b'e' | b'i' | b'o' | b'u')
                ) =>
            {
                format!("{}ies", &word[..word.len() - 1])
            }
            _ if lower.ends_with('s')
                || lower.ends_with('x')
                || lower.ends_with('z')
                || lower.ends_with("ch")
                || lower.ends_with("sh") =>
            {
                format!("{word}es")
            }
            _ => format!("{word}s"),
        };
        if word.chars().next().is_some_and(char::is_uppercase) {
            let mut chars = plural.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        } else {
            plural
        }
    };
    let description = filter
        .description()
        .split_whitespace()
        .map(|word| {
            let bare_word = word.trim_end_matches(',');
            let punctuation = &word[bare_word.len()..];
            let is_subtype = has_subtype_named(filter, bare_word);
            if is_subtype
                || matches!(
                    bare_word,
                    "artifact"
                        | "battle"
                        | "card"
                        | "creature"
                        | "enchantment"
                        | "land"
                        | "permanent"
                        | "planeswalker"
                        | "player"
                        | "source"
                        | "spell"
                        | "token"
                )
            {
                format!("{}{punctuation}", pluralize_word(bare_word))
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let description = after_exact_prefix(&description, "a ")
        .or_else(|| after_exact_prefix(&description, "an "))
        .unwrap_or(&description);
    let description = if filter.other {
        after_exact_prefix(description, "another ")
            .map(|rest| format!("other {rest}"))
            .unwrap_or_else(|| description.to_string())
    } else {
        description.to_string()
    };
    format!("one or more {description}")
}

fn describe_source_and_or_one_or_more_other_enters(
    left: &TriggerSpec,
    right: &TriggerSpec,
) -> Option<String> {
    fn pair<'a>(
        source: &'a TriggerSpec,
        others: &'a TriggerSpec,
    ) -> Option<(
        &'a crate::target::SourceReferenceSurface,
        &'a crate::target::ObjectFilter,
    )> {
        let TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface,
            subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber::Singular,
            origin_condition: source_origin,
        } = source
        else {
            return None;
        };
        let TriggerSpec::EntersBattlefieldOneOrMore {
            filter,
            cause_filter: None,
            origin_condition: other_origin,
        } = others
        else {
            return None;
        };
        (source_origin == other_origin
            && filter.other
            && filter.union_connective() == crate::filter::ObjectFilterUnionConnective::AndOr)
            .then_some((surface, filter))
    }

    let (surface, filter) = pair(left, right).or_else(|| pair(right, left))?;
    Some(format!(
        "Whenever {} and/or {} enter",
        surface.display_text(),
        one_or_more_subject_description(filter),
    ))
}

fn damage_source_description(
    source: &crate::target::ObjectFilter,
    source_surface: &crate::triggers::DamageSourceSurface,
) -> String {
    let description = source.description();
    if *source_surface == crate::triggers::DamageSourceSurface::Source {
        description
            .split_whitespace()
            .map(|word| if word == "permanent" { "source" } else { word })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        description
    }
}

fn indefinite_subject_description(description: String) -> String {
    let lower = description.to_ascii_lowercase();
    if [
        "a ",
        "an ",
        "another ",
        "any ",
        "each ",
        "every ",
        "all ",
        "one or more ",
        "this ",
        "that ",
        "the ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
    {
        return description;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {description}")
}

fn strip_indefinite_article(description: &str) -> &str {
    after_exact_prefix(description, "a ")
        .or_else(|| after_exact_prefix(description, "an "))
        .unwrap_or(description)
}

fn repeated_intro_branch_description(trigger: &TriggerSpec) -> Option<String> {
    let TriggerSpec::WithIntro { intro, trigger } = trigger else {
        return None;
    };
    let intro = match intro {
        TriggerIntroSurfaceAst::When => "When",
        TriggerIntroSurfaceAst::Whenever => "Whenever",
        TriggerIntroSurfaceAst::At => "At",
    };
    let body = match trigger.as_ref() {
        TriggerSpec::AttacksOneOrMore(filter) if filter.controller == Some(PlayerFilter::You) => {
            let mut described_filter = filter.clone();
            described_filter.controller = None;
            format!(
                "you attack with {}",
                one_or_more_subject_description(&described_filter)
            )
        }
        TriggerSpec::Dies(filter) => format!(
            "{} dies",
            indefinite_subject_description(filter.description())
        ),
        _ => return None,
    };
    Some(format!("{intro} {body}"))
}

fn describe_damage_to_object_and_player_union(
    left: &TriggerSpec,
    right: &TriggerSpec,
) -> Option<String> {
    let (source, target, source_surface, player_source, player, player_source_surface) =
        match (left, right) {
            (
                TriggerSpec::DealsDamageTo {
                    source,
                    target,
                    source_surface,
                },
                TriggerSpec::DealsDamageToPlayer {
                    source: player_source,
                    player,
                    source_surface: player_source_surface,
                },
            ) => (
                source,
                target,
                source_surface,
                player_source,
                player,
                player_source_surface,
            ),
            (
                TriggerSpec::DealsDamageToPlayer {
                    source: player_source,
                    player,
                    source_surface: player_source_surface,
                },
                TriggerSpec::DealsDamageTo {
                    source,
                    target,
                    source_surface,
                },
            ) => (
                source,
                target,
                source_surface,
                player_source,
                player,
                player_source_surface,
            ),
            _ => return None,
        };

    if source != player_source
        || source_surface != player_source_surface
        || *player != PlayerFilter::Any
        || !target.union_is_one_or_more()
        || target.union_connective() != crate::filter::ObjectFilterUnionConnective::AndOr
    {
        return None;
    }
    let target_description = one_or_more_subject_description(target);
    let target_description =
        after_exact_prefix(&target_description, "one or more ").unwrap_or(&target_description);
    Some(format!(
        "Whenever {} deals damage to one or more {target_description} and/or players",
        damage_source_description(source, source_surface),
    ))
}

pub fn compile_trigger_spec(mut trigger: TriggerSpec) -> Trigger {
    let mut intro_surfaces = Vec::new();
    while let TriggerSpec::WithIntro {
        intro,
        trigger: inner,
    } = trigger
    {
        intro_surfaces.push(intro);
        trigger = *inner;
    }

    let mut compiled = compile_trigger_spec_without_intro(trigger);
    for intro in intro_surfaces.into_iter().rev() {
        compiled = compiled.with_intro_surface(match intro {
            TriggerIntroSurfaceAst::When => crate::triggers::TriggerIntroSurface::When,
            TriggerIntroSurfaceAst::Whenever => crate::triggers::TriggerIntroSurface::Whenever,
            TriggerIntroSurfaceAst::At => crate::triggers::TriggerIntroSurface::At,
        });
    }
    compiled
}

fn compile_trigger_spec_without_intro(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::WithIntro { .. } => {
            unreachable!("leading trigger intro surfaces are removed before lowering")
        }
        TriggerSpec::StateBased { display, .. } => Trigger::state_based(display),
        TriggerSpec::AnyOf(branches) => {
            Trigger::any_of(branches.into_iter().map(compile_trigger_spec).collect())
        }
        TriggerSpec::ConditionQualified {
            trigger,
            condition,
            surface,
        } => {
            let condition = super::compile_condition_from_predicate_ast(
                &condition,
                &mut super::EffectLoweringContext::new(),
                &None,
            )
            .expect("grammar-proven trigger qualification must lower");
            Trigger::condition_qualified(compile_trigger_spec(*trigger), condition, surface)
        }
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::ThisAttacksWhileYouControl(filter) => {
            Trigger::this_attacks_while_you_control(filter)
        }
        TriggerSpec::ThisAndAnotherAttackDifferentPlayers => {
            Trigger::this_and_another_attack_different_players()
        }
        TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast { count, filter } => {
            Trigger::this_attacks_player_who_controls_at_least(count as usize, filter)
        }
        TriggerSpec::ThisAttacksWithNOthers {
            other_count,
            display_subject,
            other_filter,
            other_surface,
        } => Trigger::this_attacks_with_n_others_display_subject_filter_and_other_surface(
            other_count as usize,
            display_subject,
            other_filter,
            other_surface,
        ),
        TriggerSpec::ThisAttacksWithExactlyNOthers(other_count) => {
            Trigger::this_attacks_with_exact_n_others(other_count as usize)
        }
        TriggerSpec::ThisAttacksAndIsntBlocked => Trigger::this_attacks_and_isnt_blocked(),
        TriggerSpec::ThisAttacksWhileSaddled => Trigger::this_attacks_while_saddled(),
        TriggerSpec::Attacks(filter) => Trigger::attacks(filter),
        TriggerSpec::AttacksAndIsntBlocked(filter) => Trigger::attacks_and_isnt_blocked(filter),
        TriggerSpec::AttacksWhileSaddled(filter) => Trigger::attacks_while_saddled(filter),
        TriggerSpec::AttacksOneOrMore(filter) => Trigger::attacks_one_or_more(filter),
        TriggerSpec::PlayersAttackedOneOrMore(player_filter) => {
            Trigger::players_attacked_one_or_more(player_filter)
        }
        TriggerSpec::PlayerAttacksOneOrMore { attacker, target } => {
            Trigger::player_attacks_one_or_more(attacker, target)
        }
        TriggerSpec::PlayerAttacksTargetWithOneOrMore { attacker, target } => {
            Trigger::player_attacks_target_with_one_or_more(attacker, target)
        }
        TriggerSpec::AttacksOneOrMoreWithMinTotal {
            filter,
            min_total_attackers,
        } => Trigger::attacks_one_or_more_with_min_total(filter, min_total_attackers as usize),
        TriggerSpec::AttacksOneOrMoreWithExactTotal {
            filter,
            total_attackers,
        } => Trigger::attacks_one_or_more_with_exact_total(filter, total_attackers as usize),
        TriggerSpec::AttacksOneOrMoreWithAggregate {
            filter,
            metric,
            comparison,
        } => Trigger::attacks_one_or_more_with_aggregate(filter, metric, comparison),
        TriggerSpec::AttacksAlone(filter) => Trigger::attacks_alone(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(filter) => Trigger::attacks_you(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter) => {
            Trigger::attacks_you_one_or_more(filter)
        }
        TriggerSpec::ThisBlocks => Trigger::this_blocks(),
        TriggerSpec::ThisBlocksObject {
            filter,
            min_blocked_objects,
        } => match min_blocked_objects {
            Some(minimum) => Trigger::this_blocks_objects_with_minimum(filter, minimum as usize),
            None => Trigger::this_blocks_object(filter),
        },
        TriggerSpec::Blocks(filter) => Trigger::blocks(filter),
        TriggerSpec::BlocksOneOrMore(filter) => Trigger::blocks_one_or_more(filter),
        TriggerSpec::BlocksOrBecomesBlockedByObject { subject, other } => {
            Trigger::blocks_or_becomes_blocked_by_object(subject, other)
        }
        TriggerSpec::BlocksObjectWithLesserPower { blocker, blocked } => {
            Trigger::blocks_object_with_lesser_power(blocker, blocked)
        }
        TriggerSpec::ThisBecomesBlocked => Trigger::this_becomes_blocked(),
        TriggerSpec::BecomesBlocked(filter) => Trigger::becomes_blocked(filter),
        TriggerSpec::ThisBecomesBlockedByObject(filter) => {
            Trigger::this_becomes_blocked_by_object(filter)
        }
        TriggerSpec::BecomesBlockedByObjectWithLesserPower { blocked, blocker } => {
            Trigger::becomes_blocked_by_object_with_lesser_power(blocked, blocker)
        }
        TriggerSpec::ThisDies => Trigger::this_dies(),
        TriggerSpec::ThisDiesOrIsExiled => Trigger::this_dies_or_is_exiled(),
        TriggerSpec::ThisDiesOrIsExiledWithSurface(surface) => {
            Trigger::this_dies_or_is_exiled_with_surface(surface.clone())
        }
        TriggerSpec::ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker { marker } => {
            Trigger::new(
                crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .from(crate::zone::Zone::Battlefield)
                    .to(crate::zone::Zone::Exile)
                    .filter(crate::target::ObjectFilter::creature())
                    .this()
                    .cause_filter(Some(
                        crate::events::cause::CauseFilter::from_cost()
                            .with_source(
                                crate::target::ObjectFilter::default().with_ability_marker(marker),
                            )
                            .with_controller(crate::events::cause::ControllerFilter::You),
                    )),
            )
        }
        TriggerSpec::ThisLeavesBattlefield => Trigger::this_leaves_battlefield(),
        TriggerSpec::ThisPhasesOut => Trigger::this_phases_out(),
        TriggerSpec::ThisLeavesBattlefieldWithSurface(surface) => Trigger::new(
            crate::triggers::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Battlefield)
                .this()
                .this_surface(surface.clone()),
        ),
        TriggerSpec::ThisMutates => Trigger::this_mutates(),
        TriggerSpec::ThisBecomesMonstrous => Trigger::this_becomes_monstrous(),
        TriggerSpec::ThisBecomesTapped => Trigger::becomes_tapped(),
        TriggerSpec::PermanentBecomesTapped(filter) => Trigger::permanent_becomes_tapped(filter),
        TriggerSpec::ThisBecomesUntapped => Trigger::becomes_untapped(),
        TriggerSpec::ThisTurnedFaceUp => Trigger::this_is_turned_face_up(),
        TriggerSpec::TurnedFaceUp(filter) => Trigger::turned_face_up(filter),
        TriggerSpec::ThisBecomesTargeted => Trigger::becomes_targeted(),
        TriggerSpec::BecomesTargeted(filter) => Trigger::becomes_targeted_object(filter),
        TriggerSpec::ThisBecomesTargetedBySpell(filter) => {
            Trigger::becomes_targeted_by_spell(filter)
        }
        TriggerSpec::ThisBecomesTargetedByStackObject(filter) => {
            Trigger::becomes_targeted_by_stack_object(filter)
        }
        TriggerSpec::BecomesTargetedByStackObject {
            target,
            stack_object,
        } => Trigger::becomes_targeted_object_by_stack_object(target, stack_object),
        TriggerSpec::BecomesTargetedBySourceController {
            target,
            source_controller,
        } => Trigger::becomes_targeted_by_source_controller(target, source_controller),
        TriggerSpec::PlayerOrObjectBecomesTargetedBySourceController {
            player,
            object,
            source_controller,
        } => Trigger::player_or_object_becomes_targeted_by_source_controller(
            player,
            object,
            source_controller,
        ),
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisDealsDamageToPlayer { player, amount } => {
            Trigger::this_deals_damage_to_player(player, amount)
        }
        TriggerSpec::ThisDealsDamageTo(filter) => Trigger::this_deals_damage_to(filter),
        TriggerSpec::ThisDealsCombatDamage => Trigger::this_deals_combat_damage(),
        TriggerSpec::ThisDealsCombatDamageTo(filter) => {
            Trigger::this_deals_combat_damage_to(filter)
        }
        TriggerSpec::DealsDamage {
            source,
            source_surface,
        } => Trigger::deals_damage_with_source_surface(source, source_surface),
        TriggerSpec::DealsDamageTo {
            source,
            target,
            source_surface,
        } => Trigger::deals_damage_to_with_source_surface(source, target, source_surface),
        TriggerSpec::DealsDamageToPlayer {
            source,
            player,
            source_surface,
        } => Trigger::deals_damage_to_player_with_source_surface(source, player, source_surface),
        TriggerSpec::DealsExactDamageToObjectOrPlayer {
            source,
            object,
            player,
            player_first,
            amount,
            source_surface,
        } => {
            let source_description = damage_source_description(&source, &source_surface);
            let object_description = indefinite_subject_description(object.description());
            let player_description = player.description();
            let recipients = if player_first {
                format!(
                    "{player_description} or {}",
                    strip_indefinite_article(&object_description)
                )
            } else {
                format!(
                    "{object_description} or {}",
                    strip_indefinite_article(&player_description)
                )
            };
            let display = format!(
                "Whenever {source_description} deals exactly {amount} damage to {recipients}"
            );
            Trigger::deals_exact_damage_to_object_or_player_with_source_surface(
                source,
                object,
                player,
                player_first,
                amount,
                source_surface,
            )
            .with_display_label(display)
        }
        TriggerSpec::DealsNoncombatDamageToPlayer {
            source,
            player,
            source_surface,
            damaged_player_one_or_more,
            during_turn,
        } => {
            let source_description =
                indefinite_subject_description(damage_source_description(&source, &source_surface));
            let player_description =
                if damaged_player_one_or_more && player == PlayerFilter::Opponent {
                    "one or more of your opponents".to_string()
                } else {
                    player.description()
                };
            let turn_description = match during_turn.as_ref() {
                Some(PlayerFilter::You) => " during your turn",
                Some(PlayerFilter::Opponent) => " during an opponent's turn",
                _ => "",
            };
            let display = format!(
                "Whenever {source_description} deals noncombat damage to {player_description}{turn_description}"
            );
            Trigger::deals_noncombat_damage_to_player_qualified(
                source,
                player,
                source_surface,
                damaged_player_one_or_more,
                during_turn,
            )
            .with_display_label(display)
        }
        TriggerSpec::DealsCombatDamage(filter) => Trigger::deals_combat_damage(filter),
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            Trigger::deals_combat_damage_to(source, target)
        }
        TriggerSpec::PlayerPlaysLand { player, filter } => {
            Trigger::player_plays_land(player, filter)
        }
        TriggerSpec::PlayerGivesGift(player) => Trigger::player_gives_gift(player),
        TriggerSpec::PlayerSearchesLibrary(player) => Trigger::player_searches_library(player),
        TriggerSpec::PlayerShufflesLibrary {
            player,
            caused_by_effect,
            source_controller_shuffles,
        } => Trigger::player_shuffles_library(player, caused_by_effect, source_controller_shuffles),
        TriggerSpec::PlayerTapsForMana { player, filter } => {
            Trigger::player_taps_for_mana(player, filter)
        }
        TriggerSpec::PlayerRollsResult { player, result } => {
            Trigger::player_rolls_result(player, result)
        }
        TriggerSpec::PlayerRollsHighestNaturalResult { player } => {
            Trigger::player_rolls_highest_natural_result(player)
        }
        TriggerSpec::PlayerRollsDie {
            player,
            one_or_more,
        } => Trigger::player_rolls_die_with_surface(player, one_or_more),
        TriggerSpec::PlayerCoinFlipResult { player, won } => {
            Trigger::player_coin_flip_result(player, won)
        }
        TriggerSpec::AbilityActivated {
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        } => Trigger::ability_activated_qualified_with_activation_cost_tap(
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        ),
        TriggerSpec::AbilityTriggered {
            another,
            source_filter,
            caused_by_source_entering,
        } => {
            Trigger::ability_triggered_qualified(another, source_filter, caused_by_source_entering)
        }
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::ThisIsDealtCombatDamage => Trigger::is_dealt_combat_damage(ChooseSpec::Source),
        TriggerSpec::IsDealtDamage(filter) => Trigger::is_dealt_damage(ChooseSpec::Object(filter)),
        TriggerSpec::IsDealtCombatDamage(filter) => {
            Trigger::is_dealt_combat_damage(ChooseSpec::Object(filter))
        }
        TriggerSpec::IsDealtExcessNoncombatDamage(filter) => {
            Trigger::is_dealt_excess_noncombat_damage(ChooseSpec::Object(filter))
        }
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::YouGainLifeCausedBy(source) => Trigger::you_gain_life_caused_by(source),
        TriggerSpec::YouGainLifeDuringTurn(during_turn) => {
            Trigger::you_gain_life_during_turn(during_turn)
        }
        TriggerSpec::PlayerLosesLife(player) => Trigger::player_loses_life(player),
        TriggerSpec::PlayersLoseLifeOneOrMore(player) => {
            Trigger::players_lose_life_one_or_more(player)
        }
        TriggerSpec::OpponentsEachLoseExactLife { amount } => {
            Trigger::opponents_each_lose_exact_life(amount)
        }
        TriggerSpec::PlayerLosesGame(player) => Trigger::player_loses_game(player),
        TriggerSpec::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        } => Trigger::player_loses_life_during_turn(player, during_turn),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::PlayerDrawsCard(player) => Trigger::player_draws_card(player),
        TriggerSpec::PlayerDrawsCardNotDuringTurn {
            player,
            during_turn,
        } => Trigger::player_draws_card_not_during_turn(player, during_turn),
        TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(player) => {
            Trigger::player_draws_card_except_first_in_draw_step(player)
        }
        TriggerSpec::PlayerDrawsNthCardEachTurn {
            player,
            card_number,
        } => Trigger::player_draws_nth_card_each_turn(player, card_number),
        TriggerSpec::PlayerDrawsNumberedCardsEachTurn {
            player,
            card_numbers,
        } => Trigger::player_draws_numbered_cards_each_turn(player, card_numbers),
        TriggerSpec::PlayerDiscardsCard {
            player,
            filter,
            cause_controller,
            effect_like_only,
            one_or_more,
        } => {
            if one_or_more {
                Trigger::player_discards_cards(player, filter)
            } else if let Some(cause_controller) = cause_controller {
                Trigger::player_discards_card_caused_by_controller(
                    player,
                    filter,
                    cause_controller,
                    effect_like_only,
                )
            } else {
                Trigger::player_discards_card(player, filter)
            }
        }
        TriggerSpec::PlayerRevealsCard {
            player,
            filter,
            from_source,
        } => Trigger::player_reveals_card(player, filter, from_source),
        TriggerSpec::PlayerSacrifices {
            player,
            filter,
            one_or_more,
        } => Trigger::player_sacrifices_with_surface(player, filter, one_or_more),
        TriggerSpec::PermanentSacrificed(filter) => Trigger::permanent_sacrificed(filter),
        TriggerSpec::PermanentDestroyed(filter) => Trigger::permanent_destroyed(filter),
        TriggerSpec::TokensCreated {
            player,
            filter,
            one_or_more,
        } => Trigger::tokens_created(player, filter, one_or_more),
        TriggerSpec::LeavesBattlefield(filter) => Trigger::leaves_battlefield(filter),
        TriggerSpec::LeavesBattlefieldWithoutDying {
            filter,
            one_or_more,
        } => Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Battlefield)
                .to_any_except(crate::zone::Zone::Graveyard)
                .filter(filter)
                .count(if one_or_more {
                    crate::triggers::CountMode::OneOrMore
                } else {
                    crate::triggers::CountMode::One
                }),
        ),
        TriggerSpec::Dies(filter) => {
            let display = format!("Whenever {} dies", filter.description());
            Trigger::new(
                crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .from(crate::zone::Zone::Battlefield)
                    .to(crate::zone::Zone::Graveyard)
                    .filter(filter)
                    .graveyard_surface(crate::triggers::GraveyardTriggerSurface::Dies),
            )
            .with_display_label(display)
        }
        TriggerSpec::DiesOneOrMore(filter) => {
            let display = format!("Whenever {} die", one_or_more_subject_description(&filter));
            Trigger::new(
                crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .from(crate::zone::Zone::Battlefield)
                    .to(crate::zone::Zone::Graveyard)
                    .filter(filter)
                    .count(crate::triggers::CountMode::OneOrMore)
                    .graveyard_surface(crate::triggers::GraveyardTriggerSurface::Dies),
            )
            .with_display_label(display)
        }
        TriggerSpec::DiesDuringTurn {
            filter,
            one_or_more,
            during_turn,
        } => {
            let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Battlefield)
                .to(crate::zone::Zone::Graveyard)
                .filter(filter)
                .during_turn(during_turn)
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::Dies);
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::DiesDuringCombat {
            filter,
            one_or_more,
        } => {
            let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Battlefield)
                .to(crate::zone::Zone::Graveyard)
                .during_combat()
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::Dies);
            if let Some(filter) = filter {
                trigger = trigger.filter(filter);
            } else {
                trigger = trigger.this();
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::PutIntoGraveyard(filter) => Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .to(crate::zone::Zone::Graveyard)
                .filter(filter)
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::PutIntoGraveyard),
        ),
        TriggerSpec::PutIntoGraveyardOneOrMore(filter) => Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .to(crate::zone::Zone::Graveyard)
                .filter(filter)
                .count(crate::triggers::CountMode::OneOrMore)
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::PutIntoGraveyard),
        ),
        TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from,
            one_or_more,
        } => {
            let trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Graveyard)
                .filter(filter)
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::PutIntoGraveyard);
            if one_or_more {
                Trigger::new(trigger.count(crate::triggers::CountMode::OneOrMore))
            } else {
                Trigger::new(trigger)
            }
        }
        TriggerSpec::PutIntoGraveyardFromAnyExcept {
            filter,
            excluded,
            one_or_more,
        } => {
            let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from_any_except(excluded)
                .to(crate::zone::Zone::Graveyard)
                .filter(filter)
                .graveyard_surface(crate::triggers::GraveyardTriggerSurface::PutIntoGraveyard);
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::PutIntoExileFromZones {
            filter,
            from,
            one_or_more,
            during_turn,
            cause_filter,
        } => {
            let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .to(crate::zone::Zone::Exile)
                .filter(filter)
                .cause_filter(cause_filter);
            if !from.is_empty() {
                trigger = trigger.from_any_of(from);
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            if let Some(during_turn) = during_turn {
                trigger = trigger.during_turn(during_turn);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::CardsLeaveYourGraveyard {
            filter,
            one_or_more,
            during_your_turn,
        } => Trigger::cards_leave_your_graveyard(filter, one_or_more, during_your_turn),
        TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller,
            one_or_more,
            include_players,
        } => {
            let mut trigger = crate::triggers::CounterPutOnTrigger::new(filter);
            if include_players {
                trigger = trigger.include_players();
            }
            if let Some(counter_type) = counter_type {
                trigger = trigger.counter_type(counter_type);
            }
            if let Some(source_controller) = source_controller {
                trigger = trigger.source_controller(source_controller);
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::NthCounterPutOn {
            filter,
            counter_type,
            counter_number,
        } => Trigger::nth_counter_put_on(filter, counter_type, counter_number),
        TriggerSpec::CounterRemovedFrom {
            filter,
            counter_type,
            last,
            one_or_more,
            caused_by_source,
        } => {
            let mut trigger = crate::triggers::CounterRemovedFromTrigger::new(filter);
            if let Some(counter_type) = counter_type {
                trigger = trigger.counter_type(counter_type);
            }
            if last {
                trigger = trigger.last();
            }
            if one_or_more {
                trigger = trigger.one_or_more();
            }
            if caused_by_source {
                trigger = trigger.caused_by_source();
            }
            Trigger::new(trigger)
        }
        TriggerSpec::PlayerGetsCounters {
            player,
            counter_type,
            one_or_more,
        } => {
            let mut trigger = crate::triggers::PlayerGetsCountersTrigger::new(player);
            if let Some(counter_type) = counter_type {
                trigger = trigger.counter_type(counter_type);
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::DiesCreatureDealtDamageByThisTurn { victim, damager } => match damager {
            DamageBySpec::ThisCreature => {
                Trigger::creature_dealt_damage_by_this_creature_this_turn_dies(victim)
            }
            DamageBySpec::EquippedCreature => {
                Trigger::creature_dealt_damage_by_equipped_creature_this_turn_dies(victim)
            }
            DamageBySpec::EnchantedCreature => {
                Trigger::creature_dealt_damage_by_enchanted_creature_this_turn_dies(victim)
            }
        },
        TriggerSpec::DiesCreatureDealtDamageByFilteredSourceThisTurn {
            victim,
            damager_filter,
        } => {
            Trigger::creature_dealt_damage_by_filtered_source_this_turn_dies(victim, damager_filter)
        }
        TriggerSpec::SpellCast {
            filter,
            mana_source_filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => Trigger::spell_cast_qualified_with_mana_source(
            filter,
            mana_source_filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        ),
        TriggerSpec::NthSpellOfTurnCast { spell_number } => {
            Trigger::nth_spell_of_turn_cast(spell_number)
        }
        TriggerSpec::SpellCopied { filter, copier } => Trigger::spell_copied(filter, copier),
        TriggerSpec::SpellCountered { filter, controller } => {
            Trigger::spell_countered(filter, controller)
        }
        TriggerSpec::EntersBattlefield {
            mut filter,
            cause_filter,
            origin_condition,
            during_turn,
        } => {
            let during_turn_surface = during_turn.as_ref().and_then(|player| match player {
                PlayerFilter::You => Some(" during your turn"),
                PlayerFilter::Opponent => Some(" during an opponent's turn"),
                _ => None,
            });
            if filter.source {
                filter.source = false;
                let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .to(crate::zone::Zone::Battlefield)
                    .filter(filter)
                    .this()
                    .cause_filter(cause_filter);
                if let Some(origin_condition) = origin_condition {
                    trigger = trigger.origin_condition(origin_condition);
                }
                if let Some(during_turn) = during_turn {
                    trigger = trigger.during_turn(during_turn);
                }
                Trigger::new(trigger)
            } else if let Some(origin_condition) = origin_condition {
                let display = format!(
                    "Whenever {} enters the battlefield{}{}",
                    indefinite_subject_description(filter.description()),
                    origin_condition.display_suffix(false),
                    during_turn_surface.unwrap_or_default(),
                );
                let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .to(crate::zone::Zone::Battlefield)
                    .filter(filter)
                    .cause_filter(cause_filter)
                    .origin_condition(origin_condition);
                if let Some(during_turn) = during_turn {
                    trigger = trigger.during_turn(during_turn);
                }
                Trigger::new(trigger).with_display_label(display)
            } else {
                let display = if filter.has_player_puts_onto_battlefield_surface() {
                    format!(
                        "Whenever a player puts {} onto the battlefield{}",
                        indefinite_subject_description(filter.description()),
                        during_turn_surface.unwrap_or_default(),
                    )
                } else {
                    format!(
                        "Whenever {} enters the battlefield{}",
                        indefinite_subject_description(filter.description()),
                        during_turn_surface.unwrap_or_default(),
                    )
                };
                let mut trigger = crate::triggers::zone_changes::ZoneChangeTrigger::new()
                    .to(crate::zone::Zone::Battlefield)
                    .filter(filter)
                    .cause_filter(cause_filter);
                if let Some(during_turn) = during_turn {
                    trigger = trigger.during_turn(during_turn);
                }
                Trigger::new(trigger).with_display_label(display)
            }
        }
        TriggerSpec::EntersBattlefieldOneOrMore {
            filter,
            cause_filter,
            origin_condition,
        } => {
            if let Some(origin_condition) = origin_condition {
                let display = format!(
                    "Whenever {} enter the battlefield{}",
                    one_or_more_subject_description(&filter),
                    origin_condition.display_suffix(true),
                );
                Trigger::new(
                    crate::triggers::zone_changes::ZoneChangeTrigger::new()
                        .to(crate::zone::Zone::Battlefield)
                        .filter(filter)
                        .count(crate::triggers::CountMode::OneOrMore)
                        .cause_filter(cause_filter)
                        .origin_condition(origin_condition),
                )
                .with_display_label(display)
            } else {
                let display = format!(
                    "Whenever {} enter the battlefield",
                    one_or_more_subject_description(&filter),
                );
                Trigger::enters_battlefield_one_or_more(filter, cause_filter)
                    .with_display_label(display)
            }
        }
        TriggerSpec::EntersBattlefieldFromZone {
            mut filter,
            from,
            owner,
            one_or_more,
            cause_filter,
        } => {
            if let Some(owner) = owner {
                filter.owner = Some(owner);
            }
            let trigger = crate::triggers::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Battlefield)
                .filter(filter)
                .cause_filter(cause_filter);
            if one_or_more {
                Trigger::new(trigger.count(crate::triggers::CountMode::OneOrMore))
            } else {
                Trigger::new(trigger)
            }
        }
        TriggerSpec::EntersBattlefieldTapped {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield_tapped(filter, cause_filter),
        TriggerSpec::EntersBattlefieldUntapped {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield_untapped(filter, cause_filter),
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfDrawStep(player) => Trigger::beginning_of_draw_step(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfTheEndStep => Trigger::beginning_of_the_end_step(),
        TriggerSpec::BeginningOfMonarchEndStep => Trigger::beginning_of_monarch_end_step(),
        TriggerSpec::BeginningOfMainPhase { player, surface } => {
            Trigger::beginning_of_main_phase_with_surface(player, surface)
        }
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::BeginningOfPostcombatMain { player, surface } => {
            Trigger::beginning_of_postcombat_main_phase_with_surface(player, surface)
        }
        TriggerSpec::DayNightChanged => Trigger::day_night_changed(),
        TriggerSpec::ThisEntersBattlefield { origin_condition } => match origin_condition {
            None => Trigger::this_enters_battlefield(),
            Some(origin_condition) => Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .to(crate::zone::Zone::Battlefield)
                    .this()
                    .origin_condition(origin_condition),
            ),
        },
        TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface,
            subject_number,
            origin_condition,
        } => {
            let mut trigger = crate::triggers::ZoneChangeTrigger::new()
                .to(crate::zone::Zone::Battlefield)
                .this()
                .this_surface(surface.clone())
                .this_subject_number(subject_number);
            if let Some(origin_condition) = origin_condition {
                trigger = trigger.origin_condition(origin_condition);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::ThisEntersBattlefieldFromZone {
            mut subject_filter,
            from,
            owner,
        } => {
            if let Some(owner) = owner {
                subject_filter.owner = Some(owner);
            }
            Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(from)
                    .to(crate::zone::Zone::Battlefield)
                    .filter(subject_filter)
                    .this(),
            )
        }
        TriggerSpec::ThisTransforms { destination_name } => {
            Trigger::transforms_with_destination(destination_name.clone())
        }
        TriggerSpec::ThisTransformsWithSurface {
            surface,
            destination_name,
        } => Trigger::transforms_with_surface_and_destination(
            surface.clone(),
            destination_name.clone(),
        ),
        TriggerSpec::ThisDealsCombatDamageToPlayer {
            player,
            source_surface,
        } => match source_surface {
            Some(surface) => Trigger::this_deals_combat_damage_to_player_with_surface(
                player.clone(),
                surface.clone(),
            ),
            None => Trigger::this_deals_combat_damage_to_player(player.clone()),
        },
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
            Trigger::deals_combat_damage_to_player(source, player)
        }
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => {
            Trigger::deals_combat_damage_to_player_one_or_more(source, player)
        }
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction {
            action,
            player,
            source_filter,
            during_your_turn,
        } => match (source_filter, during_your_turn) {
            (Some(filter), true) => {
                Trigger::keyword_action_matching_object_during_your_turn(action, player, filter)
            }
            (Some(filter), false) => {
                Trigger::keyword_action_matching_object(action, player, filter)
            }
            (None, true) => Trigger::keyword_action_during_your_turn(action, player),
            (None, false) => Trigger::keyword_action(action, player),
        },
        TriggerSpec::KeywordActionTaggedObject {
            action,
            player,
            source_filter,
            object_tag,
            object_filter,
            during_your_main_phase,
        } => {
            if during_your_main_phase {
                Trigger::keyword_action_matching_source_and_tagged_object_during_your_main_phase(
                    action,
                    player,
                    source_filter,
                    object_tag.key.clone(),
                    object_filter,
                )
            } else {
                Trigger::keyword_action_matching_source_and_tagged_object(
                    action,
                    player,
                    source_filter,
                    object_tag.key.clone(),
                    object_filter,
                )
            }
        }
        TriggerSpec::KeywordActionFromSource { action, player } => {
            Trigger::keyword_action_from_source(action, player)
        }
        TriggerSpec::WinsClash { player, surface } => {
            Trigger::wins_clash_with_surface(player, surface)
        }
        TriggerSpec::Expend { player, amount } => Trigger::expend(amount, player),
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::FinalChapterAbilityResolved(filter) => {
            Trigger::final_chapter_ability_resolved(filter)
        }
        TriggerSpec::HauntedCreatureDies => Trigger::custom(
            "haunted_creature_dies",
            "When the creature it haunts dies".to_string(),
        ),
        TriggerSpec::Either(left, right) => {
            let display = describe_damage_to_object_and_player_union(&left, &right);
            let source_and_or_other_display =
                describe_source_and_or_one_or_more_other_enters(&left, &right);
            let repeated_intro_display = repeated_intro_branch_description(&left)
                .zip(repeated_intro_branch_description(&right))
                .map(|(left, right)| {
                    let right = after_exact_prefix(&right, "Whenever ")
                        .map(|tail| format!("whenever {tail}"))
                        .or_else(|| {
                            after_exact_prefix(&right, "When ").map(|tail| format!("when {tail}"))
                        })
                        .or_else(|| {
                            after_exact_prefix(&right, "At ").map(|tail| format!("at {tail}"))
                        })
                        .unwrap_or(right);
                    format!("{left} and {right}")
                });
            let trigger =
                Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right));
            if let Some(display) = display
                .or(source_and_or_other_display)
                .or(repeated_intro_display)
            {
                trigger.with_display_label(display)
            } else {
                trigger
            }
        }
    }
}

pub fn ensure_concrete_trigger_spec(trigger: &TriggerSpec) -> Result<(), CardTextError> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => ensure_concrete_trigger_spec(trigger),
        TriggerSpec::Either(left, right) => {
            ensure_concrete_trigger_spec(left)?;
            ensure_concrete_trigger_spec(right)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn trigger_binds_iterated_player(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_binds_iterated_player(trigger),
        TriggerSpec::SpellCast { .. }
        | TriggerSpec::NthSpellOfTurnCast { .. }
        | TriggerSpec::SpellCopied { .. }
        | TriggerSpec::SpellCountered { .. }
        | TriggerSpec::PlayerLosesLife(_)
        | TriggerSpec::PlayersLoseLifeOneOrMore(_)
        | TriggerSpec::OpponentsEachLoseExactLife { .. }
        | TriggerSpec::PlayerLosesGame(_)
        | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
        | TriggerSpec::PlayerDrawsCard(_)
        | TriggerSpec::PlayerDrawsCardNotDuringTurn { .. }
        | TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(_)
        | TriggerSpec::PlayerDrawsNthCardEachTurn { .. }
        | TriggerSpec::PlayerDrawsNumberedCardsEachTurn { .. }
        | TriggerSpec::PlayerDiscardsCard { .. }
        | TriggerSpec::PlayerRevealsCard { .. }
        | TriggerSpec::PlayerPlaysLand { .. }
        | TriggerSpec::PlayerGivesGift(_)
        | TriggerSpec::PlayerSearchesLibrary(_)
        | TriggerSpec::PlayerShufflesLibrary { .. }
        | TriggerSpec::PlayerTapsForMana { .. }
        | TriggerSpec::PlayerRollsResult { .. }
        | TriggerSpec::PlayerRollsHighestNaturalResult { .. }
        | TriggerSpec::PlayerRollsDie { .. }
        | TriggerSpec::PlayerCoinFlipResult { .. }
        | TriggerSpec::PlayerSacrifices { .. }
        | TriggerSpec::TokensCreated { .. }
        | TriggerSpec::ThisDealsDamageToPlayer { .. }
        | TriggerSpec::DealsDamageToPlayer { .. }
        | TriggerSpec::DealsExactDamageToObjectOrPlayer { .. }
        | TriggerSpec::DealsNoncombatDamageToPlayer { .. }
        | TriggerSpec::ThisDealsCombatDamageToPlayer { .. }
        | TriggerSpec::DealsCombatDamageToPlayer { .. }
        | TriggerSpec::BeginningOfUpkeep(_)
        | TriggerSpec::BeginningOfDrawStep(_)
        | TriggerSpec::BeginningOfCombat(_)
        | TriggerSpec::BeginningOfEndStep(_)
        | TriggerSpec::BeginningOfTheEndStep
        | TriggerSpec::BeginningOfMonarchEndStep
        | TriggerSpec::BeginningOfMainPhase { .. }
        | TriggerSpec::BeginningOfPrecombatMain(_)
        | TriggerSpec::BeginningOfPostcombatMain { .. }
        | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
        | TriggerSpec::KeywordAction { .. }
        | TriggerSpec::KeywordActionTaggedObject { .. }
        | TriggerSpec::KeywordActionFromSource { .. }
        | TriggerSpec::WinsClash { .. }
        | TriggerSpec::Expend { .. } => true,
        TriggerSpec::StateBased { .. } => false,
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        } => *source_controller != PlayerFilter::Any,
        // A card put into an owner-restricted graveyard binds "that player" to the
        // card's owner (e.g. "a card is put into an opponent's graveyard, … have
        // that player lose 2 life"); the runtime resolves it from the zone-change
        // event's owner. A graveyard with no owner restriction is ambiguous.
        TriggerSpec::PutIntoGraveyard(filter) | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            filter.owner.is_some()
        }
        TriggerSpec::Either(left, right) => {
            trigger_binds_iterated_player(left) && trigger_binds_iterated_player(right)
        }
        _ => false,
    }
}

pub use crate::trigger_players::inferred_trigger_player_filter;

pub fn trigger_binds_player_reference_context(trigger: &TriggerSpec) -> bool {
    trigger_binds_iterated_player(trigger)
        || inferred_trigger_player_filter(trigger)
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
}

pub fn trigger_supports_event_value(trigger: &TriggerSpec, spec: &EventValueSpec) -> bool {
    match spec {
        EventValueSpec::Amount | EventValueSpec::LifeAmount => match trigger {
            TriggerSpec::WithIntro { trigger, .. } => trigger_supports_event_value(trigger, spec),
            TriggerSpec::SpellCast {
                filter: Some(filter),
                ..
            } if spell_cast_filter_binds_target_count(filter) => true,
            TriggerSpec::YouGainLife
            | TriggerSpec::YouGainLifeCausedBy(_)
            | TriggerSpec::YouGainLifeDuringTurn(_)
            | TriggerSpec::PlayerLosesLife(_)
            | TriggerSpec::PlayersLoseLifeOneOrMore(_)
            | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
            | TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::ThisIsDealtCombatDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::IsDealtCombatDamage(_)
            | TriggerSpec::IsDealtExcessNoncombatDamage(_)
            | TriggerSpec::ThisDealsDamage
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::ThisDealsDamageToPlayer { .. }
            | TriggerSpec::DealsDamage { .. }
            | TriggerSpec::DealsDamageTo { .. }
            | TriggerSpec::DealsDamageToPlayer { .. }
            | TriggerSpec::DealsExactDamageToObjectOrPlayer { .. }
            | TriggerSpec::DealsNoncombatDamageToPlayer { .. }
            | TriggerSpec::ThisDealsCombatDamage
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsCombatDamage(_)
            | TriggerSpec::DealsCombatDamageTo { .. }
            | TriggerSpec::ThisDealsCombatDamageToPlayer { .. }
            | TriggerSpec::DealsCombatDamageToPlayer { .. }
            | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
            | TriggerSpec::AttacksOneOrMore(_)
            | TriggerSpec::AttacksOneOrMoreWithMinTotal { .. }
            | TriggerSpec::AttacksOneOrMoreWithExactTotal { .. }
            | TriggerSpec::AttacksOneOrMoreWithAggregate { .. }
            | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
            | TriggerSpec::KeywordAction { .. }
            | TriggerSpec::KeywordActionTaggedObject { .. }
            | TriggerSpec::KeywordActionFromSource { .. }
            | TriggerSpec::CounterPutOn { .. }
            | TriggerSpec::NthCounterPutOn { .. }
            | TriggerSpec::CounterRemovedFrom { .. }
            | TriggerSpec::TokensCreated { .. }
            | TriggerSpec::EntersBattlefieldOneOrMore { .. } => true,
            TriggerSpec::PutIntoExileFromZones { one_or_more, .. } => *one_or_more,
            TriggerSpec::PlayerDiscardsCard { one_or_more, .. } => *one_or_more,
            TriggerSpec::StateBased { .. } => false,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
        EventValueSpec::BlockersBeyondFirst { .. } => match trigger {
            TriggerSpec::WithIntro { trigger, .. } => trigger_supports_event_value(trigger, spec),
            TriggerSpec::ThisBecomesBlocked
            | TriggerSpec::BecomesBlocked(_)
            | TriggerSpec::ThisBecomesBlockedByObject(_) => true,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
    }
}

fn spell_cast_filter_binds_target_count(filter: &crate::target::ObjectFilter) -> bool {
    filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_player.is_some()
        || filter.targets_only_object.is_some()
        || filter.target_count.is_some()
        || filter
            .any_of
            .iter()
            .any(spell_cast_filter_binds_target_count)
}

pub fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let lowered =
        compile_trigger_effects_with_imports(trigger, effects, &ReferenceImports::default())?;
    Ok((lowered.effects.to_vec(), lowered.choices))
}

pub fn compile_trigger_effects_with_imports(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared =
        super::stage_effects_with_trigger_context_for_lowering(trigger, effects, imports.clone())?;
    super::materialize_prepared_effects_with_trigger_context(&prepared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graveyard_surface(trigger: TriggerSpec) -> crate::triggers::GraveyardTriggerSurface {
        let trigger = compile_trigger_spec(trigger);
        let crate::triggers::TriggerKind::ZoneChange(zone_change) = trigger.kind else {
            panic!("authored graveyard trigger did not lower to a zone-change trigger");
        };
        zone_change
            .graveyard_surface
            .expect("authored graveyard wording should be retained")
    }

    #[test]
    fn authored_graveyard_wording_survives_trigger_lowering() {
        assert_eq!(
            graveyard_surface(TriggerSpec::Dies(
                crate::target::ObjectFilter::planeswalker()
            )),
            crate::triggers::GraveyardTriggerSurface::Dies
        );
        assert_eq!(
            graveyard_surface(TriggerSpec::PutIntoGraveyardFromZone {
                filter: crate::target::ObjectFilter::creature(),
                from: crate::zone::Zone::Battlefield,
                one_or_more: false,
            }),
            crate::triggers::GraveyardTriggerSurface::PutIntoGraveyard
        );
    }

    #[test]
    fn each_player_phase_triggers_bind_relative_players_from_the_event() {
        for trigger in [
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
            TriggerSpec::BeginningOfDrawStep(PlayerFilter::Any),
            TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
            TriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
            TriggerSpec::BeginningOfPrecombatMain(PlayerFilter::Any),
            TriggerSpec::BeginningOfPostcombatMain {
                player: PlayerFilter::Any,
                surface: ironsmith_core::trigger_model::PostcombatMainPhaseSurface::PostcombatMain,
            },
        ] {
            assert_eq!(
                inferred_trigger_player_filter(&trigger),
                Some(PlayerFilter::IteratedPlayer),
                "phase trigger must preserve its concrete event participant: {trigger:?}"
            );
            assert!(trigger_binds_player_reference_context(&trigger));
        }
    }

    #[test]
    fn non_phase_any_filter_does_not_claim_a_player_iteration_scope() {
        let trigger = TriggerSpec::WinsClash {
            player: PlayerFilter::Any,
            surface: ironsmith_core::ClashWinTriggerSurface::WinAClash,
        };
        assert_eq!(
            inferred_trigger_player_filter(&trigger),
            Some(PlayerFilter::Active)
        );
    }
}
