use super::*;

struct ChosenHandDecisions {
    accept: bool,
    opponent: crate::ids::PlayerId,
    chosen: Vec<crate::ids::ObjectId>,
    spell: crate::ids::ObjectId,
    choosers: Vec<crate::ids::PlayerId>,
    views: Vec<(crate::ids::PlayerId, Vec<crate::ids::ObjectId>)>,
}
impl crate::decision::DecisionMaker for ChosenHandDecisions {
    fn decide_boolean(&mut self, _: &crate::game_state::GameState, _: &crate::decisions::context::BooleanContext) -> bool { self.accept }
    fn decide_objects(&mut self, _: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
        self.choosers.push(ctx.player);
        let desired = if ctx.player == self.opponent { self.chosen.clone() } else { vec![self.spell] };
        desired.into_iter().filter(|id| ctx.candidates.iter().any(|candidate| candidate.legal && candidate.id == *id)).collect()
    }
    fn view_cards(&mut self, _: &crate::game_state::GameState, viewer: crate::ids::PlayerId, cards: &[crate::ids::ObjectId], _: &crate::decisions::context::ViewCardsContext) {
        self.views.push((viewer, cards.to_vec()));
    }
}

#[test]
fn chosen_hand_free_cast_uses_opponents_selection_and_your_cast_choice() {
    for accept in [false, true] {
        for x in [0, 1, 2, 5] { check_chosen_hand_cast(accept, x); }
    }
}

fn check_chosen_hand_cast(accept: bool, x: u32) {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chosen hand fixture")
        .card_types(vec![CardType::Sorcery]).parse_text("Target opponent chooses X cards from their hand. Look at those cards. You may cast a spell from among them without paying its mana cost.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id; let bob = game.players[1].id;
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Expensive spell")
        .card_types(vec![CardType::Instant]).mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(7)]))
        .parse_text("You gain 3 life.").unwrap();
    let selected = game.create_object_from_definition(&spell, bob, Zone::Hand);
    let unselected = game.create_object_from_definition(&spell, bob, Zone::Hand);
    let own = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected land").card_types(vec![CardType::Land]).build();
    let land = game.create_object_from_definition(&land, bob, Zone::Hand);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let selected_stable = game.object(selected).unwrap().stable_id;
    let chosen = match x { 0 => vec![], 1 => vec![land], 2 => vec![land, selected], _ => vec![land, selected, unselected] };
    let mut dm = ChosenHandDecisions { accept, opponent: bob, chosen: chosen.clone(), spell: selected, choosers: vec![], views: vec![] };
    let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_x(x)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]).with_decision_maker(&mut dm);
    for segment in &definition.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    drop(ctx);
    let casts = accept && x >= 2;
    assert_eq!(game.stack.len(), usize::from(casts), "accept={accept} x={x}");
    if casts {
        let entry = &game.stack[0];
        assert_eq!(entry.controller, alice);
        assert_eq!(game.object(entry.object_id).unwrap().stable_id, selected_stable);
        assert_eq!(dm.choosers, vec![bob, alice]);
    } else {
        assert_eq!(game.object(selected).unwrap().zone, Zone::Hand);
    }
    assert!(dm.views.iter().all(|(viewer, cards)| *viewer == alice && cards.iter().all(|id| chosen.contains(id))), "only the selected cards are exposed: {:?}", dm.views);
    if x == 0 {
        assert!(dm.views.is_empty());
    } else {
        assert_eq!(dm.views[0].1.len(), chosen.len());
        assert!(chosen.iter().all(|id| dm.views[0].1.contains(id)));
    }
    for id in [own, unselected, land] { assert_eq!(game.object(id).unwrap().zone, Zone::Hand); }
}

#[test]
fn chosen_hand_free_cast_render_requires_the_same_selected_pool() {
    let oracle = "Target opponent chooses X cards from their hand. Look at those cards. You may cast a spell from among them without paying its mana cost.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected pool rendering fixture")
        .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [oracle]);
    let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
    let render = super::super::structural_bundles::describe_chosen_hand_optional_free_cast;
    assert!(render(effects).is_some());
    for mutation in 0..5 {
        let mut broken = effects.clone();
        if mutation == 0 {
            let mut look = broken[2].downcast_ref::<crate::effects::LookAtObjectsEffect>().unwrap().clone();
            look.filter = ObjectFilter::tagged("unrelated_pool").in_zone(Zone::Hand);
            broken[2] = Effect::new(look);
        } else {
            let mut optional = broken[3].downcast_ref::<crate::effects::MayEffect>().unwrap().clone();
            if mutation == 1 {
                let mut choice = optional.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
                choice.filter.zone = Some(Zone::Exile);
                optional.effects[0] = Effect::new(choice);
            } else {
                let mut each = optional.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>().unwrap().clone();
                if mutation == 2 { each.tag = TagKey::from("unrelated_selection"); }
                else {
                    let mut cast = each.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>().unwrap().clone();
                    if mutation == 3 { cast.without_paying_mana_cost = false; }
                    else { cast.player = PlayerFilter::Opponent; }
                    each.effects[0] = Effect::new(cast);
                }
                optional.effects[1] = Effect::new(each);
            }
            broken[3] = Effect::new(optional);
        }
        assert!(render(&broken).is_none(), "mutation {mutation}");
    }
}
