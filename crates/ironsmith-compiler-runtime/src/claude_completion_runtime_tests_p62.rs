//! Claude package 62 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p62_find_effects<T: Clone + 'static>(effect: &Effect, found: &mut Vec<T>) {
    if let Some(hit) = effect.downcast_ref::<T>() {
        found.push(hit.clone());
    }
    effect.visit_child_effects(&mut |child| p62_find_effects(child, found));
}

fn p62_triggered_effects<T: Clone + 'static>(definition: &ironsmith::cards::CardDefinition) -> Vec<T> {
    let mut found = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                p62_find_effects(effect, &mut found);
            }
        }
    }
    found
}

fn p62_spell_effects<T: Clone + 'static>(definition: &ironsmith::cards::CardDefinition) -> Vec<T> {
    let mut found = Vec::new();
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p62_find_effects(effect, &mut found);
    }
    found
}

const GUILD_FEUD: &str = "Type: Enchantment\nAt the beginning of your upkeep, target opponent reveals the top three cards of their library, may put a creature card from among them onto the battlefield, then puts the rest into their graveyard. You do the same with the top three cards of your library. If two creatures are put onto the battlefield this way, those creatures fight each other.";

#[test]
fn guild_feud_each_participant_reveals_puts_and_fights() {
    use ironsmith::target::PlayerFilter;
    let definition = compile_to_runtime_definition("Guild Feud", GUILD_FEUD, false).unwrap();
    let choices = p62_triggered_effects::<ironsmith::effects::ChooseObjectsEffect>(&definition);
    assert_eq!(choices.len(), 2, "{definition:#?}");
    // The opponent chooses among their own revealed cards, you among yours.
    assert!(choices.iter().any(|choice| choice.chooser == PlayerFilter::You), "{choices:?}");
    assert!(choices.iter().any(|choice| choice.chooser != PlayerFilter::You), "{choices:?}");
    let fights = p62_triggered_effects::<ironsmith::effects::FightEffect>(&definition);
    assert_eq!(fights.len(), 1, "{definition:#?}");
    assert_ne!(fights[0].creature1, fights[0].creature2, "each participant's arrival fights the other");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Graveyard"), "the rest go to the graveyard: {debug}");
}

const HAMA: &str = "Type: Legendary Creature — Human Warlock\nWhen Hama enters, target opponent mills three cards. Exile up to one noncreature, nonland card from that player's graveyard. For as long as you control Hama, you may cast the exiled card during your turn by waterbending {X} rather than paying its mana cost, where X is its mana value. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)";

#[test]
fn hama_grants_turn_limited_waterbend_mana_value_price() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition =
        compile_to_runtime_definition("Hama, the Bloodbender", HAMA, false).unwrap();
    let grants = p62_triggered_effects::<ironsmith::effects::GrantPlayTaggedEffect>(&definition);
    assert_eq!(grants.len(), 1, "{definition:#?}");
    let grant = &grants[0];
    assert_eq!(grant.duration, ironsmith_core::GrantPlayTaggedDuration::ForAsLongAsYouControlSource);
    assert!(grant.during_your_turns, "{grant:?}");
    assert!(grant.alternative_cost_x_is_mana_value, "{grant:?}");
    let price = grant.alternative_cost.as_ref().expect("waterbend price");
    let mana = price.costs().iter().find_map(|cost| cost.mana_cost_ref()).expect("mana price");
    assert!(mana.has_waterbend_obligation() && mana.has_x(), "{mana:?}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hama = CardBuilder::new(CardId::new(), "Hama source").card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_card(&hama, alice, Zone::Battlefield);
    let sorcery = CardBuilder::new(CardId::new(), "Exiled sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Blue]]))
        .build();
    let exiled = game.create_object_from_card(&sorcery, PlayerId::from_index(1), Zone::Exile);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.tag_objects(grant.tag.clone(), vec![snapshot]);
    let before = game.effect_store.grant_registry.grants.len();
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grant.clone()), &mut ctx).unwrap();
    let created = &game.effect_store.grant_registry.grants[before..];
    assert!(!created.is_empty());
    assert!(created.iter().all(|created| created.usage_limit
        == Some(ironsmith::grant::GrantUsageLimit::DuringYourTurns)), "{created:?}");
    let debug = format!("{created:?}");
    // X is the card's mana value (4), paid by waterbending.
    assert!(debug.contains("Generic(4)"), "{debug}");
    assert!(!debug.contains(" X]") && !debug.contains("[X"), "{debug}");
}

