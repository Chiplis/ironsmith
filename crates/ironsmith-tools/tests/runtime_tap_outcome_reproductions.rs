//! Canonical fixed multi-permanent tap costs with exact output and resource boundaries.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Choices {
    targets: Vec<Target>,
    names: Vec<String>,
    x: u32,
    allow_optional: bool,
    choose_untap: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let s = self.x.max(c.min).min(c.max);
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"minimum":c.min,"maximum":c.max,"is_x_value":c.is_x_value,"requested":self.x,"selected":s}));
        s
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.trace.push(
            json!({"choice":"boolean","context":format!("{c:?}"),"selected":self.allow_optional}),
        );
        self.allow_optional
    }
    fn decide_colors(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        let s = vec![ironsmith::color::Color::Blue; c.count as usize];
        self.trace.push(
            json!({"choice":"colors","context":format!("{c:?}"),"selected":format!("{s:?}")}),
        );
        s
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if let Some(o) = c.options.iter().find(|o| {
            o.legal
                && if self.choose_untap {
                    o.description.to_lowercase().contains("untap")
                } else {
                    o.description.to_lowercase().starts_with("tap ")
                }
        }) {
            vec![o.index]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_partition(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        let selected = c.cards.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        self.trace.push(json!({"choice":"partition_to_graveyard","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut s = vec![];
        for name in &self.names {
            for o in &c.candidates {
                if o.legal && g.object(o.id).is_some_and(|o| o.name == *name) && !s.contains(&o.id)
                {
                    s.push(o.id)
                }
            }
        }
        s.truncate(c.max.unwrap_or(s.len()));
        if s.len() < c.min {
            for o in &c.candidates {
                if o.legal && !s.contains(&o.id) {
                    s.push(o.id);
                    if s.len() >= c.min {
                        break;
                    }
                }
            }
        }
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":s.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string())})).collect::<Vec<_>>()}));
        s
    }
}
fn setup(players: usize, lands: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 30);
    }
    for _ in 0..lands {
        let def = CardDefinitionBuilder::new(CardId::new(), "Trigger sacrifice land")
            .card_types(vec![CardType::Land])
            .build();
        g.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
    }
    g
}
fn announce(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<(TriggerQueue, Value), String> {
    g.turn.priority_player = Some(PlayerId(actor));
    let source = g.create_object_from_definition(def, PlayerId(actor), Zone::Hand);
    let action = compute_legal_actions(g, PlayerId(actor)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .ok_or("intended cast unavailable")?;
    let mana = g.player(PlayerId(actor)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("announcement returned priority without spell".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    if g.stack.is_empty() {
        return Err("announcement budget".into());
    }
    let entry = g
        .stack
        .iter()
        .find(|entry| {
            g.object(entry.object_id)
                .is_some_and(|o| o.name == def.name())
        })
        .ok_or("intended spell missing from stack")?;
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",entry.targets),"resolution_error":null,"stack_names_at_announcement":g.stack.iter().filter_map(|e|g.object(e.object_id).map(|o|o.name.to_string())).collect::<Vec<_>>()});
    Ok((q, evidence))
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<Value, String> {
    let (mut q, mut evidence) = announce(g, def, actor, dm)?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        if let Err(error) = advance_priority_with_dm(g, &mut q, dm) {
            evidence["resolution_error"] = json!(error.to_string());
            return Ok(evidence);
        }
        if g.stack.is_empty() {
            return Ok(evidence);
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            if let Err(error) = apply_priority_response_with_dm(
                g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            ) {
                evidence["resolution_error"] = json!(error.to_string());
                return Ok(evidence);
            }
        }
    }
    Err("resolution decision budget".into())
}

fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(
                g,
                q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("finish budget".into())
}
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == name && o.zone == zone)
        .map(|o| o.id)
        .ok_or(format!("{name} absent in {zone:?}"))
}
fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    dm: &mut Choices,
    mana: u32,
) -> Result<Value, String> {
    let e = cast(g, &defs[name], 0, dm)?;
    if e["mana_paid"] != mana || !e["resolution_error"].is_null() {
        return Err(format!("{name} cast/payment mismatch:{e}"));
    }
    Ok(e)
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn action(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    mana: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::ActivateAbility {
                source: s,
                ability_index,
            } => !mana && *s == source && *ability_index == index,
            LegalAction::ActivateManaAbility {
                source: s,
                ability_index,
            } => mana && *s == source && *ability_index == index,
            _ => false,
        })
        .ok_or("intended activation unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(a.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        match progress {
            GameProgress::NeedsDecisionCtx(ref ctx)
                if !matches!(ctx, DecisionContext::Priority(_)) =>
            {
                progress = apply_decision_context_with_dm(g, &mut q, &mut st, ctx, dm)
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                let err = finish(g, &mut q, dm).err();
                return Ok(
                    json!({"action":format!("{a:?}"),"mana_change":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before as i64,"resolution_error":err}),
                );
            }
        }
    }
    Err("activation decision budget".into())
}

const NAMES: [&str; 21] = [
    "Altar Golem",
    "Baylen, the Haymaker",
    "Crookclaw Elder",
    "Goldfury Strider",
    "Honeymoon Hearse",
    "Inalla, Archmage Ritualist",
    "Kithkeeper",
    "Kumena, Tyrant of Orazca",
    "Persistent Petitioners",
    "Prosperous Partnership",
    "Root-Kin Ally",
    "Sanctuary Lockdown",
    "Sandsower",
    "Shacklegeist",
    "Skaab Wrangler",
    "Spurred Wolverine",
    "Supportive Parents",
    "Symbiotic Deployment",
    "Tangle Tumbler",
    "The Archimandrite",
    "Voyager Glidecar",
];
struct Spec {
    name: &'static str,
    index: usize,
    required: i32,
    price: u32,
    producer: &'static str,
    producer_price: u32,
    include_source: bool,
    effect: &'static str,
    cost: i64,
}
fn specs() -> Vec<Spec> {
    vec![
        Spec {
            name: "Altar Golem",
            index: 3,
            required: 5,
            price: 7,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: false,
            effect: "untap",
            cost: 0,
        },
        Spec {
            name: "Baylen, the Haymaker",
            index: 0,
            required: 2,
            price: 3,
            producer: "Sprout",
            producer_price: 1,
            include_source: false,
            effect: "mana",
            cost: 0,
        },
        Spec {
            name: "Baylen, the Haymaker",
            index: 1,
            required: 3,
            price: 3,
            producer: "Sprout",
            producer_price: 1,
            include_source: false,
            effect: "draw",
            cost: 0,
        },
        Spec {
            name: "Baylen, the Haymaker",
            index: 2,
            required: 4,
            price: 3,
            producer: "Sprout",
            producer_price: 1,
            include_source: false,
            effect: "counters_trample",
            cost: 0,
        },
        Spec {
            name: "Crookclaw Elder",
            index: 1,
            required: 2,
            price: 6,
            producer: "Storm Crow",
            producer_price: 2,
            include_source: true,
            effect: "draw",
            cost: 0,
        },
        Spec {
            name: "Crookclaw Elder",
            index: 2,
            required: 2,
            price: 6,
            producer: "Fugitive Wizard",
            producer_price: 1,
            include_source: true,
            effect: "flying",
            cost: 0,
        },
        Spec {
            name: "Goldfury Strider",
            index: 1,
            required: 2,
            price: 5,
            producer: "Bonesplitter",
            producer_price: 1,
            include_source: true,
            effect: "power",
            cost: 0,
        },
        Spec {
            name: "Honeymoon Hearse",
            index: 1,
            required: 2,
            price: 3,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: false,
            effect: "vehicle",
            cost: 0,
        },
        Spec {
            name: "Inalla, Archmage Ritualist",
            index: 1,
            required: 5,
            price: 5,
            producer: "Fugitive Wizard",
            producer_price: 1,
            include_source: true,
            effect: "drain",
            cost: 0,
        },
        Spec {
            name: "Kithkeeper",
            index: 1,
            required: 3,
            price: 7,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: true,
            effect: "kith_pump",
            cost: 0,
        },
        Spec {
            name: "Kumena, Tyrant of Orazca",
            index: 1,
            required: 3,
            price: 3,
            producer: "Merfolk of the Pearl Trident",
            producer_price: 1,
            include_source: true,
            effect: "draw",
            cost: 0,
        },
        Spec {
            name: "Kumena, Tyrant of Orazca",
            index: 2,
            required: 5,
            price: 3,
            producer: "Merfolk of the Pearl Trident",
            producer_price: 1,
            include_source: true,
            effect: "merfolk_counters",
            cost: 0,
        },
        Spec {
            name: "Persistent Petitioners",
            index: 1,
            required: 4,
            price: 2,
            producer: "Persistent Petitioners",
            producer_price: 2,
            include_source: true,
            effect: "mill",
            cost: 0,
        },
        Spec {
            name: "Prosperous Partnership",
            index: 1,
            required: 3,
            price: 3,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: false,
            effect: "treasure",
            cost: 0,
        },
        Spec {
            name: "Root-Kin Ally",
            index: 1,
            required: 2,
            price: 6,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: true,
            effect: "pump",
            cost: 0,
        },
        Spec {
            name: "Sanctuary Lockdown",
            index: 1,
            required: 2,
            price: 3,
            producer: "Fugitive Wizard",
            producer_price: 1,
            include_source: false,
            effect: "tap",
            cost: 2,
        },
        Spec {
            name: "Sandsower",
            index: 0,
            required: 3,
            price: 4,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: true,
            effect: "tap",
            cost: 0,
        },
        Spec {
            name: "Shacklegeist",
            index: 2,
            required: 2,
            price: 2,
            producer: "Lantern Kami",
            producer_price: 1,
            include_source: true,
            effect: "tap",
            cost: 0,
        },
        Spec {
            name: "Skaab Wrangler",
            index: 0,
            required: 3,
            price: 2,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: true,
            effect: "tap",
            cost: 0,
        },
        Spec {
            name: "Spurred Wolverine",
            index: 0,
            required: 2,
            price: 5,
            producer: "Leatherback Baloth",
            producer_price: 3,
            include_source: true,
            effect: "first_strike",
            cost: 0,
        },
        Spec {
            name: "Supportive Parents",
            index: 0,
            required: 2,
            price: 3,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: true,
            effect: "mana",
            cost: 0,
        },
        Spec {
            name: "Symbiotic Deployment",
            index: 1,
            required: 2,
            price: 3,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: false,
            effect: "draw",
            cost: 1,
        },
        Spec {
            name: "Tangle Tumbler",
            index: 2,
            required: 2,
            price: 3,
            producer: "Sprout",
            producer_price: 1,
            include_source: false,
            effect: "vehicle",
            cost: 0,
        },
        Spec {
            name: "The Archimandrite",
            index: 2,
            required: 3,
            price: 5,
            producer: "Sage of Lat-Nam",
            producer_price: 2,
            include_source: true,
            effect: "draw",
            cost: 0,
        },
        Spec {
            name: "Voyager Glidecar",
            index: 1,
            required: 3,
            price: 1,
            producer: "Grizzly Bears",
            producer_price: 2,
            include_source: false,
            effect: "vehicle_flying_counter",
            cost: 0,
        },
    ]
}
fn fund(g: &mut GameState, actor: PlayerId) {
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(actor).unwrap().mana_pool.add(c, 30);
    }
}
fn next_main(g: &mut GameState) {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(g.turn.active_player);
    fund(g, g.turn.active_player);
}

