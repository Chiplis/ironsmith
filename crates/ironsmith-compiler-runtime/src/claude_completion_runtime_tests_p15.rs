//! Claude package 15 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p15_definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn p15_triggered_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => {
                Some(trigger.effects.all_effects_owned())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

const OMNATH_TEXT: &str = "Mana cost: {W}{U}{B/P}{R}{G}\nType: Legendary Creature — Phyrexian Elemental\nIf you would lose unspent mana, that mana becomes black instead.\nAt the beginning of your first main phase, look at the top card of your library. You may reveal that card if it has three or more colored mana symbols in its mana cost. If you do, add three mana in any combination of its colors and put it into your hand. If you don't reveal it, put it into your hand.";

#[test]
fn omnath_reveal_gate_counts_colored_pips_and_adds_mana_of_the_cards_colors() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::game_state::GameState;
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::{PlayerId, Zone};
    let definition = p15_definition("Omnath, Locus of All", OMNATH_TEXT);
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("colored_mana_symbol_count:Some("), "{debug}");
    assert!(compact.contains("available_colors_among:Some("), "{debug}");
    // (cost, expected mana added)
    let cases = [
        (ManaCost::from_pips(vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue, ManaSymbol::Black], vec![ManaSymbol::Black, ManaSymbol::Life(2)]]), 3),
        (ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Red, ManaSymbol::Red]), 0),
    ];
    for (cost, expected_mana) in cases {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Top card").mana_cost(cost).build();
        let top = game.create_object_from_card(&card, alice, Zone::Library);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, alice);
        for effect in p15_triggered_effects(&definition) {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }
        let player = game.player(alice).unwrap();
        let pool = &player.mana_pool;
        let total = pool.white + pool.blue + pool.black + pool.red + pool.green + pool.colorless;
        assert_eq!(total as i32, expected_mana, "{debug}");
        assert_eq!(pool.red + pool.green + pool.colorless, 0, "only the card's colors");
        assert_eq!(player.hand.len(), 1, "the card goes to hand either way");
        assert!(game.object(top).is_none_or(|object| object.zone != Zone::Library));
    }
}

#[test]
fn spellweaver_volute_copies_and_casts_the_enchanted_graveyard_card() {
    let definition = p15_definition("Spellweaver Volute", "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant instant card in a graveyard\nWhenever you cast a sorcery spell, copy the enchanted instant card. You may cast the copy without paying its mana cost. If you do, exile the enchanted card and attach this Aura to another instant card in a graveyard.");
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    fn visit(effect: &Effect, tags: &mut Vec<ironsmith::tag::TagKey>, casts: &mut Vec<ironsmith::effects::CastTaggedEffect>) {
        if let Some(attached) = effect.downcast_ref::<ironsmith::effects::TagAttachedToSourceEffect>() {
            tags.push(attached.tag.clone());
        }
        if let Some(cast) = effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>() { casts.push(cast.clone()); }
        effect.visit_child_effects(&mut |child| visit(child, tags, casts));
    }
    let (mut tags, mut casts) = (Vec::new(), Vec::new());
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { visit(effect, &mut tags, &mut casts); }
        }
    }
    assert!(casts.iter().any(|cast| tags.contains(&cast.tag) && cast.as_copy && cast.without_paying_mana_cost),
        "the cast uses the captured attached card, regardless of generated tag spelling: {debug}");
    assert!(!compact.contains("CopySpellEffect"), "the card is not a stack spell: {debug}");
    assert!(compact.contains("IfEffect") && compact.contains("ExileEffect") && compact.contains("AttachObjectsEffect"), "{debug}");
}

