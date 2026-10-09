use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::{Anthem, GrantObjectAbilityForFilter, StaticAbility};
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn enchanted_creature_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter
}

pub fn treasure_token_definition() -> CardDefinition {
    let mana_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![Cost::tap(), Cost::sacrifice_self()]),
            effects: vec![Effect::add_mana_of_any_color(1)].into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Treasure")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Treasure])
        .with_ability(mana_ability)
        .build()
}

pub fn food_token_definition() -> CardDefinition {
    let ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::tap(),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::gain_life(3)],
        crate::ability::ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Food")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Food])
        .with_ability(ability)
        .build()
}

pub fn blood_token_definition() -> CardDefinition {
    let ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
            Cost::tap(),
            Cost::discard(1, None),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::draw(1)],
        crate::ability::ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Blood")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Blood])
        .with_ability(ability)
        .build()
}

fn nonartifact_spell_restricted_mana_ability() -> crate::ability::Ability {
    let restriction = crate::ability::ManaUsageRestriction::PaymentTransaction {
        restriction: Some(crate::ability::ManaPaymentPredicate::Not(Box::new(
            crate::ability::ManaPaymentPredicate::All(vec![
                crate::ability::ManaPaymentPredicate::Purpose(
                    crate::ability::ManaPaymentPurpose::CastSpell,
                ),
                crate::ability::ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default().without_type(CardType::Artifact),
                ),
            ]),
        ))),
        on_spend: Vec::new(),
    };
    crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![Cost::tap()]),
            effects: Vec::new().into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![ManaSymbol::Colorless]),
            activation_condition: None,
            mana_usage_restrictions: vec![restriction],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn powerstone_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Powerstone")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Powerstone])
        .with_ability(nonartifact_spell_restricted_mana_ability())
        .build()
}