fn characteristics(g: &GameState, id: ObjectId) -> Value {
    let c = g.calculated_characteristics(id).unwrap();
    let creature = c.card_types.contains(&CardType::Creature);
    json!({"creature":creature,"power":if creature{c.power}else{None},"toughness":if creature{c.toughness}else{None},"flying":g.object_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Flying),"first_strike":g.object_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::FirstStrike),"trample":g.object_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Trample),"counters":g.counter_count(id,ironsmith::CounterType::PlusOnePlusOne)})
}
fn add_stat(v: &mut Value, k: &str, n: i64) {
    v[k] = json!(v[k].as_i64().unwrap() + n);
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    s: &Spec,
    extra: i32,
    state: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: false,
        choose_untap: false,
        trace: vec![],
    };
    let mut producers = vec![];
    for _ in 0..24 {
        for p in [PlayerId(0), PlayerId(1)] {
            g.create_object_from_definition(&defs["Plains"], p, Zone::Library);
        }
    }
    next_main(&mut g);
    let target_cast = cast(&mut g, &defs["Hill Giant"], 1, &mut dm)?;
    if target_cast["mana_paid"] != 4 || !target_cast["resolution_error"].is_null() {
        return Err("opponent target cast failed".into());
    }
    let target = find(&g, "Hill Giant", Zone::Battlefield)?;
    next_main(&mut g);
    let source_cast = paid(&mut g, defs, s.name, &mut dm, s.price)?;
    let source = find(&g, s.name, Zone::Battlefield)?;
    let mut resources = if s.include_source {
        vec![source]
    } else {
        vec![]
    };
    let etb_tokens = g
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Battlefield && o.kind == ironsmith::object::ObjectKind::Token)
        .map(|o| o.id)
        .collect::<Vec<_>>();
    if s.name == "Kithkeeper" && etb_tokens.len() != 1 {
        return Err(format!(
            "actual Kithkeeper token count {}",
            etb_tokens.len()
        ));
    }
    if s.name == "Prosperous Partnership" && etb_tokens.len() != 2 {
        return Err(format!(
            "actual Partnership token count {}",
            etb_tokens.len()
        ));
    }
    resources.extend(etb_tokens);
    while resources.len() < (s.required + extra) as usize {
        let before = g.battlefield.clone();
        dm.targets.clear();
        producers.push(paid(&mut g, defs, s.producer, &mut dm, s.producer_price)?);
        let created = g
            .battlefield
            .iter()
            .filter(|id| !before.contains(id) && g.controller_of_id(**id) == Some(PlayerId(0)))
            .copied()
            .collect::<Vec<_>>();
        if created.len() != 1 {
            return Err(format!("producer new objects {}", created.len()));
        }
        resources.push(created[0]);
    }
    if resources.len() != (s.required + extra) as usize {
        return Err("uncontrolled base resources".into());
    }
    if extra < 0
        && ![
            "Goldfury Strider",
            "Altar Golem",
            "Honeymoon Hearse",
            "Kithkeeper",
            "Root-Kin Ally",
            "Sandsower",
            "Skaab Wrangler",
            "Supportive Parents",
            "Symbiotic Deployment",
            "Voyager Glidecar",
            "Prosperous Partnership",
        ]
        .contains(&s.name)
    {
        producers.push(paid(&mut g, defs, "Ornithopter", &mut dm, 0)?);
    }
    if s.name == "Altar Golem" {
        dm.allow_optional = true;
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("Golem not tapped".into());
        }
        dm.allow_optional = false;
    }
    if state == "opponent_turn" {
        next_main(&mut g);
        g.turn.priority_player = Some(PlayerId(0));
        fund(&mut g, PlayerId(0));
    }
    if state == "upkeep" {
        g.turn.phase = ironsmith::Phase::Beginning;
        g.turn.step = Some(ironsmith::Step::Upkeep);
    }
    dm.targets = if ["tap", "flying", "first_strike", "power"].contains(&s.effect) {
        vec![Target::Object(target)]
    } else if ["mill", "drain"].contains(&s.effect) {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    dm.names = resources
        .iter()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect();
    let abilities = g
        .current_abilities(source)
        .ok_or("source abilities absent")?;
    let ability = match &abilities.get(s.index).ok_or("intended index missing")?.kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("intended index not activated".into()),
    };
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let is_mana = s.effect == "mana";
    let offered = actions.iter().any(|a| match a {
        LegalAction::ActivateAbility {
            source: id,
            ability_index,
        } => !is_mana && *id == source && *ability_index == s.index,
        LegalAction::ActivateManaAbility {
            source: id,
            ability_index,
        } => is_mana && *id == source && *ability_index == s.index,
        _ => false,
    });
    let baseline_source = characteristics(&g, source);
    let baseline_target = characteristics(&g, target);
    let before = json!({"source_index":s.index,"source_characteristics":baseline_source,"target_characteristics":baseline_target,"source_tapped":g.is_tapped(source),"source_sick":g.is_summoning_sick(source),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"characteristics":characteristics(&g,*id),"sick":g.is_summoning_sick(*id),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"actions":format!("{actions:?}"),"cost":format!("{:?}",ability.mana_cost)});
    let valid = extra >= 0 && state == "ready";
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.clone();
    let tokens_before = g
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Battlefield && o.kind == ironsmith::object::ObjectKind::Token)
        .count();
    let activation = action(&mut g, source, s.index, is_mana, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let mut expected_source = baseline_source.clone();
    let mut expected_target = baseline_target.clone();
    let mut cleanup_source = baseline_source.clone();
    let cleanup_target = baseline_target.clone();
    if s.effect == "counters_trample" {
        add_stat(&mut expected_source, "power", 3);
        add_stat(&mut expected_source, "toughness", 3);
        expected_source["counters"] = json!(3);
        expected_source["trample"] = json!(true);
        cleanup_source = expected_source.clone();
        cleanup_source["trample"] = baseline_source["trample"].clone();
    }
    if s.effect == "merfolk_counters" {
        add_stat(&mut expected_source, "power", 1);
        add_stat(&mut expected_source, "toughness", 1);
        expected_source["counters"] = json!(1);
        cleanup_source = expected_source.clone();
    }
    if s.effect == "pump" {
        add_stat(&mut expected_source, "power", 2);
        add_stat(&mut expected_source, "toughness", 2);
    }
    if s.effect == "kith_pump" {
        add_stat(&mut expected_source, "power", 3);
        expected_source["flying"] = json!(true);
    }
    if s.effect == "flying" {
        expected_target["flying"] = json!(true);
    }
    if s.effect == "first_strike" {
        expected_target["first_strike"] = json!(true);
    }
    if s.effect == "power" {
        add_stat(&mut expected_target, "power", 2);
    }
    if s.effect.starts_with("vehicle") {
        expected_source["creature"] = json!(true);
        let (p, t) = match s.name {
            "Honeymoon Hearse" => (5, 5),
            "Tangle Tumbler" => (6, 6),
            _ => (3, 4),
        };
        expected_source["power"] = json!(p);
        expected_source["toughness"] = json!(t);
        if s.effect == "vehicle_flying_counter" {
            expected_source["flying"] = json!(true);
            expected_source["counters"] = json!(1);
            cleanup_source["counters"] = json!(1);
        }
    }
    let pool = &g.player(PlayerId(0)).unwrap().mana_pool;
    let new_tokens = g
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Battlefield && o.kind == ironsmith::object::ObjectKind::Token)
        .count()
        - tokens_before;
    let treasure=g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.kind==ironsmith::object::ObjectKind::Token&&o.subtypes.iter().any(|t|format!("{t:?}")=="Treasure")).map(|o|json!({"artifact":o.card_types.contains(&CardType::Artifact),"controller":g.controller_of_id(o.id).map(|p|p.index()),"tapped":g.is_tapped(o.id),"mana_abilities":g.current_abilities(o.id).unwrap().iter().filter(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Activated(a)if a.is_mana_ability())).count()})).collect::<Vec<_>>();
    let mut actual = json!({"error":activation["resolution_error"],"mana_delta":pool.total() as i64-mana_before.total() as i64,"blue_delta":if is_mana{Some(pool.blue as i64-mana_before.blue as i64)}else{None},"source":characteristics(&g,source),"target":characteristics(&g,target),"source_tapped":g.is_tapped(source),"target_tapped":g.is_tapped(target),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"resource_counters":if s.effect=="merfolk_counters"{Some(resources.iter().map(|id|g.counter_count(*id,ironsmith::CounterType::PlusOnePlusOne)).collect::<Vec<_>>())}else{None},"plains_hand":count(&g,"Plains",Zone::Hand),"bob_library":g.player(PlayerId(1)).unwrap().library.len(),"bob_graveyard":g.player(PlayerId(1)).unwrap().graveyard.len(),"bob_life":g.player(PlayerId(1)).unwrap().life,"new_tokens":new_tokens,"treasure":treasure,"stack_length":g.stack.len()});
    let mut expected = json!({"error":null,"mana_delta":if is_mana{1}else{-s.cost},"blue_delta":if is_mana{Some(1)}else{None},"source":expected_source,"target":expected_target,"source_tapped":s.include_source,"target_tapped":s.effect=="tap","tapped_resources":s.required,"untapped_resources":extra,"resource_counters":if s.effect=="merfolk_counters"{Some(vec![1;s.required as usize+extra as usize])}else{None},"plains_hand":if s.effect=="draw"{1}else{0},"bob_library":if s.effect=="mill"{12}else{24},"bob_graveyard":if s.effect=="mill"{12}else{0},"bob_life":if s.effect=="drain"{13}else{20},"new_tokens":if s.effect=="treasure"{1}else{0},"treasure":if s.effect=="treasure"{vec![json!({"artifact":true,"controller":0,"tapped":false,"mana_abilities":1})]}else{vec![]},"stack_length":0});
    g.turn.phase = ironsmith::Phase::Ending;
    g.turn.step = Some(ironsmith::Step::Cleanup);
    ironsmith::turn::execute_cleanup_step(&mut g);
    actual["cleanup_source"] = characteristics(&g, source);
    actual["cleanup_target"] = characteristics(&g, target);
    expected["cleanup_source"] = cleanup_source;
    expected["cleanup_target"] = cleanup_target;
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Actual paid canonical source/resources/target, exact advertised index, real token creation, resource boundaries and selected taps. Standard execute_cleanup_step verifies expiration and persistent counters; no cleanup discard is required."}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_tap_outcomes() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Bonesplitter",
                "Fugitive Wizard",
                "Grizzly Bears",
                "Hill Giant",
                "Lantern Kami",
                "Leatherback Baloth",
                "Merfolk of the Pearl Trident",
                "Ornithopter",
                "Persistent Petitioners",
                "Plains",
                "Sage of Lat-Nam",
                "Sprout",
                "Storm Crow",
                "Twiddle",
            ]
            .contains(&n)
        {
            continue;
        }
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(
            json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}),
        );
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for spec in specs() {
        let name = spec.name;
        let mut cases = vec![(-1, "ready"), (0, "ready"), (1, "ready")];
        if name == "Goldfury Strider" {
            cases.push((0, "opponent_turn"));
            cases.push((0, "upkeep"));
        }
        for (extra, state) in cases {
            let (status, expected, actual, evidence) = match run(&defs, &spec, extra, state) {
                Ok((e, a, f)) => (
                    if e == a {
                        "expected_result_observed"
                    } else if !a["error"].is_null() {
                        "execution_failed"
                    } else {
                        "semantic_mismatch"
                    },
                    e,
                    a,
                    f,
                ),
                Err(e) => (
                    "fixture_or_producer_error",
                    Value::Null,
                    json!({"error":e}),
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"state":state,"ability_index":spec.index,"effect":spec.effect},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Fixed multi-tap draw, pump, mana, creature-tap and vehicle paths. Actual paid sources, resources and targets, canonical token producers and selected exact indices. Required-minus-one/exact/surplus costs and standard cleanup expiration checks.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_tap_outcome_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/tap-outcome-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
