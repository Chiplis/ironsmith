//! Additional coverage probes; diagnostic only, no production changes.
use ironsmith::{
    ability::AbilityKind,
    cards::builders::CardDefinitionBuilder,
    game_state::GameState,
    ids::{CardId, PlayerId},
    types::CardType,
    zone::Zone,
};
use ironsmith_core::StaticAbilityPayload as P;
use serde_json::json;

fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
fn run() {
    let payloads =
        ironsmith_tools::load_card_payloads_by_names("cards.json", &["Augur of Autumn".into()])
            .unwrap();
    let def = ironsmith_tools::compile_runtime_definition_from_payload(
        payloads.values().next().unwrap().first().unwrap(),
    )
    .unwrap();
    let condition = def
        .abilities
        .iter()
        .find_map(|a| {
            let AbilityKind::Static(s) = &a.kind else {
                return None;
            };
            match &s.compiled_model()?.payload {
                P::Conditional { condition, .. } => Some(condition.clone()),
                _ => None,
            }
        })
        .unwrap();
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let player = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, player, Zone::Battlefield);
    for i in 0..2 {
        let witness = CardDefinitionBuilder::new(CardId::new(), format!("Witness {i}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3))
            .build();
        game.create_object_from_definition(&witness, player, Zone::Battlefield);
    }
    game.refresh_continuous_state().unwrap();
    let actual = ironsmith::condition_eval::evaluate_condition_cast_time_checked(
        &game, &condition, player, source,
    )
    .unwrap();
    println!(
        "{}",
        json!({"probe":"same distinct-powers predicate through cast-time evaluator", "expected":false,"actual":actual,"condition":format!("{condition:?}")})
    );
    let mut outputs = vec![];
    for life in [5, 15] {
        let grant = ironsmith_core::GrantSpec::new(
            ironsmith_core::Grantable::graveyard_cast_from_cards_mana_cost_with_condition(
                ironsmith_core::ThisSpellCostCondition::YouLifeTotalOrLess(life),
                false,
            ),
            ironsmith::target::ObjectFilter::source(),
            Zone::Graveyard,
        );
        let mut changed = def.clone();
        changed.abilities.clear();
        changed.ability_labels.clear();
        let mut model = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Static(s) => s.compiled_model().cloned(),
                _ => None,
            })
            .unwrap();
        model.payload = P::Grants(Box::new(grant));
        changed
            .abilities
            .push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::from_model(model),
            ));
        outputs.push(ironsmith::compiled_text::semantic_audit_lines(&changed));
    }
    println!(
        "{}",
        json!({"probe":"graveyard grant life threshold 5 to 15", "audit_hidden":outputs[0]==outputs[1], "before":outputs[0],"after":outputs[1]})
    );
}
