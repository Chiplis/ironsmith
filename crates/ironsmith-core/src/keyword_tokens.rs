//! Token-creation instructions defined by keyword rules. Their characteristics
//! carry rules-implied text roles even when the quantity is an authored value.

use crate::{
    CardBuilder, CardDefinition, CardId, CardType, ChooseModeEffect, ColorSet, CostComponent,
    CreateTokenEffect, EffectMode, PowerToughness, Subtype, TokenNameTextRole, TokenTextRoles,
    Value,
};

fn creature(name: &str, subtypes: Vec<Subtype>, size: i32) -> CardBuilder {
    CardBuilder::new(CardId::new(), name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(size, size))
}

impl<A, E, C: CostComponent, AC, OC> CreateTokenEffect<CardDefinition<A, E, C, AC, OC>> {
    fn keyword_token(face: CardBuilder, abilities: Vec<A>, count: impl Into<Value>) -> Self {
        let roles =
            TokenTextRoles::rules_implied(TokenNameTextRole::SubtypeDerived, abilities.len());
        Self::you(
            CardDefinition::with_abilities(face.build(), abilities),
            count,
        )
        .with_text_roles(roles)
    }

    pub fn afterlife_spirits(count: u32, flying: A) -> Self {
        let face = creature("Spirit Token", vec![Subtype::Spirit], 1)
            .color_indicator(ColorSet::WHITE.union(ColorSet::BLACK));
        Self::keyword_token(face, vec![flying], count)
    }

    pub fn fabricate_servos(count: u32) -> Self {
        let face = creature("Servo Token", vec![Subtype::Servo], 1)
            .card_types(vec![CardType::Artifact, CardType::Creature]);
        Self::keyword_token(face, vec![], count)
    }

    pub fn for_mirrodin_rebel() -> Self {
        let face = creature("Rebel Token", vec![Subtype::Rebel], 2).color_indicator(ColorSet::RED);
        Self::keyword_token(face, vec![], 1)
    }

    pub fn job_select_hero() -> Self {
        Self::keyword_token(creature("Hero Token", vec![Subtype::Hero], 1), vec![], 1)
    }

    pub fn living_weapon_germ() -> Self {
        let face = creature(
            "Phyrexian Germ Token",
            vec![Subtype::Phyrexian, Subtype::Germ],
            0,
        )
        .color_indicator(ColorSet::BLACK);
        Self::keyword_token(face, vec![], 1)
    }

    pub fn mobilize_warriors(count: Value) -> Self {
        let face =
            creature("Warrior Token", vec![Subtype::Warrior], 1).color_indicator(ColorSet::RED);
        Self::keyword_token(face, vec![], count)
            .tapped()
            .attacking()
            .sacrifice_at_next_end_step()
    }
}

impl<E> ChooseModeEffect<E> {
    /// Fabricate uses the counters-else-tokens choice shared with endure.
    pub fn fabricate(amount: u32, counters: E, servos: E) -> Self {
        let (counter_label, token_label) = if amount == 1 {
            (
                "Put a +1/+1 counter on this creature".to_owned(),
                "Create a 1/1 colorless Servo artifact creature token".to_owned(),
            )
        } else {
            (
                format!("Put {amount} +1/+1 counters on this creature"),
                format!("Create {amount} 1/1 colorless Servo artifact creature tokens"),
            )
        };
        Self::choose_one(vec![
            EffectMode::new(counter_label, vec![counters]),
            EffectMode::new(token_label, vec![servos]),
        ])
        .as_endure()
    }
}