const GRELL_PHILOSOPHER: &str = "Type: Creature — Horror Wizard\nAberrant Tinkering — When this creature enters and at the beginning of your upkeep, each Horror you control gains all activated abilities of target artifact an opponent controls until end of turn. You may spend blue mana as though it were mana of any color to activate those abilities.";

#[test]
fn grell_philosopher_horrors_copy_artifact_abilities_payable_with_blue() {
    use ironsmith::ability::{Ability, AbilityKind};
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::{PlayerId, Zone};
    let definition =
        compile_to_runtime_definition("Grell Philosopher", GRELL_PHILOSOPHER, false).unwrap();
    let grants = p62_triggered_effects::<ironsmith::effects::GrantAbilitiesTargetEffect>(&definition);
    assert!(!grants.is_empty(), "{definition:#?}");
    let grant = &grants[0];
    assert!(matches!(grant.target.base(), ChooseSpec::All(_)), "{:?}", grant.target);
    assert!(grant.activated_from.is_some());
    assert_eq!(grant.activated_from_symbol_as_any_color, Some(ManaSymbol::Blue));

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let horror = CardBuilder::new(CardId::new(), "Horror")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .build();
    let first = game.create_object_from_card(&horror, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&horror, alice, Zone::Battlefield);
    let artifact = CardBuilder::new(CardId::new(), "Opposing artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
    let red = ManaCost::from_pips(vec![vec![ManaSymbol::Red]]);
    game.object_mut(artifact).unwrap().abilities = std::sync::Arc::new(vec![Ability::activated(
        ironsmith::cost::TotalCost::mana(red),
        Vec::<Effect>::new(),
    )]);
    let mut ctx = ironsmith::effects::EffectContext::new_default(first, alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(artifact)]);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grant.clone()), &mut ctx).unwrap();
    game.refresh_continuous_state().unwrap();
    let activated_pips = |id| {
        game.calculated_characteristics(id)
            .unwrap()
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => activated
                    .mana_cost
                    .costs()
                    .iter()
                    .find_map(|cost| cost.mana_cost_ref())
                    .map(|mana| mana.pips().to_vec()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // Both Horrors received the copy; only the copies accept blue for red.
    for horror in [first, second] {
        assert_eq!(
            activated_pips(horror),
            vec![vec![vec![ManaSymbol::Red, ManaSymbol::Blue]]],
            "{horror:?}"
        );
    }
    assert_eq!(activated_pips(artifact), vec![vec![vec![ManaSymbol::Red]]]);
}

const GOBLIN_GAME: &str = "Type: Sorcery\nEach player hides at least one item, then all players reveal them simultaneously. Each player loses life equal to the number of items they revealed. The player who revealed the fewest items then loses half their life, rounded up. If two or more players are tied for fewest, each loses half their life, rounded up.";

#[test]
fn goblin_game_hidden_item_counts_drive_life_loss() {
    let definition = compile_to_runtime_definition("Goblin Game", GOBLIN_GAME, false).unwrap();
    let choices = p62_spell_effects::<ironsmith_core::ChoosePlayerOptionEffect>(&definition);
    assert_eq!(choices.len(), 1, "{definition:#?}");
    assert!(choices[0].secret, "{:?}", choices[0]);
    assert_eq!(choices[0].number, Some(ironsmith_core::PlayerNumberChoice { min: 1, max: None }));
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("IteratedPlayerChosenNumber"), "{debug}");
    assert!(debug.contains("HalfLifeTotalRoundedUp"), "{debug}");
    assert!(debug.contains("__player_number_choice__:lowest"), "{debug}");
    // The tie sentence restates the fewest-player loss; half life is lost once.
    assert_eq!(debug.matches("HalfLifeTotalRoundedUp").count(), 1, "{debug}");
}