pub fn clue_token_definition() -> CardDefinition {
    let draw_ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::draw(1)],
        crate::ability::ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Clue")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Clue])
        .with_ability(draw_ability)
        .build()
}
/// CR 111.10h: "{1}, {T}, Sacrifice this artifact: Target creature you
/// control explores. Activate only as a sorcery."
pub fn map_token_definition() -> CardDefinition {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let explore_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
                Cost::tap(),
                Cost::sacrifice_self(),
            ]),
            effects: vec![Effect::explore(target.clone())].into(),
            choices: vec![target],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Map")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Map])
        .with_ability(explore_ability)
        .build()
}
/// Lander: "{2}, {T}, Sacrifice this token: Search your library for a basic
/// land card, put it onto the battlefield tapped, then shuffle."
pub fn lander_token_definition() -> CardDefinition {
    let searched = crate::tag::CompilerReferenceTag::Searched.bind();
    let search_ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::tap(),
            Cost::sacrifice_self(),
        ]),
        vec![
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::land()
                        .with_supertype(crate::types::Supertype::Basic)
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You),
                    1,
                    PlayerFilter::You,
                    searched.clone(),
                )
                .in_zone(Zone::Library)
                .as_search(),
            ),
            Effect::put_onto_battlefield(
                ChooseSpec::Tagged(searched.into()),
                true,
                PlayerFilter::You,
            ),
            Effect::shuffle_library_player(PlayerFilter::You),
        ],
        crate::ability::ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Lander")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Lander])
        .with_ability(search_ability)
        .build()
}
pub fn junk_token_definition() -> CardDefinition {
    let exile_tag = crate::tag::CompilerReferenceTag::JunkExiledCard.bind();
    let impulse_draw_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![Cost::tap(), Cost::sacrifice_self()]),
            effects: vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        crate::filter::ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        1,
                        PlayerFilter::You,
                        exile_tag.clone(),
                    )
                    .in_zone(Zone::Library)
                    .top_only(),
                ),
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
                    exile_tag.clone().into(),
                ))),
                Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    exile_tag.key.clone(),
                    PlayerFilter::You,
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                    true,
                    false,
                )),
            ]
            .into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Junk")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Junk])
        .with_ability(impulse_draw_ability)
        .build()
}
pub fn mutagen_token_definition() -> CardDefinition {
    let ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
                Cost::tap(),
                Cost::sacrifice_self(),
            ]),
            effects: vec![Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::target(ChooseSpec::creature()),
            )]
            .into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Mutagen")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Mutagen])
        .with_ability(ability)
        .build()
}
pub fn gold_token_definition() -> CardDefinition {
    let mana_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![Cost::sacrifice_self()]),
            effects: vec![Effect::add_mana_of_any_color(1)].into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Gold")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Gold])
        .with_ability(mana_ability)
        .build()
}
pub fn shard_token_definition() -> CardDefinition {
    let scry_and_draw_ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::scry(1), Effect::draw(1)],
        crate::ability::ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Shard")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Shard])
        .with_ability(scry_and_draw_ability)
        .build()
}
/// Walker: a 2/2 black Zombie creature token.
pub fn walker_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Walker")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .color_indicator(crate::color::ColorSet::BLACK)
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build()
}
pub fn wicked_role_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Wicked Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text(
            "Enchant creature\nEnchanted creature gets +1/+1.\nWhen this token is put into a graveyard from the battlefield, each opponent loses 1 life.",
        )
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            Anthem::<crate::ConditionExpr>::new(enchanted_creature_filter(), 1, 1),
        )))
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_dies(),
            vec![Effect::for_each_opponent(vec![Effect::lose_life_player(
                1,
                PlayerFilter::IteratedPlayer,
            )])],
        ))
        .build()
}
pub fn young_hero_role_token_definition() -> CardDefinition {
    let triggering = crate::tag::CompilerReferenceTag::Triggering.bind();
    let granted_trigger = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_attacks(),
            effects: vec![
                Effect::tag_triggering_object(triggering.clone()),
                Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::Tagged(triggering.clone().into()),
                ),
            ]
            .into(),
            choices: vec![],
            intervening_if: Some(crate::ConditionExpr::TaggedObjectMatches(
                triggering.into(),
                ObjectFilter::creature()
                    .with_toughness(crate::filter::Comparison::LessThanOrEqual(3)),
            )),
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    CardDefinitionBuilder::new(CardId::new(), "Young Hero Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text(
            "Enchant creature\nEnchanted creature has \"Whenever this creature attacks, if its toughness is 3 or less, put a +1/+1 counter on it.\"",
        )
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            crate::static_abilities::AttachedAbilityGrant::new(
                granted_trigger,
                "enchanted creature has whenever this creature attacks if its toughness is 3 or less put a +1/+1 counter on it",
            ),
        )))
        .build()
}
pub fn monster_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    CardDefinitionBuilder::new(CardId::new(), "Monster Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text("Enchant creature\nEnchanted creature gets +1/+1 and has trample.")
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            crate::static_abilities::Anthem::<crate::ConditionExpr>::new(enchanted.clone(), 1, 1),
        )))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            GrantObjectAbilityForFilter::from_static_grant(
                enchanted,
                StaticAbility::trample().into(),
            ),
        )))
        .build()
}
pub fn sorcerer_role_token_definition() -> CardDefinition {
    let granted_trigger = crate::ability::Ability::triggered(
        crate::triggers::Trigger::this_attacks(),
        vec![Effect::scry(1)],
    );
    CardDefinitionBuilder::new(CardId::new(), "Sorcerer Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text(
            "Enchant creature\nEnchanted creature gets +1/+1 and has \"Whenever this creature attacks, scry 1.\"",
        )
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            crate::static_abilities::Anthem::<crate::ConditionExpr>::new(enchanted_creature_filter(), 1, 1),
        )))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            crate::static_abilities::AttachedAbilityGrant::new(
                granted_trigger,
                "enchanted creature has whenever this creature attacks scry 1",
            ),
        )))
        .build()
}
pub fn royal_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    let ward_cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]);
    CardDefinitionBuilder::new(CardId::new(), "Royal Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text("Enchant creature\nEnchanted creature gets +1/+1 and has ward {1}.")
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            Anthem::<crate::ConditionExpr>::new(enchanted.clone(), 1, 1),
        )))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            GrantObjectAbilityForFilter::from_static_grant(
                enchanted,
                StaticAbility::ward(TotalCost::mana(ward_cost)).into(),
            ),
        )))
        .build()
}
/// CR 111.10 (Virtuous Role): "Enchant creature / Enchanted creature gets
/// +1/+1 for each enchantment you control." The count is a live layer-7c
/// characteristic of the enchanted creature, not a value locked on creation.
pub fn virtuous_role_token_definition() -> CardDefinition {
    let count = crate::static_abilities::AnthemCountExpression::MatchingFilter(
        ObjectFilter::enchantment().you_control(),
    );
    CardDefinitionBuilder::new(CardId::new(), "Virtuous Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text(
            "Enchant creature\nEnchanted creature gets +1/+1 for each enchantment you control.",
        )
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            Anthem::<crate::ConditionExpr>::new(enchanted_creature_filter(), 0, 0).with_values(
                crate::static_abilities::AnthemValue::scaled(1, count.clone()),
                crate::static_abilities::AnthemValue::scaled(1, count),
            ),
        )))
        .build()
}
pub fn cursed_role_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Cursed Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text("Enchant creature\nEnchanted creature has base power and toughness 1/1.")
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(
            StaticAbility::set_base_power_toughness(enchanted_creature_filter(), 1, 1),
        ))
        .build()
}