#[test]
fn vesuvan_shapeshifter_copy_lasts_until_it_is_turned_face_down() {
    let definition = p15_definition("Vesuvan Shapeshifter", "Mana cost: {3}{U}{U}\nType: Creature — Shapeshifter\nAs this creature enters or is turned face up, you may choose another creature on the battlefield. If you do, until this creature is turned face down, it becomes a copy of that creature, except it has \"At the beginning of your upkeep, you may turn this creature face down.\"\nMorph {1}{U}");
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("also_turns_face_up:true"), "{debug}");
    assert!(compact.contains("ObjectFaceUp(Source)"), "copy must end when turned face down: {debug}");
    assert!(compact.contains("CopyOfWithAbilities") && compact.contains("TurnFaceDownEffect"), "{debug}");
    let apply = compact
        .split_once("ApplyContinuousEffect")
        .map(|(_, tail)| tail.split_once("runtime_modifications").map_or(tail, |(head, _)| head))
        .expect("becomes-copy continuous effect");
    assert!(!apply.contains("__it__"), "the copier is this creature, not the chosen creature: {debug}");
    let choose = compact.split_once("ChooseObjectsEffect").map(|(_, tail)| tail).expect("choice");
    let choose = choose.split_once("count:").map_or(choose, |(head, _)| head);
    assert!(choose.contains("other:true"), "'another creature' excludes this creature: {debug}");
}

#[test]
fn search_the_city_matches_plays_by_linked_exile_name() {
    let definition = p15_definition("Search the City", "Mana cost: {4}{U}\nType: Enchantment\nWhen this enchantment enters, exile the top five cards of your library.\nWhenever you play a card with the same name as one of the exiled cards, you may put one of those cards with that name into its owner's hand. Then if there are no cards exiled with this enchantment, sacrifice it. If you do, take an extra turn after this one.");
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    let trigger = compact.split_once("AnyOfTrigger").map(|(_, tail)| tail).expect("cast-or-land play trigger");
    let trigger = trigger.split_once("effects:").map_or(trigger, |(head, _)| head);
    assert!(trigger.contains("__source_exiled__") && trigger.contains("SameNameAsTagged"), "{debug}");
    assert!(!trigger.contains("zone:Some(Exile)"), "the played card is not in exile: {debug}");
    assert!(compact.contains("ChooseObjectsEffect") && compact.contains("relation:SameNameAsTagged"), "{debug}");
    assert!(compact.contains("MoveToZoneEffect") && compact.contains("zone:Hand"), "{debug}");
    assert!(compact.contains("SacrificeEffect") || compact.contains("Sacrifice"), "{debug}");
    assert!(compact.contains("ExtraTurn"), "{debug}");
}

#[test]
fn spellweaver_helix_copies_the_other_linked_card_on_a_matching_card_cast() {
    let definition = p15_definition("Spellweaver Helix", "Mana cost: {3}\nType: Artifact\nImprint — When this artifact enters, you may exile two target sorcery cards from a single graveyard.\nWhenever a player casts a card, if it has the same name as one of the cards exiled with this artifact, you may copy the other. If you do, you may cast the copy without paying its mana cost.");
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("SpellCastTrigger") && compact.contains("physical_card:true"), "casting a copy must not trigger: {debug}");
    assert!(compact.contains("relation:DifferentNameFromTagged"), "'the other' excludes the matching card: {debug}");
    let cast = compact.split_once("CastTaggedEffect").map(|(_, tail)| tail).expect("copy cast");
    assert!(cast.contains("as_copy:true") && cast.contains("without_paying_mana_cost:true"), "{debug}");
    assert!(!compact.contains("CopySpellEffect"), "{debug}");
}

#[test]
fn sanar_exiles_at_most_one_revealed_card_per_color_among_your_permanents() {
    let definition = p15_definition("Sanar, Innovative First-Year", "Mana cost: {2}{U/R}{U/R}\nType: Legendary Creature — Goblin Sorcerer\nVivid — At the beginning of your first main phase, reveal cards from the top of your library until you reveal X nonland cards, where X is the number of colors among permanents you control. For each of those colors, you may exile a card of that color from among the revealed cards. Then shuffle. You may cast the exiled cards this turn.");
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("ConsultTopOfLibraryEffect") && compact.contains("ColorsAmong("), "{debug}");
    // One optional choice per color, gated on controlling a permanent of it.
    assert_eq!(compact.matches("ChooseObjectsEffect").count(), 5, "{debug}");
    assert!(compact.contains("relation:IsNotTaggedObject"), "a card fills one color slot: {debug}");
    assert!(compact.contains("ConditionalEffect"), "{debug}");
    assert!(compact.contains("ShuffleLibraryEffect") && compact.contains("GrantPlayTaggedEffect"), "{debug}");
}
