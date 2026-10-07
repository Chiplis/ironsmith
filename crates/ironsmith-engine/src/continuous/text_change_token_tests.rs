//! Source-authored contracts; execution deferred with the text campaign.
use super::*;
use crate::ability::ProtectionFrom;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::CardDefinition;
use crate::continuous::{ContinuousEffect, Modification};
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{EffectContext, execute_effect};
use crate::game_state::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use ironsmith_core::{CardType, Color, ColorSet, Subtype, TokenNameTextRole, TokenTextRoles, TokenWordRole, Zone};

fn all_colors() -> ColorSet { ColorSet::WHITE.union(ColorSet::BLUE).union(ColorSet::BLACK).union(ColorSet::RED).union(ColorSet::GREEN) }

fn template(name: &str, role: TokenNameTextRole) -> CreateTokenEffect {
    let card = CardBuilder::new(CardId::new(), name).token().card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf]).color_indicator(ColorSet::RED)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let ability = Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(ColorSet::GREEN)));
    CreateTokenEffect::one(CardDefinition::with_abilities(card, vec![ability]))
        .with_text_roles(TokenTextRoles::authored(role, 1))
}

#[test]
fn future_token_type_changes_derive_names_but_explicit_names_and_symbols_remain_exact() {
    for role in [TokenNameTextRole::SubtypeDerived, TokenNameTextRole::Explicit] {
        let original = template("Elf", role);
        let id = original.token.card.id;
        let ability_id = match &original.token.abilities[0].kind { AbilityKind::Static(ability) => ability.instance_id(), _ => unreachable!() };
        let source = Effect::new(original.clone());
        let changed = source.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.id, id);
        assert_eq!(changed.token.card.subtypes, vec![Subtype::Human]);
        assert_eq!(changed.token.card.name, if role == TokenNameTextRole::Explicit { "Elf" } else { "Human Token" });
        assert_eq!(changed.token.card.mana_cost, original.token.card.mana_cost);
        let AbilityKind::Static(ability) = &changed.token.abilities[0].kind else { unreachable!() };
        assert_eq!(ability.instance_id(), ability_id);
        assert_eq!(original.token.card.subtypes, vec![Subtype::Elf]);
        assert_eq!(source.downcast_ref::<CreateTokenEffect>().unwrap().token.card.name, "Elf");
    }
}

#[test]
fn word_roles_separate_color_lists_and_quoted_abilities_from_implied_values() {
    let red_to_blue = TextChange::color(Color::Red, Color::Blue).unwrap();
    let green_to_blue = TextChange::color(Color::Green, Color::Blue).unwrap();
    let mut literal = template("Unchanged explicit Green", TokenNameTextRole::Explicit);
    literal.token.card.color_indicator = Some(all_colors());
    let mut implied = literal.clone();
    implied.text_roles = Some(TokenTextRoles::rules_implied(TokenNameTextRole::Explicit, 1));
    let literal = Effect::new(literal).with_text_change(red_to_blue).unwrap();
    let implied = Effect::new(implied).with_text_change(red_to_blue).unwrap();
    assert!(!literal.downcast_ref::<CreateTokenEffect>().unwrap().token.card.color_indicator.unwrap().contains(Color::Red));
    assert_eq!(implied.downcast_ref::<CreateTokenEffect>().unwrap().token.card.color_indicator, Some(all_colors()));
    let literal = literal.with_text_change(green_to_blue).unwrap();
    let implied = implied.with_text_change(green_to_blue).unwrap();
    for (effect, expected) in [(&literal, ColorSet::BLUE), (&implied, ColorSet::GREEN)] {
        let token = &effect.downcast_ref::<CreateTokenEffect>().unwrap().token;
        let AbilityKind::Static(protection) = &token.abilities[0].kind else { unreachable!() };
        assert_eq!(protection.protection_from(), Some(&ProtectionFrom::Color(expected)));
        assert_eq!(token.card.name, "Unchanged explicit Green");
    }
}