/// CR 111.10x. The subtype is part of the definition even where the reminder
/// text abbreviates it. This is a repeatable tap ability, not a sacrifice.
pub fn heartwood_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Heartwood")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Heartwood])
        .color_indicator(crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN))
        .oracle_text("{T}: Add {R} or {G}.")
        .with_ability(crate::ability::Ability::mana_with_effects(
            TotalCost::from_costs(vec![]),
            vec![Effect::add_mana_of_any_color_restricted(
                1,
                vec![crate::color::Color::Red, crate::color::Color::Green],
            )],
        ))
        .build()
}

/// CR 111.10w. The restriction prohibits only casting a nonartifact spell;
/// activation and other costs remain legal uses of the produced mana.
pub fn vibranium_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vibranium")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vibranium])
        .oracle_text(
            "Indestructible\n{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.",
        )
        .indestructible()
        .with_ability(nonartifact_spell_restricted_mana_ability())
        .build()
}

/// CR 111.11: complete printed characteristics, frozen from the canonical
/// Oracle card. Runtime execution uses this payload and never looks up a name.
pub fn gingerbrute_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Gingerbrute")
        .token().mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .card_types(vec![CardType::Artifact, CardType::Creature]).subtypes(vec![Subtype::Food, Subtype::Golem])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .oracle_text("Haste\n{1}: This creature can't be blocked this turn except by creatures with haste.\n{2}, {T}, Sacrifice this creature: You gain 3 life.")
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::haste()))
        .with_ability(crate::ability::Ability::activated_with_timing(
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
            vec![Effect::cant_until(crate::effect::Restriction::BlockSpecificAttacker {
                blockers: ObjectFilter::creature().without_static_ability(crate::static_abilities::StaticAbilityId::Haste),
                attacker: ObjectFilter::source(),
            }, crate::effect::Until::EndOfTurn)], crate::ability::ActivationTiming::AnyTime))
        .with_ability(crate::ability::Ability::activated_with_timing(
            TotalCost::from_costs(vec![Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
                Cost::tap(), Cost::sacrifice_self()]), vec![Effect::gain_life(3)], crate::ability::ActivationTiming::AnyTime))
        .build()
}

pub fn mutavault_token_definition() -> CardDefinition {
    let animation = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        crate::effect::Until::EndOfTurn,
    )
    .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
        power: crate::effect::Value::Fixed(2),
        toughness: crate::effect::Value::Fixed(2),
        sublayer: crate::continuous::PtSublayer::Setting,
    })
    .with_additional_modification(crate::continuous::Modification::AddAllSubtypesOfFamily(
        crate::types::SubtypeFamily::Creature,
    ));
    CardDefinitionBuilder::new(CardId::new(), "Mutavault")
        .token().card_types(vec![CardType::Land])
        .oracle_text("{T}: Add {C}.\n{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land.")
        .with_ability(crate::ability::Ability::mana(TotalCost::from_costs(vec![]), vec![ManaSymbol::Colorless]))
        .with_ability(crate::ability::Ability::activated_with_timing(
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
            vec![Effect::new(animation)], crate::ability::ActivationTiming::AnyTime))
        .build()
}

pub fn spellgorger_weird_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Spellgorger Weird")
        .token()
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Red,
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Weird])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .oracle_text("Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.")
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::spell_cast(
                Some(ObjectFilter::default().without_type(CardType::Creature)),
                PlayerFilter::You,
            ),
            vec![Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Source,
            )],
        ))
        .build()
}

pub fn tarmogoyf_token_definition() -> CardDefinition {
    let count = crate::effect::Value::CardTypesAmong(
        ObjectFilter::default().in_zone(Zone::Graveyard).nontoken(),
    );
    CardDefinitionBuilder::new(CardId::new(), "Tarmogoyf")
        .token().mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Green]))
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Lhurgoyf])
        .power_toughness(crate::card::PowerToughness::new(crate::card::PtValue::Star, crate::card::PtValue::StarPlus(1)))
        .oracle_text("Tarmogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.")
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::characteristic_defining_pt(
            count.clone(), crate::effect::Value::Add(Box::new(count), Box::new(crate::effect::Value::Fixed(1)))))
            .in_zones(vec![Zone::Library, Zone::Hand, Zone::Battlefield, Zone::Graveyard, Zone::Stack,
                Zone::Exile, Zone::Command, Zone::Ante, Zone::OutsideGame]))
        .build()
}
