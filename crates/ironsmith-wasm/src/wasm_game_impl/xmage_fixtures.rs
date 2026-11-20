//! Native definitions for synthetic Java test cards with no supported Oracle syntax.
//! The XMage test build exposes setup only. Rules decisions use the ordinary runtime.

use super::WasmGame;
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::continuous::Modification;
use ironsmith::effect::{Effect, Until, Value, ValueComparisonOperator};
use ironsmith::effects::continuous::ApplyContinuousEffect;
use ironsmith::events::counters::matchers::WouldRemoveCountersMatcher;
use ironsmith::grant::{GrantSpec, Grantable};
use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use ironsmith::static_abilities::{StaticAbilityId, StaticAbilityKind};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::types::CardType;
use ironsmith::{
    Ability, CardId, ConditionExpr, ManaCost, ManaSymbol, ObjectId, PlayerId, StaticAbility,
    TotalCost, Zone,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FixtureAbility {
    LibraryPlayWhileCreatureExists,
    TargetOpponentsGraveyardPlayWhileCreatureExists,
    RemoveOneFewerCounter,
    RetainTwoCounters,
}

#[derive(Debug, Clone)]
struct ModifyOwnCounterRemoval {
    modification: EventModification,
    description: &'static str,
}

impl StaticAbilityKind for ModifyOwnCounterRemoval {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EventAmountReplacement
    }

    fn display(&self) -> String {
        self.description.into()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldRemoveCountersMatcher::new(ObjectFilter::source(), None),
            ReplacementAction::Modify(self.modification.clone()),
        ))
    }
}

fn play_while_creature_exists(zone: Zone, filter: ObjectFilter) -> StaticAbility {
    let mut grant = GrantSpec::new(Grantable::play_from(), filter, zone);
    grant.beneficiary = PlayerFilter::Any;
    let condition = ConditionExpr::ValueComparison {
        left: Value::Count(ObjectFilter::creature()),
        operator: ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    };
    StaticAbility::grants(grant)
        .with_condition(condition)
        .expect("a static play permission supports a condition")
}

impl FixtureAbility {
    fn build(self) -> Ability {
        match self {
            Self::LibraryPlayWhileCreatureExists => Ability::static_ability(
                play_while_creature_exists(Zone::Library, ObjectFilter::default()),
            ),
            Self::TargetOpponentsGraveyardPlayWhileCreatureExists => {
                let target = ChooseSpec::target(ChooseSpec::Object(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::Opponent),
                ));
                // The permission rides on the targeted graveyard card, so the
                // granted static must function in the graveyard (CR 113.6);
                // a plain `AddAbility` static functions only on the battlefield.
                let effect = ApplyContinuousEffect::with_spec(
                    target,
                    Modification::AddAbilityGeneric(
                        Ability::static_ability(play_while_creature_exists(
                            Zone::Graveyard,
                            ObjectFilter::source(),
                        ))
                        .in_zones(vec![Zone::Graveyard]),
                    ),
                    Until::EndOfTurn,
                );
                let cost = TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]));
                Ability::activated(cost, vec![Effect::new(effect)])
            }
            Self::RemoveOneFewerCounter => {
                Ability::static_ability(StaticAbility::new(ModifyOwnCounterRemoval {
                    modification: EventModification::Subtract(1),
                    description: "If counters would be removed from this creature, remove one fewer instead.",
                }))
            }
            Self::RetainTwoCounters => {
                Ability::static_ability(StaticAbility::new(ModifyOwnCounterRemoval {
                    modification: EventModification::PreserveObjectCounterMinimum(2),
                    description: "A counter removal can't reduce the number of those counters on this creature below two.",
                }))
            }
        }
    }
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = stageXmageFixture)]
    pub fn stage_xmage_fixture(
        &mut self,
        player_index: u8,
        name: &str,
        ability_js: JsValue,
    ) -> Result<u64, JsValue> {
        if self.runner.is_some()
            || self.pregame.is_some()
            || !self.loaded_decks.is_empty()
            || self.pending_decision.is_some()
        {
            return Err(JsValue::from_str(
                "XMage fixture staging requires a resetEmpty puzzle before play",
            ));
        }
        let ability: FixtureAbility =
            serde_wasm_bindgen::from_value(ability_js).map_err(|error| {
                JsValue::from_str(&format!("invalid XMage fixture ability: {error}"))
            })?;
        let player = PlayerId::from_index(player_index);
        if name.trim().is_empty() || self.game.player(player).is_none() {
            return Err(JsValue::from_str(
                "XMage fixture requires a name and a valid player",
            ));
        }
        let mut definition =
            CardDefinitionBuilder::new(CardId::new(), name).with_ability(ability.build());
        if matches!(
            ability,
            FixtureAbility::RemoveOneFewerCounter | FixtureAbility::RetainTwoCounters
        ) {
            definition = definition
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1));
        }
        let definition = definition.build();
        let object =
            self.game
                .create_object_from_definition(&definition, player, Zone::Battlefield);
        Ok(object.0)
    }
}