#[test]
fn existing_created_token_changes_subtype_without_rederiving_its_name_or_color() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let player = PlayerId::from_index(0);
    let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Creator").card_types(vec![CardType::Artifact]).build(), player, Zone::Battlefield);
    let original = Effect::new(template("Unused template label", TokenNameTextRole::SubtypeDerived));
    let changed = original.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap()).unwrap();
    let mut dm = SelectFirstDecisionMaker;
    let outcome = execute_effect(&mut game, &changed, &mut EffectContext::new(source, player, &mut dm)).unwrap();
    let crate::effect::OutcomeValue::Objects(tokens) = outcome.value else { panic!("created token result"); };
    assert_eq!(tokens.len(), 1);
    let token = tokens[0];
    assert_eq!(game.object(token).unwrap().name.as_ref(), "Human Token");
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(source, player, vec![token],
        Modification::RewriteText(TextChange::creature_type(Subtype::Human, Subtype::Zombie).unwrap())));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(source, player, vec![token],
        Modification::RewriteText(TextChange::color(Color::Red, Color::Blue).unwrap())));
    game.refresh_continuous_state().unwrap();
    let current = game.calculated_characteristics(token).unwrap();
    assert_eq!(current.subtypes.to_vec(), vec![Subtype::Zombie]);
    assert_eq!(current.name.as_ref(), "Human Token");
    assert_eq!(current.colors, ColorSet::RED, "an existing token's actual color is not a color word");
}

#[test]
fn missing_or_misaligned_token_roles_reject_atomically() {
    for missing in [true, false] {
        let mut model = template("Elf", TokenNameTextRole::SubtypeDerived);
        if missing { model.text_roles = None; }
        else { model.text_roles.as_mut().unwrap().abilities.push(TokenWordRole::Authored); }
        let effect = Effect::new(model);
        assert!(matches!(effect.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap()), Err(Error::TokenDefinition)));
        assert_eq!(effect.downcast_ref::<CreateTokenEffect>().unwrap().token.card.subtypes, vec![Subtype::Elf]);
    }
}

#[test]
fn a_copy_of_the_changed_creator_spell_uses_its_pre_text_token_blueprint() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let player = PlayerId::from_index(0);
    let effect = Effect::new(template("Template label", TokenNameTextRole::SubtypeDerived));
    let card = CardBuilder::new(CardId::new(), "Creator spell").card_types(vec![CardType::Sorcery]).build();
    let source = game.create_object_from_definition(&CardDefinition::spell(card, vec![effect]), player, Zone::Stack);
    game.stack.push(crate::game_state::StackEntry::new(source, player));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(source, player, vec![source],
        Modification::RewriteText(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap())));
    game.refresh_continuous_state().unwrap();
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(&mut game, &Effect::new(CopySpellEffect::single(crate::target::ChooseSpec::Source)),
        &mut EffectContext::new(source, player, &mut dm)).unwrap();
    assert_eq!(game.stack.len(), 2);
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.battlefield.len(), 1);
    assert_eq!(game.object(game.battlefield[0]).unwrap().name.as_ref(), "Elf Token");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.battlefield.len(), 2);
    assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Human Token"));
    assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Elf Token"));
}

#[test]
fn unknown_ability_words_hold_rewriting_without_losing_the_proven_creation_name_policy() {
    let mut token = template("Legacy label", TokenNameTextRole::SubtypeDerived);
    token.text_roles.as_mut().unwrap().abilities[0] = TokenWordRole::Unrecorded;
    let effect = Effect::new(token);
    assert!(matches!(effect.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap()), Err(Error::TokenDefinition)));
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let player = PlayerId::from_index(0);
    let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Creator").card_types(vec![CardType::Artifact]).build(), player, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let outcome = execute_effect(&mut game, &effect, &mut EffectContext::new(source, player, &mut dm)).unwrap();
    let crate::effect::OutcomeValue::Objects(tokens) = outcome.value else { panic!("created token"); };
    assert_eq!(game.object(tokens[0]).unwrap().name.as_ref(), "Elf Token");
}
