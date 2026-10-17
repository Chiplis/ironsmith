//! Claude package 38 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::{Ability, AbilityKind, ProtectionFrom};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::CardType;
use ironsmith::{Color, ColorSet, PlayerId, Zone};

const P38_SWIRL_THE_MISTS: &str = "Mana cost: {2}{U}{U}\nType: Enchantment\nAs this enchantment enters, choose a color word.\nAll instances of color words in the text of spells and permanents are changed to the chosen color word.";

/// An instruction outside the typed text-change domain, with no color word.
#[derive(Debug, Clone)]
struct P38WordlessNative;
impl ironsmith::effects::EffectExecutor for P38WordlessNative {
    fn execute(&self, _: &mut ironsmith::GameState, _: &mut ironsmith::effects::EffectContext)
        -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError>
    { Ok(ironsmith::effect::EffectOutcome::resolved()) }
}

fn p38_permanent(game: &mut ironsmith::GameState, name: &str, abilities: Vec<Ability>) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let id = game.create_object_from_card(&card, PlayerId::from_index(1), Zone::Battlefield);
    game.object_mut(id).unwrap().abilities = std::sync::Arc::new(abilities);
    id
}

#[test]
fn swirl_the_mists_rewrites_every_other_color_word_and_leaves_wordless_text_unaffected() {
    let definition = compile_to_runtime_definition("Swirl the Mists", P38_SWIRL_THE_MISTS, false)
        .expect("the color-word choice and the global color-word change should compile");
    let ids: Vec<StaticAbilityId> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => Some(ability.id()),
        _ => None,
    }).collect();
    assert!(ids.contains(&StaticAbilityId::ChooseColorAsEnters), "choose a color word: {ids:?}");
    assert!(ids.contains(&StaticAbilityId::AllColorWordsBecomeChosenColor), "global change: {ids:?}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let swirl = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_color(swirl, Color::Green);

    // Protection from black and "black creatures get +1/+1" both hold words.
    let mut black_creatures = ironsmith::target::ObjectFilter::creature();
    black_creatures.colors = Some(ColorSet::BLACK);
    let pump = Effect::new(ironsmith::effects::ModifyPowerToughnessAllEffect {
        filter: black_creatures,
        power: ironsmith::effect::Value::Fixed(1),
        toughness: ironsmith::effect::Value::Fixed(1),
        duration: ironsmith::effect::Until::EndOfTurn,
    });
    let knight = p38_permanent(&mut game, "Swirl knight", vec![
        Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(ColorSet::BLACK))),
        Ability::activated(ironsmith::cost::TotalCost::free(), vec![pump]),
    ]);
    // Text outside the typed domain without any color word is unaffected.
    let rock = p38_permanent(&mut game, "Swirl rock", vec![
        Ability::activated(ironsmith::cost::TotalCost::free(), vec![Effect::new(P38WordlessNative)]),
    ]);

    game.refresh_continuous_state().expect("a global color-word change must not fail wordless objects");
    let knight_chars = game.calculated_characteristics(knight).unwrap();
    let protections: Vec<_> = knight_chars.static_abilities.iter()
        .filter_map(|ability| ability.protection_from().cloned()).collect();
    assert_eq!(protections, vec![ProtectionFrom::Color(ColorSet::GREEN)], "black became green");
    let pumped = knight_chars.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => activated.effects.all_effects().into_iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::ModifyPowerToughnessAllEffect>().map(|model| model.filter.colors)),
        _ => None,
    });
    assert_eq!(pumped, Some(Some(ColorSet::GREEN)), "the P/T instruction's color word changed too");
    let rock_chars = game.calculated_characteristics(rock).unwrap();
    assert_eq!(rock_chars.abilities.len(), 1, "the wordless object keeps its text");
    let swirl_chars = game.calculated_characteristics(swirl).unwrap();
    assert!(swirl_chars.static_abilities.iter().any(|ability|
        ability.id() == StaticAbilityId::AllColorWordsBecomeChosenColor), "Swirl names no color word itself");
}

const P38_SHARED_FATE: &str = "Mana cost: {4}{U}\nType: Enchantment\nIf a player would draw a card, that player exiles the top card of one of their opponents' libraries face down instead.\nEach player may look at cards they exiled with this enchantment, and they may play lands and cast spells from among those cards.";
const P38_UBA_MASK: &str = "Mana cost: {4}\nType: Artifact\nIf a player would draw a card, that player exiles that card face up instead.\nEach player may play lands and cast spells from among cards they exiled with this artifact this turn.";

/// Exile `card` with `source` as the exile performed by `actor`.
fn p38_exile_by(game: &mut ironsmith::GameState, source: ironsmith::ids::ObjectId, controller: PlayerId,
    card: ironsmith::ids::ObjectId, actor: PlayerId, face_down: bool) -> ironsmith::ids::ObjectId
{
    use ironsmith::effects::{EffectContext, execute_effect};
    let stable = game.object(card).unwrap().stable_id;
    let exile = Effect::new(ironsmith::effects::ExileEffect::with_spec(
        ironsmith::target::ChooseSpec::SpecificObject(card))
        .with_face_down(face_down)
        .with_actor(ironsmith::target::PlayerFilter::Specific(actor)));
    let mut ctx = EffectContext::new_default(source, controller);
    execute_effect(game, &exile, &mut ctx).unwrap();
    drop(ctx);
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    exiled
}

fn p38_library_card(game: &mut ironsmith::GameState, owner: PlayerId, name: &str) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    game.create_object_from_card(&card, owner, Zone::Library)
}

#[test]
fn shared_fate_scopes_look_and_play_to_the_player_who_exiled_each_card() {
    let definition = compile_to_runtime_definition("Shared Fate", P38_SHARED_FATE, false)
        .expect("the opponent-library draw replacement and per-exiler permission should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentOf(IteratedPlayer)"), "their opponents are the drawer's: {debug}");
    assert!(debug.contains("actor: IteratedPlayer"), "the drawer performs the exile: {debug}");
    assert!(debug.contains(ironsmith::tag::SOURCE_EXILED_BY_YOU_TAG), "per-exiler pool: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let fate = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alices_card = p38_library_card(&mut game, alice, "Fate card of Alice");
    let bobs_card = p38_library_card(&mut game, bob, "Fate card of Bob");
    // Bob exiled Alice's top card; Alice exiled Bob's.
    let by_bob = p38_exile_by(&mut game, fate, alice, alices_card, bob, true);
    let by_alice = p38_exile_by(&mut game, fate, alice, bobs_card, alice, true);
    assert_eq!(game.source_exiler(by_bob), Some(bob));
    assert_eq!(game.source_exiler(by_alice), Some(alice));
    game.refresh_continuous_state().unwrap();

    let grants = |game: &ironsmith::GameState, card, player| !game.effect_store.grant_registry
        .get_grants_for_card(game, card, Zone::Exile, player).is_empty();
    assert!(grants(&game, by_bob, bob), "Bob may play the card he exiled");
    assert!(!grants(&game, by_bob, alice), "Alice may not play Bob's card, though she owns it");
    assert!(grants(&game, by_alice, alice));
    assert!(!grants(&game, by_alice, bob));
    assert!(game.can_player_look_at_face_down_exiled_card(by_bob, bob), "Bob may look at his card");
    assert!(!game.can_player_look_at_face_down_exiled_card(by_bob, alice));
    assert!(game.can_player_look_at_face_down_exiled_card(by_alice, alice));
    assert!(!game.can_player_look_at_face_down_exiled_card(by_alice, bob));
}

#[test]
fn uba_mask_permits_only_this_turns_cards_each_player_exiled() {
    let definition = compile_to_runtime_definition("Uba Mask", P38_UBA_MASK, false)
        .expect("the drawn-card exile replacement and per-exiler this-turn permission should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ExileTopOfLibraryEffect"), "the would-be-drawn card is the library top: {debug}");
    assert!(debug.contains("LibraryOwnerAsActor"), "the drawer exiles it: {debug}");
    assert!(debug.contains(ironsmith::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG), "this turn's pool: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mask = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = p38_library_card(&mut game, bob, "Mask card of Bob");
    let exiled = p38_exile_by(&mut game, mask, alice, card, bob, false);
    game.refresh_continuous_state().unwrap();
    let grants = |game: &ironsmith::GameState, player| !game.effect_store.grant_registry
        .get_grants_for_card(game, exiled, Zone::Exile, player).is_empty();
    assert!(grants(&game, bob), "Bob may play the card he exiled this turn");
    assert!(!grants(&game, alice), "the controller gets no permission for Bob's card");
    game.turn.turn_number += 1;
    assert!(!grants(&game, bob), "the permission covers only cards exiled this turn");
}
