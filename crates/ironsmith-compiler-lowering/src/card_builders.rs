use crate::ability::{ActivationTiming, PresentationKeyword, PresentationLabel};
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
pub use crate::cards::CardDefinition;
use crate::color::ColorSet;
pub use crate::cost::OptionalCost;
use crate::cost::TotalCost;
pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use crate::effect::EffectPredicate;
pub use crate::effect::{ChoiceCount, EventValueSpec, Value};
pub use ironsmith_core::ConditionConjunction;
use ironsmith_core::ConditionConjunction as _;

use crate::mana::ManaCost;
pub use crate::model::compiler_semantic::{
    ConditionalModeSelection, GiftTimingAst, LineAst, ParsedAbility, ParsedCardItem,
    ParsedConditionalModeChange, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst,
    ParsedLevelActivatedAbilityAst, ParsedLineAst, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
};
pub use crate::model::facts::{EffectLoweringContext, IdGenContext, LoweringFrame, MetadataLine};
pub use crate::model::reference::RefState;
pub use crate::model::reference_state::{ReferenceEnv, ReferenceImports};
pub use crate::model::{
    AdditionalCostChoiceOptionAst, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RedirectNextTimeDamageDestinationAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use crate::object::AuraAttachmentFilter;
pub use crate::payload::{IfResultPredicate, KeywordAction};
pub use ironsmith_compiler_source::LineInfo;

use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
pub use crate::tag::TagKey;
pub use crate::target::{ObjectFilter, PlayerFilter};
pub use crate::types::CardType;
use crate::types::{Subtype, Supertype};

pub use ironsmith_compiler_source::NormalizedLine;
pub use ironsmith_core::CardId;

pub use ironsmith_compiler_semantic::cards::builders::GrantedAbilityAst;

pub use ironsmith_compiler_syntax::OwnedLexToken;

pub use crate::model::ast::{
    CharacteristicActionAst, ChoiceActionAst, ChooseOneModeAst, ConditionalEffectAst,
    ControlActionAst, CounterActionAst, DamageActionAst, DamagePreventionActionAst,
    DelayedEffectAst, EffectAst, ExchangeActionAst, ForEachEffectAst, GameActionAst,
    GrantActionAst, KeywordActionAst, LibraryActionAst, LifeResourceActionAst, ManaActionAst,
    ObjectChoiceEffectAst, PermanentStateActionAst, PermissionEffectAst, PlayerPredicateAst,
    PredicateAst, RandomActionAst, ReplacementActionAst, RevealLookActionAst, SourcePredicateAst,
    StackActionAst, StatChangeActionAst, StaticAbilityAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TokenActionAst,
    TriggerFrequencyPredicateAst, TriggerSpec, TriggeringPredicateAst, TurnEventPredicateAst,
    TurnHistoryPredicateAst, TurnStructureActionAst, VoteEffectAst, ZoneMoveActionAst,
};

pub use ironsmith_compiler_semantic::cards::builders::InsteadSemantics;

#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    pub card_builder: CardBuilder,
    pub abilities: Vec<crate::ability::Ability>,
    pub spell_effect: Option<ResolutionProgram>,
    pub alternative_casts: Vec<crate::alternative_cast::AlternativeCastingMethod>,
    pub optional_costs: Vec<OptionalCost>,
    pub additional_cost: TotalCost,
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
    pub has_fuse: bool,
}

impl CardDefinitionBuilder {
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            aura_attach_filter: None,
            has_fuse: false,
        }
    }

    /// An empty definition seed, for lowering a card whose caller declared
    /// nothing up front. The placeholder face is always replaced by
    /// [`Self::with_face`] before the definition is built.
    pub fn seed() -> Self {
        Self::new(CardId::from_raw(0), String::new())
    }

    /// Separate the card's face facts from the definition being built.
    ///
    /// Recognition needs the face facts — a spell's type line changes how its
    /// text parses — but never the definition half, so only the face travels
    /// through the front end. The remainder is the seed lowering starts from,
    /// carrying anything a caller pre-declared (`.flying()`, an added cost).
    pub fn split_face(mut self) -> (CardBuilder, Self) {
        let card = std::mem::replace(
            &mut self.card_builder,
            CardBuilder::new(CardId::from_raw(0), String::new()),
        );
        (card, self)
    }

    /// Re-attach face facts to a definition seed once recognition has folded in
    /// every metadata line.
    pub fn with_face(mut self, card: CardBuilder) -> Self {
        self.card_builder = card;
        self
    }

    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
        self
    }

    pub fn first_printed_set_name(mut self, set_name: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.first_printed_set_name(set_name);
        self
    }

    pub fn attraction_lights(mut self, lights: Vec<u8>) -> Self {
        self.card_builder = self.card_builder.attraction_lights(lights);
        self
    }

    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.card_builder = self.card_builder.color_indicator(colors);
        self
    }

    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.card_builder = self.card_builder.supertypes(supertypes);
        self
    }

    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_builder = self.card_builder.card_types(types);
        self
    }

    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.card_builder = self.card_builder.subtypes(subtypes);
        self
    }

    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.oracle_text(text);
        self
    }

    pub fn other_face(mut self, face: CardId) -> Self {
        self.card_builder = self.card_builder.other_face(face);
        self
    }

    pub fn other_face_name(mut self, name: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.other_face_name(name);
        self
    }

    pub fn linked_face_layout(mut self, layout: LinkedFaceLayout) -> Self {
        self.card_builder = self.card_builder.linked_face_layout(layout);
        self
    }

    pub fn has_fuse(mut self) -> Self {
        self.has_fuse = true;
        self
    }

    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.card_builder = self.card_builder.power_toughness(pt);
        self
    }

    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.card_builder = self.card_builder.loyalty(loyalty);
        self
    }

    pub fn defense(mut self, defense: u32) -> Self {
        self.card_builder = self.card_builder.defense(defense);
        self
    }

    pub fn token(mut self) -> Self {
        self.card_builder = self.card_builder.token();
        self
    }

    pub fn with_abilities(mut self, abilities: Vec<crate::ability::Ability>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    pub fn with_ability(mut self, ability: crate::ability::Ability) -> Self {
        self.abilities.push(ability);
        self
    }

    pub fn with_level_abilities(mut self, abilities: Vec<crate::ability::LevelAbility>) -> Self {
        self.abilities.extend(abilities.into_iter().map(|ability| {
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::level(
                ability,
            ))
        }));
        self
    }

    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.optional_costs.push(cost);
        self
    }

    pub fn alternative_cast(
        mut self,
        method: crate::alternative_cast::AlternativeCastingMethod,
    ) -> Self {
        self.alternative_casts.push(method);
        self
    }

    pub fn enchants(mut self, filter: AuraAttachmentFilter) -> Self {
        let mut attachment =
            ResolutionProgram::from_effects(vec![crate::effect::Effect::attach_to(
                filter.target_spec(),
            )]);
        if let Some(existing) = self.spell_effect.take() {
            attachment.extend(existing);
        }
        self.spell_effect = Some(attachment);
        self.aura_attach_filter = Some(filter);
        self
    }

    pub fn apply_keyword_action(self, action: KeywordAction) -> Self {
        crate::keyword_actions::apply_keyword_action(self, action)
    }

    pub fn flying(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        ))
    }

    pub fn defender(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::defender(),
        ))
    }

    pub fn decayed(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("decayed"),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::cant_block(),
        ))
        .with_ability(crate::runtime_static_ability_helpers::decayed_triggered_ability())
    }

    pub fn vigilance(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::vigilance(),
        ))
    }

    pub fn prowess(self) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::spell_cast(
                    Some(crate::target::ObjectFilter::noncreature_spell()),
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::pump(
                    1,
                    1,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Prowess)),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn trample(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        ))
    }

    pub fn lifelink(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::lifelink(),
        ))
    }

    pub fn deathtouch(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::deathtouch(),
        ))
    }

    pub fn haste(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        ))
    }

    pub fn menace(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::menace(),
        ))
    }

    pub fn reach(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::reach(),
        ))
    }

    pub fn hexproof(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::hexproof(),
        ))
    }

    pub fn indestructible(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ))
    }

    /// CR 702.164a: toxic is a static ability; the poison counters are a
    /// result of the combat damage itself (702.164c), not a trigger.
    pub fn toxic(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::toxic(amount),
        ))
    }

    pub fn poisonous(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_deals_combat_damage_to_player(
                    crate::target::PlayerFilter::Any,
                ),
                effects: vec![crate::effect::Effect::poison_counters_player(
                    amount as i32,
                    crate::target::PlayerFilter::DamagedPlayer,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(
                    PresentationKeyword::Poisonous(amount),
                )),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn ward_generic(self, amount: u32) -> Self {
        let mana = ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(amount as u8)]);
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::ward(TotalCost::mana(mana)),
        ))
    }

    pub fn first_strike(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::first_strike(),
        ))
    }

    pub fn double_strike(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::double_strike(),
        ))
    }

    pub fn afflict(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_becomes_blocked(),
                effects: vec![crate::effect::Effect::lose_life_player(
                    amount as i32,
                    crate::target::PlayerFilter::Defending,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Afflict(
                    amount,
                ))),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    /// CR 702.38a: "As this object enters, reveal any number of cards ...
    /// This permanent enters with N +1/+1 counters on it for each card
    /// revealed this way." An "as enters" replacement, not a trigger.
    pub fn amplify(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                vec![crate::effect::Effect::amplify(amount)].into(),
                "this creature",
                false,
                false,
                Some(PresentationLabel::Keyword(PresentationKeyword::Amplify(
                    amount,
                ))),
            ),
        ))
    }

    /// CR 702.82a: "As this object enters, you may sacrifice any number of
    /// creatures. This permanent enters with N +1/+1 counters on it for each
    /// creature sacrificed this way." An "as enters" replacement.
    pub fn devour(self, multiplier: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                vec![crate::effect::Effect::devour(multiplier)].into(),
                "this creature",
                false,
                false,
                Some(PresentationLabel::Keyword(PresentationKeyword::Devour(
                    multiplier,
                ))),
            ),
        ))
    }

    pub fn afterlife(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_dies(),
            vec![crate::effect::Effect::create_tokens(
                Self::afterlife_spirit_token(),
                amount,
            )],
        ))
    }

    pub fn fabricate(self, amount: u32) -> Self {
        let put_description = if amount == 1 {
            "Put a +1/+1 counter on this creature".to_string()
        } else {
            format!("Put {amount} +1/+1 counters on this creature")
        };
        let create_description = if amount == 1 {
            "Create a 1/1 colorless Servo artifact creature token".to_string()
        } else {
            format!("Create {amount} 1/1 colorless Servo artifact creature tokens")
        };
        let modes = vec![
            crate::effect::EffectMode::new(
                put_description,
                vec![crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    amount as i32,
                    crate::target::ChooseSpec::Source,
                )],
            ),
            crate::effect::EffectMode::new(
                create_description,
                vec![crate::effect::Effect::create_tokens(
                    Self::fabricate_servo_token(),
                    amount,
                )],
            ),
        ];

        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::choose_one(modes)],
        ))
    }

    pub fn exalted(self) -> Self {
        let attacker_tag = crate::tag::CompilerReferenceTag::ExaltedAttacker.bind();
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::attacks_alone(
                crate::target::ObjectFilter::creature().you_control(),
            ),
            vec![
                crate::effect::Effect::tag_triggering_object(attacker_tag.clone()),
                crate::effect::Effect::pump(
                    1,
                    1,
                    crate::target::ChooseSpec::Tagged(attacker_tag.key.clone()),
                    crate::effect::Until::EndOfTurn,
                ),
            ],
        ))
    }

    pub fn storm(self) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::new(
                            crate::target::ChooseSpec::Source,
                            // CR 702.40a: every player's spells count toward storm.
                            crate::effect::Value::SpellsCastBeforeThisTurn(
                                crate::target::PlayerFilter::Any,
                            ),
                        )),
                    ),
                    crate::effect::Effect::new(crate::effects::ChooseNewTargetsEffect::may(
                        crate::effect::EffectId(0),
                    )),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn gravestorm(self) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::new(
                            crate::target::ChooseSpec::Source,
                            crate::effect::Value::TurnHistoryCount(
                                ironsmith_core::TurnHistoryCount::died(
                                    crate::target::ObjectFilter::default(),
                                ),
                            ),
                        )),
                    ),
                    crate::effect::Effect::new(crate::effects::ChooseNewTargetsEffect::may(
                        crate::effect::EffectId(0),
                    )),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn battle_cry(self) -> Self {
        // CR 702.91a: "each other attacking creature", not only yours.
        let mut filter = crate::target::ObjectFilter::creature().other();
        filter.attacking = true;
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::for_each(
                filter,
                vec![crate::effect::Effect::pump(
                    1,
                    0,
                    crate::target::ChooseSpec::Iterated,
                    crate::effect::Until::EndOfTurn,
                )],
            )],
        ))
    }

    /// CR 702.121a: "Whenever this creature attacks, it gets +1/+1 until end
    /// of turn for each opponent you attacked with a creature this combat."
    /// Only attacked players count (attacking a planeswalker or battle doesn't
    /// attack its controller), taken from this combat's attack declarations.
    pub fn melee(self) -> Self {
        let opponents_attacked = crate::effect::Value::TurnHistoryCount(
            ironsmith_core::TurnHistoryCount::PlayersAttackedThisCombat(
                crate::target::PlayerFilter::You,
            ),
        );
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: vec![crate::effect::Effect::pump(
                    opponents_attacked.clone(),
                    opponents_attacked,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Melee)),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn firebending(self, amount: u32) -> Self {
        self.firebending_with_surface(
            crate::effect::Value::Fixed(amount as i32),
            amount.to_string(),
        )
    }

    pub(crate) fn firebending_with_surface(
        self,
        amount: crate::effect::Value,
        presentation: String,
    ) -> Self {
        let add_mana = crate::effect::Effect::new(crate::effects::mana::AddScaledManaEffect::new(
            vec![crate::mana::ManaSymbol::Red],
            amount,
            crate::target::PlayerFilter::You,
        ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::mana_retained_until_end_of_combat(vec![add_mana]),
                    crate::effect::Effect::emit_keyword_action(
                        crate::events::KeywordActionKind::Firebend,
                        1,
                    ),
                ]),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(
                    PresentationKeyword::Firebending(presentation),
                )),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn dethrone(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks_player_with_most_life(),
            vec![crate::effect::Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::Source,
            )],
        ))
    }

    pub fn evolve(self) -> Self {
        // CR 702.100a: "Whenever a creature you control enters, if that
        // creature's power is greater than this creature's power and/or ..."
        // is an intervening-if, checked on triggering and resolution (603.4).
        let mut ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(
                crate::target::ObjectFilter::creature().you_control(),
                None,
            ),
            vec![crate::effect::Effect::new(
                crate::effects::EvolveEffect::new(),
            )],
        );
        if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.intervening_if =
                Some(crate::effect::Condition::EvolveEnteringCreatureIsLarger);
        }
        self.with_ability(ability)
    }

    pub fn ingest(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(
                crate::target::PlayerFilter::Any,
            ),
            vec![crate::effect::Effect::exile_top_of_library_player(
                1,
                crate::target::PlayerFilter::DamagedPlayer,
                ironsmith_compiler_semantic::tag::declared_key("ingested"),
                None,
            )],
        ))
    }

    pub fn mentor(self) -> Self {
        let mut target_filter =
            crate::target::ObjectFilter::creature().with_power_less_than_source();
        target_filter.attacking = true;
        let target =
            crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(target_filter));

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        target.clone(),
                    ),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn training(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks_with_greater_power(),
            vec![
                crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                ),
                crate::effect::Effect::emit_keyword_action(
                    crate::events::KeywordActionKind::Train,
                    1,
                ),
            ],
        ))
    }

    pub fn enlist(self) -> Self {
        let linked_trigger = crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_attacks(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::pump_for_each(
                    crate::target::ChooseSpec::Source,
                    1,
                    0,
                    crate::effect::Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(
                        ironsmith_compiler_semantic::tag::declared_key("enlisted_creature").into(),
                    ))),
                    crate::effect::Until::EndOfTurn,
                ),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enlist_attack(linked_trigger, "Enlist"),
        ))
    }

    pub fn crew(
        self,
        amount: u32,
        timing: crate::ability::ActivationTiming,
        additional_restrictions: Vec<String>,
    ) -> Self {
        let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
            crate::effects::CrewCostEffect::new(amount),
        ));
        let animate = crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddCardTypes(vec![crate::types::CardType::Creature]),
            crate::effect::Until::EndOfTurn,
        ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![animate]),
                choices: Vec::new(),
                timing,
                additional_restrictions,
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn riot(self) -> Self {
        let modes = vec![
            crate::effect::EffectMode::new(
                "This creature enters with a +1/+1 counter on it",
                vec![crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                )],
            ),
            crate::effect::EffectMode::new(
                "This creature gains haste",
                vec![crate::effect::Effect::grant_object_ability_to_source(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::haste(),
                    ),
                )],
            ),
        ];

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                vec![crate::effect::Effect::choose_one(modes)].into(),
                "this creature",
                false,
                false,
                None,
            ),
        ))
    }

    pub fn unleash(self) -> Self {
        // CR 702.98a: "You may have this permanent enter with an additional
        // +1/+1 counter on it" is an entry replacement (CR 614.1c), not an
        // ETB trigger, so it is lowered like riot.
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        crate::target::ChooseSpec::Source,
                    ),
                ])]
                .into(),
                "this creature",
                false,
                false,
                None,
            ),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::unleash(),
        ))
    }

    pub fn outlast(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::tap(),
        ]);

        self.with_ability(crate::ability::Ability::activated_with_timing(
            total_cost,
            vec![crate::effect::Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::Source,
            )],
            crate::ability::ActivationTiming::SorcerySpeed,
        ))
    }

    pub fn scavenge(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature());

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        crate::effect::Value::SourcePower,
                        target.clone(),
                    ),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn unearth(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::unearth()],
                crate::ability::ActivationTiming::SorcerySpeed,
            )
            .in_zones(vec![crate::zone::Zone::Graveyard]),
        )
    }

    pub fn embalm(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let create_embalmed_copy = crate::effect::Effect::new(
            crate::effects::CreateTokenCopyEffect::new(
                crate::target::ChooseSpec::Source,
                1,
                crate::target::PlayerFilter::You,
            )
            .set_colors(crate::color::ColorSet::WHITE)
            .added_subtype(crate::types::Subtype::Zombie)
            .without_mana_cost(),
        );

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    create_embalmed_copy,
                ]),
                choices: vec![],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn encore(self, cost: ManaCost) -> Self {
        let cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let mut copy = crate::effects::CreateTokenCopyEffect::new(
            crate::target::ChooseSpec::Source,
            1,
            crate::target::PlayerFilter::You,
        )
        .sacrifice_at_next_end_step(true);
        copy.must_attack_player_this_turn = Some(crate::target::PlayerFilter::IteratedPlayer);
        let effects = vec![
            crate::effect::Effect::for_each_opponent(vec![
                crate::effect::Effect::new(copy).tag("encore_tokens"),
            ]),
            crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(crate::target::ObjectFilter::tagged(
                    "encore_tokens",
                )),
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::haste(),
                ),
                crate::effect::Until::Forever,
            )),
        ];
        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                cost,
                effects,
                crate::ability::ActivationTiming::SorcerySpeed,
            )
            .in_zones(vec![crate::zone::Zone::Graveyard]),
        )
    }

    pub fn eternalize(self, cost: TotalCost) -> Self {
        fn append_exile_source(cost: &TotalCost) -> TotalCost {
            match cost.kind() {
                ironsmith_core::TotalCostKind::All(costs) => {
                    let mut costs = costs.clone();
                    costs.push(crate::costs::Cost::exile_self());
                    TotalCost::from_costs(costs)
                }
                ironsmith_core::TotalCostKind::OneOf(branches) => {
                    TotalCost::one_of(branches.iter().map(append_exile_source).collect())
                }
            }
        }
        let total_cost = append_exile_source(&cost);
        let create_eternalized_copy = crate::effect::Effect::new(
            crate::effects::CreateTokenCopyEffect::new(
                crate::target::ChooseSpec::Source,
                1,
                crate::target::PlayerFilter::You,
            )
            .set_colors(crate::color::ColorSet::BLACK)
            .added_subtype(crate::types::Subtype::Zombie)
            .set_base_power_toughness(4, 4)
            .without_mana_cost(),
        );

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    create_eternalized_copy,
                ]),
                choices: vec![],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn emerge(self, cost: ManaCost) -> Self {
        self.alternative_cast(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                "Emerge",
                Some(cost),
                vec![crate::costs::Cost::sacrifice(
                    crate::target::ObjectFilter::creature().you_control(),
                )],
            ),
        )
    }

    pub fn aura_swap(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(crate::ability::Ability::activated(
            total_cost,
            vec![crate::effect::Effect::aura_swap()],
        ))
    }

    pub fn ninjutsu(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::effect(crate::effect::Effect::new(
                crate::effects::NinjutsuCostEffect::new(),
            )),
        ]);

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::ninjutsu()],
                crate::ability::ActivationTiming::DuringCombat,
            )
            .in_zones(vec![crate::zone::Zone::Hand]),
        )
    }

    /// CR 702.112a: "When this creature deals combat damage to a player, if
    /// it isn't renowned, ..." The renowned check is an intervening if.
    pub fn renown(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_deals_combat_damage_to_player(
                    crate::target::PlayerFilter::Any,
                ),
                effects: vec![crate::effect::Effect::renown_source(amount)].into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::SourceIsRenowned,
                ))),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn soulbond(self) -> Self {
        // CR 702.95a: pairing is gated by an intervening-if (CR 603.4).
        let mut ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(
                crate::target::ObjectFilter::creature().you_control(),
                None,
            ),
            vec![crate::effect::Effect::new(
                crate::effects::SoulbondPairEffect::new(),
            )],
        );
        if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.intervening_if = Some(crate::effect::Condition::SoulbondPairingPossible);
        }
        self.with_ability(ability)
    }

    pub fn soulshift(self, amount: u32) -> Self {
        self.with_ability(Self::soulshift_triggered_ability(
            crate::filter::Comparison::LessThanOrEqual(amount as i32),
            None,
        ))
    }

    pub fn soulshift_value(self, amount: Value) -> Self {
        self.with_ability(Self::soulshift_triggered_ability(
            crate::filter::Comparison::LessThanOrEqualExpr(Box::new(amount)),
            Some(PresentationLabel::Keyword(PresentationKeyword::Soulshift(
                "X".to_string(),
            ))),
        ))
    }

    pub fn soulshift_triggered_ability_from_value(amount: Value) -> crate::ability::Ability {
        Self::soulshift_triggered_ability(
            crate::filter::Comparison::LessThanOrEqualExpr(Box::new(amount)),
            Some(PresentationLabel::Keyword(PresentationKeyword::Soulshift(
                "X".to_string(),
            ))),
        )
    }

    fn soulshift_triggered_ability(
        mana_value: crate::filter::Comparison,
        presentation_label: Option<PresentationLabel>,
    ) -> crate::ability::Ability {
        let filter = crate::target::ObjectFilter::default()
            .with_subtype(Subtype::Spirit)
            .owned_by(crate::target::PlayerFilter::You)
            .in_zone(crate::zone::Zone::Graveyard)
            .with_mana_value(mana_value);
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(filter))
            .with_count(ChoiceCount::up_to(1));

        crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::return_from_graveyard_to_hand(target.clone()),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        }
    }

    pub fn recover(self, cost: ManaCost) -> Self {
        let payment_id = crate::effect::EffectId(0);
        let cost_text = cost.to_oracle();
        let trigger = crate::triggers::Trigger::new(
            crate::triggers::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Battlefield)
                .to(crate::zone::Zone::Graveyard)
                .filter(
                    crate::target::ObjectFilter::creature()
                        .owned_by(crate::target::PlayerFilter::You),
                ),
        );

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::conditional_only(
                        crate::effect::Condition::SourceIsInZone(crate::zone::Zone::Graveyard),
                        vec![
                            crate::effect::Effect::with_id(
                                payment_id.0,
                                crate::effect::Effect::may(vec![crate::effect::Effect::new(
                                    crate::effects::PayManaEffect::new(
                                        cost,
                                        crate::target::ChooseSpec::SourceController,
                                    ),
                                )]),
                            ),
                            crate::effect::Effect::if_then(
                                payment_id,
                                crate::effect::EffectPredicate::Happened,
                                vec![crate::effect::Effect::return_from_graveyard_to_hand(
                                    crate::target::ChooseSpec::Source,
                                )],
                            ),
                            crate::effect::Effect::if_then(
                                payment_id,
                                crate::effect::EffectPredicate::DidNotHappen,
                                vec![crate::effect::Effect::exile(
                                    crate::target::ChooseSpec::Source,
                                )],
                            ),
                        ],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Recover(
                    cost_text,
                ))),
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn backup(self, amount: u32) -> Self {
        let text = format!("Backup {amount}");
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker(text.clone()),
        ))
    }

    pub fn cipher(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("Cipher"),
        ))
    }

    pub fn dash(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Dash { cost });
        self
    }

    pub fn blitz(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Blitz {
                total_cost: TotalCost::mana(cost),
            });
        self
    }

    pub fn warp(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Warp {
                cost,
                additional_cost: Default::default(),
            });
        self
    }

    pub fn plot(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Plot { cost });
        self
    }

    pub fn suspend(self, time: u32, cost: ManaCost) -> Self {
        self.alternative_cast(crate::alternative_cast::AlternativeCastingMethod::Suspend {
            cost,
            time,
        })
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::remove_counters(
                    crate::object::CounterType::Time,
                    1,
                    crate::target::ChooseSpec::Source,
                )]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasCounterAtLeast {
                    counter_type: crate::object::CounterType::Time,
                    count: 1,
                    surface: crate::SourceCounterThresholdSurface::SourceHas,
                }),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
        })
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(
                        crate::target::ObjectFilter::source(),
                    ),
                ),
                effects: vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::new(
                        crate::effects::CastSourceEffect::new()
                            .without_paying_mana_cost()
                            .require_exile()
                            .cast_as_suspend(),
                    ),
                ])]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasNoCounter(
                    crate::object::CounterType::Time,
                )),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
        })
    }

    pub fn disturb(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Disturb { cost });
        self
    }

    pub fn overload(mut self, cost: ManaCost) -> Self {
        self.alternative_casts.push(
            crate::alternative_cast::AlternativeCastingMethod::Overload {
                cost,
                effects: Vec::new(),
            },
        );
        self
    }

    pub fn cleave(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Cleave {
                cost,
                effects: Vec::new(),
            });
        self
    }

    pub fn awaken(mut self, amount: u32, cost: ManaCost) -> Self {
        let mut effects = self
            .spell_effect
            .as_ref()
            .map(|program| program.all_effects_owned())
            .unwrap_or_default();
        let spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            ObjectFilter::land().you_control(),
        ));
        effects.push(crate::effect::Effect::new(
            crate::effects::EarthbendEffect::awaken(spec, amount),
        ));
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Awaken {
                amount,
                cost,
                effects,
            });
        self
    }

    pub fn foretell(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Foretell { cost });
        self
    }

    pub fn spectacle(mut self, cost: ManaCost) -> Self {
        self.alternative_casts.push(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost_with_condition(
                "Spectacle",
                Some(cost),
                Vec::new(),
                crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::OpponentLostLifeThisTurn,
                    display: "an opponent lost life this turn".to_string(),
                },
            ),
        );
        self
    }

    /// Miracle (CR 702.94): an alternative cost plus the linked hand-zone
    /// trigger "When you draw this card, if it's the first card you've drawn
    /// this turn, you may reveal it. If you do, you may cast it by paying its
    /// miracle cost."
    pub fn miracle(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Miracle { cost });
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::miracle(),
                effects: vec![crate::effect::Effect::may_cast_for_miracle_cost()].into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Hand],
        })
    }

    pub fn echo(self, total_cost: TotalCost) -> Self {
        // CR 702.30a: "At the beginning of your upkeep, if this permanent came
        // under your control since the beginning of your last upkeep, sacrifice
        // it unless you pay [cost]." The control check is a tracked game fact,
        // not a counter, so counter effects can't change it.
        let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::unless_action(
                    vec![crate::effect::Effect::sacrifice_source()],
                    payment_effects,
                    crate::target::PlayerFilter::You,
                )]
                .into(),
                choices: vec![],
                intervening_if: Some(
                    crate::effect::Condition::SourceCameUnderYourControlSinceYourLastUpkeep,
                ),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn cumulative_upkeep(self, total_cost: TotalCost) -> Self {
        let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters_on_source(
                        crate::object::CounterType::Age,
                        1,
                    ),
                    crate::effect::Effect::cumulative_upkeep(
                        payment_effects,
                        crate::target::PlayerFilter::You,
                        vec![crate::effect::Effect::sacrifice_source()],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    /// Casualty N (CR 702.153a): the sacrifice is an optional additional cost
    /// paid while casting; the cast trigger only copies the spell when it was
    /// paid.
    pub fn casualty(mut self, power: u32) -> Self {
        let mut creature_filter = crate::target::ObjectFilter::creature().you_control();
        creature_filter.power = Some(crate::filter::Comparison::GreaterThanOrEqual(power as i32));

        self.optional_costs.push(OptionalCost::custom(
            "Casualty",
            TotalCost::from_cost(crate::costs::Cost::sacrifice(creature_filter)),
        ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::single(
                            crate::target::ChooseSpec::Source,
                        )),
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(0),
                        crate::target::PlayerFilter::You,
                    ),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel("Casualty".into())),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    /// Casualty X on a planeswalker spell (Ob Nixilis, the Adversary): the
    /// sacrifice is an optional cost paid while casting (CR 702.153a), and the
    /// copy's starting loyalty is the sacrificed creature's power.
    pub fn variable_casualty_planeswalker_copy(mut self) -> Self {
        self.optional_costs.push(OptionalCost::custom(
            "Casualty",
            TotalCost::from_cost(crate::costs::Cost::sacrifice(
                crate::target::ObjectFilter::creature().you_control(),
            )),
        ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::new(
                        crate::effects::VariableCasualtyPlaneswalkerCopyEffect::new(),
                    ),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel("Casualty".into())),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn demonstrate(self) -> Self {
        let opponent_tag = crate::tag::CompilerReferenceTag::DemonstrateOpponent.bind();
        let opponent = crate::target::PlayerFilter::TaggedPlayer(opponent_tag.clone().into());
        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::you_cast_this_spell(),
                vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::single(
                            crate::target::ChooseSpec::Source,
                        )),
                    ),
                    crate::effect::Effect::new(crate::effects::ChoosePlayerEffect::new(
                        crate::target::PlayerFilter::You,
                        crate::target::PlayerFilter::Opponent,
                        opponent_tag.clone(),
                    )),
                    crate::effect::Effect::with_id(
                        1,
                        crate::effect::Effect::new(
                            crate::effects::CopySpellEffect::new_for_player(
                                crate::target::ChooseSpec::Source,
                                1,
                                opponent.clone(),
                            ),
                        ),
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(0),
                        crate::target::PlayerFilter::You,
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(1),
                        opponent,
                    ),
                ])],
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn conspire(mut self) -> Self {
        let existing_instances = self
            .optional_costs
            .iter()
            .filter(|cost| {
                cost.source_label == "Conspire" || cost.source_label.starts_with("Conspire ")
            })
            .count();
        let label = if existing_instances == 0 {
            "Conspire".to_string()
        } else {
            format!("Conspire {}", existing_instances + 1)
        };
        let cost = TotalCost::from_cost(crate::costs::Cost::effect(crate::effect::Effect::new(
            crate::effects::ConspireCostEffect::new(),
        )));
        self.optional_costs
            .push(OptionalCost::custom(label.clone(), cost));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::single(
                            crate::target::ChooseSpec::Source,
                        )),
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(0),
                        crate::target::PlayerFilter::You,
                    ),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel(label.into())),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn ravenous(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                crate::effect::Value::X,
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_enters_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::draw(1),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::XValueAtLeast(5)),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    /// Storied: "If you control three or more artifacts, legendaries, and/or
    /// Sagas, you have an enduring story for the rest of the game."
    pub fn storied(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::storied(),
        ))
    }

    pub fn ascend(self) -> Self {
        let is_nonpermanent_spell = self
            .card_builder
            .card_types_ref()
            .iter()
            .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
        if is_nonpermanent_spell {
            let mut out = self;
            let mut effects = out.spell_effect.take().unwrap_or_default();
            effects.insert(
                0,
                crate::effect::Effect::new(ironsmith_core::AscendEffect::new()),
            );
            out.spell_effect = Some(effects);
            return out;
        }

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::ascend(),
        ))
    }

    pub fn daybound(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::daybound(),
        ))
    }

    pub fn nightbound(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::nightbound(),
        ))
    }

    pub fn haunt(self) -> Self {
        let trigger = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            crate::triggers::Trigger::this_dies()
        } else {
            crate::triggers::Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(crate::zone::Zone::Stack)
                    .to(crate::zone::Zone::Graveyard)
                    .this(),
            )
        };
        let functional_zones = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            vec![crate::zone::Zone::Battlefield]
        } else {
            vec![crate::zone::Zone::Graveyard]
        };

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::exile(crate::target::ChooseSpec::Source),
                ]),
                choices: vec![crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::creature(),
                )],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones,
        })
    }

    /// CR 702.39a: "Whenever this creature attacks, you may choose to have
    /// target creature defending player controls block this creature this
    /// combat if able. If you do, untap that creature."
    pub fn provoke(self) -> Self {
        const PROVOKED_TAG: &str = "__provoked_creature";
        let target_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::creature()
                .controlled_by(crate::target::PlayerFilter::Defending),
        ));
        let declare_target =
            crate::effect::Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                .tag(PROVOKED_TAG);
        // The requirement names this attacker, so blocking a different
        // attacker doesn't satisfy it (CR 509.1c).
        let must_block_this = crate::effect::Effect::cant_until(
            crate::effect::Restriction::must_block_specific_attacker(
                crate::target::ObjectFilter::tagged(PROVOKED_TAG),
                crate::target::ObjectFilter::source(),
            ),
            crate::effect::Until::EndOfCombat,
        );
        let choose = crate::effect::Effect::with_id(
            0,
            crate::effect::Effect::may(vec![declare_target, must_block_this]),
        );
        let untap = crate::effect::Effect::if_then(
            0,
            crate::effect::EffectPredicate::Happened,
            vec![crate::effect::Effect::untap(
                crate::target::ChooseSpec::tagged(PROVOKED_TAG),
            )],
        );
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![choose, untap]),
                choices: vec![target_spec],
                intervening_if: None,
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Provoke)),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn undaunted(self) -> Self {
        let reduction = crate::static_abilities::CostReduction::<crate::ConditionExpr>::new(
            crate::target::ObjectFilter::default(),
            crate::effect::Value::CountPlayers(crate::target::PlayerFilter::Opponent),
        );
        self.with_ability(
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::new(
                reduction,
            ))
            .in_zones(vec![crate::zone::Zone::Stack, crate::zone::Zone::Hand]),
        )
    }

    pub fn extort(self) -> Self {
        let pay_cost = ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::White,
            crate::mana::ManaSymbol::Black,
        ]]);
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::spell_cast(
                    None,
                    crate::target::PlayerFilter::You,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::may(vec![crate::effect::Effect::new(
                            crate::effects::PayManaEffect::new(
                                pay_cost,
                                crate::target::ChooseSpec::SourceController,
                            ),
                        )]),
                    ),
                    crate::effect::Effect::if_then(
                        crate::effect::EffectId(0),
                        crate::effect::EffectPredicate::Happened,
                        vec![
                            crate::effect::Effect::with_id(
                                1,
                                crate::effect::Effect::for_each_opponent(vec![
                                    crate::effect::Effect::lose_life_player(
                                        1,
                                        crate::target::PlayerFilter::IteratedPlayer,
                                    ),
                                ]),
                            ),
                            crate::effect::Effect::gain_life(crate::effect::Value::EffectValue(
                                crate::effect::EffectId(1),
                            )),
                        ],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn partner(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::partner(),
        ))
    }

    pub fn assist(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::assist(),
        ))
    }

    pub fn split_second(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::split_second(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn cascade(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::cascade(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn rebound(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::rebound(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn read_ahead(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::read_ahead(),
        ))
    }

    pub fn sunburst(self) -> Self {
        let counter_type = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            crate::object::CounterType::PlusOnePlusOne
        } else {
            crate::object::CounterType::Charge
        };

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("Sunburst"),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                counter_type,
                crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
    }

    pub fn for_mirrodin(self) -> Self {
        let created_tag = crate::tag::CompilerReferenceTag::ForMirrodinCreated.bind();
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![
                crate::effect::Effect::create_tokens(Self::for_mirrodin_rebel_token(), 1)
                    .tag(created_tag.clone()),
                crate::effect::Effect::attach_to(crate::target::ChooseSpec::Tagged(
                    created_tag.key.clone(),
                )),
            ],
        ))
    }

    pub fn living_weapon(self) -> Self {
        let created_tag = crate::tag::CompilerReferenceTag::LivingWeaponCreated.bind();
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![
                crate::effect::Effect::create_tokens(Self::living_weapon_germ_token(), 1)
                    .tag(created_tag.clone()),
                crate::effect::Effect::attach_to(crate::target::ChooseSpec::Tagged(
                    created_tag.key.clone(),
                )),
            ],
        ))
    }

    pub fn myriad(self) -> Self {
        let opponent_other_than_defending = crate::target::PlayerFilter::excluding(
            crate::target::PlayerFilter::Opponent,
            crate::target::PlayerFilter::Defending,
        );
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::for_players(
                opponent_other_than_defending,
                vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::new(
                        crate::effects::CreateTokenCopyEffect::new(
                            crate::target::ChooseSpec::Source,
                            1,
                            crate::target::PlayerFilter::You,
                        )
                        .enters_tapped(true)
                        .attacking_player_or_planeswalker_controlled_by(
                            crate::target::PlayerFilter::IteratedPlayer,
                        )
                        .exile_at_eoc(true),
                    ),
                ])],
            )],
        ))
    }

    pub fn mobilize(self, amount: u32) -> Self {
        let effect = crate::effects::CreateTokenEffect::new(
            Self::mobilize_warrior_token(),
            amount,
            crate::target::PlayerFilter::You,
        )
        .tapped()
        .attacking()
        .sacrifice_at_next_end_step();

        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::new(effect)],
        ))
    }

    pub fn fading(self, amount: u32) -> Self {
        // CR 702.32a: "At the beginning of your upkeep, remove a fade counter
        // from it. If you can't, sacrifice the permanent." Removing the last
        // counter doesn't sacrifice it; the next upkeep's failed removal does.
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::Fade,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![
                crate::effect::Effect::with_id(
                    0,
                    crate::effect::Effect::remove_counters(
                        crate::object::CounterType::Fade,
                        1,
                        crate::target::ChooseSpec::Source,
                    ),
                ),
                crate::effect::Effect::if_then(
                    crate::effect::EffectId(0),
                    crate::effect::EffectPredicate::DidNotHappen,
                    vec![crate::effect::Effect::sacrifice_source()],
                ),
            ],
        ))
    }

    pub fn impending(self, time: u32, cost: ManaCost) -> Self {
        let paid = crate::ConditionExpr::ThisSpellPaidLabel(ironsmith_core::OptionalCostRef::new(
            ironsmith_core::OptionalCostKind::AlternativeCast(
                ironsmith_core::AlternativeCostReference::by_name("Impending", Some(&cost)),
            ),
        ));
        let active = paid
            .clone()
            .and(crate::ConditionExpr::SourceHasCounterAtLeast {
                counter_type: crate::object::CounterType::Time,
                count: 1,
                surface: Default::default(),
            });
        let mut countdown = crate::ability::Ability::triggered(
            crate::triggers::Trigger::beginning_of_end_step(crate::target::PlayerFilter::You),
            vec![crate::effect::Effect::remove_counters(
                crate::object::CounterType::Time,
                1,
                crate::target::ChooseSpec::Source,
            )],
        );
        if let crate::ability::AbilityKind::Triggered(triggered) = &mut countdown.kind {
            triggered.intervening_if = Some(active.clone());
        }
        self.alternative_cast(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                "Impending",
                Some(cost),
                vec![],
            ),
        )
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::Time,
                time.into(),
            )
            .with_condition(paid),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::remove_card_types(
                crate::target::ObjectFilter::source(),
                vec![CardType::Creature],
                Some(active),
            ),
        ))
        .with_ability(countdown)
    }

    pub fn vanishing(self, amount: u32) -> Self {
        let mut builder = self;
        if amount > 0 {
            builder = builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::enters_with_counters_value(
                    crate::object::CounterType::Time,
                    amount.into(),
                ),
            ));
        }

        builder
            .with_ability({
                let mut ability = crate::ability::Ability::triggered(
                    crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
                    vec![crate::effect::Effect::remove_counters(
                        crate::object::CounterType::Time,
                        1,
                        crate::target::ChooseSpec::Source,
                    )],
                );
                if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind {
                    triggered.intervening_if =
                        Some(crate::effect::Condition::SourceHasCounterAtLeast {
                            counter_type: crate::object::CounterType::Time,
                            count: 1,
                            surface: Default::default(),
                        });
                }
                ability
            })
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(
                        crate::target::ObjectFilter::source(),
                    )
                    .counter_type(crate::object::CounterType::Time)
                    .last(),
                ),
                vec![crate::effect::Effect::sacrifice_source()],
            ))
    }

    pub fn modular(self, amount: u32) -> Self {
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = crate::tag::CompilerReferenceTag::ModularTriggeringObject.bind();
        // CR 702.43a: count the +1/+1 counters on the permanent as it last
        // existed on the battlefield. The graveyard card is a new object with
        // no counters (CR 400.7), so read the tagged dies-event snapshot.
        let transfer_count = crate::effect::Value::CountersOn(
            Box::new(crate::target::ChooseSpec::Tagged(trigger_tag.key.clone())),
            Some(crate::object::CounterType::PlusOnePlusOne),
        );

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::may(vec![crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )]),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn modular_sunburst(self) -> Self {
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = crate::tag::CompilerReferenceTag::ModularTriggeringObject.bind();
        // CR 702.43a: count the +1/+1 counters on the permanent as it last
        // existed on the battlefield. The graveyard card is a new object with
        // no counters (CR 400.7), so read the tagged dies-event snapshot.
        let transfer_count = crate::effect::Value::CountersOn(
            Box::new(crate::target::ChooseSpec::Tagged(trigger_tag.key.clone())),
            Some(crate::object::CounterType::PlusOnePlusOne),
        );

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::may(vec![crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )]),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn graft(self, amount: u32) -> Self {
        let entered_tag = crate::tag::CompilerReferenceTag::GraftEnteredCreature.bind();

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::enters_battlefield(
                    crate::target::ObjectFilter::creature().other(),
                    None,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(entered_tag.clone()),
                    crate::effect::Effect::may(vec![crate::effect::Effect::new(
                        crate::effects::MoveCountersEffect::new(
                            crate::object::CounterType::PlusOnePlusOne,
                            1,
                            crate::target::ChooseSpec::Source,
                            crate::target::ChooseSpec::Tagged(entered_tag.key.clone()),
                        ),
                    )]),
                ]),
                choices: vec![],
                // CR 702.58a: "if this permanent has a +1/+1 counter on it".
                intervening_if: Some(crate::effect::Condition::SourceHasCounterAtLeast {
                    counter_type: crate::object::CounterType::PlusOnePlusOne,
                    count: 1,
                    surface: Default::default(),
                }),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn rampage(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker(format!("rampage {amount}")),
        ))
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_becomes_blocked(),
            vec![crate::effect::Effect::pump(
                crate::effect::Value::EventValue(
                    crate::effect::EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    },
                ),
                crate::effect::Value::EventValue(
                    crate::effect::EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    },
                ),
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        ))
    }

    pub fn bushido(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::either(
                crate::triggers::Trigger::this_blocks(),
                crate::triggers::Trigger::this_becomes_blocked(),
            ),
            vec![crate::effect::Effect::pump(
                amount as i32,
                amount as i32,
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        ))
    }

    pub fn frenzy(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks_and_isnt_blocked(),
            vec![crate::effect::Effect::pump(
                amount as i32,
                0,
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        ))
    }

    pub fn protection_from(self, colors: ColorSet) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_card_type(self, card_type: CardType) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_filter(self, filter: ObjectFilter) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(filter),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_subtype(self, subtype: Subtype) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(
                crate::target::ObjectFilter::default().with_subtype(subtype),
            ),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn devoid(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::make_colorless(
                    crate::target::ObjectFilter::source(),
                ),
            )
            .in_zones(vec![
                crate::zone::Zone::Battlefield,
                crate::zone::Zone::Stack,
                crate::zone::Zone::Hand,
                crate::zone::Zone::Library,
                crate::zone::Zone::Graveyard,
                crate::zone::Zone::Exile,
                crate::zone::Zone::Command,
            ]),
        )
    }

    pub fn annihilator(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::sacrifice_player(
                        crate::target::ObjectFilter::permanent(),
                        amount as i32,
                        crate::target::PlayerFilter::Defending,
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn bloodthirst(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::bloodthirst(amount),
        ))
    }

    pub fn undying(self) -> Self {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(ironsmith_compiler_semantic::tag::declared_key(trigger_tag));

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                        filter, return_tag,
                    )),
                    crate::effect::Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            crate::target::ChooseSpec::Tagged(
                                ironsmith_compiler_semantic::tag::declared_key(return_tag).into(),
                            ),
                            crate::zone::Zone::Battlefield,
                            true,
                        )
                        .under_owner_control(),
                    )
                    .tag(ironsmith_compiler_semantic::tag::declared_key(returned_tag)),
                    crate::effect::Effect::for_each_tagged(
                        ironsmith_compiler_semantic::tag::declared_key(returned_tag),
                        vec![crate::effect::Effect::put_counters(
                            crate::object::CounterType::PlusOnePlusOne,
                            1,
                            crate::target::ChooseSpec::Iterated,
                        )],
                    ),
                ]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::TriggeringObjectHadCounters {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    pub fn persist(self) -> Self {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(ironsmith_compiler_semantic::tag::declared_key(trigger_tag));

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                        filter, return_tag,
                    )),
                    crate::effect::Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            crate::target::ChooseSpec::Tagged(
                                ironsmith_compiler_semantic::tag::declared_key(return_tag).into(),
                            ),
                            crate::zone::Zone::Battlefield,
                            true,
                        )
                        .under_owner_control(),
                    )
                    .tag(ironsmith_compiler_semantic::tag::declared_key(returned_tag)),
                    crate::effect::Effect::for_each_tagged(
                        ironsmith_compiler_semantic::tag::declared_key(returned_tag),
                        vec![crate::effect::Effect::put_counters(
                            crate::object::CounterType::MinusOneMinusOne,
                            1,
                            crate::target::ChooseSpec::Iterated,
                        )],
                    ),
                ]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::TriggeringObjectHadCounters {
                        counter_type: crate::object::CounterType::MinusOneMinusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    fn fabricate_servo_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Servo")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Servo])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn afterlife_spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .color_indicator(ColorSet::WHITE.union(ColorSet::BLACK))
            .power_toughness(PowerToughness::fixed(1, 1))
            .flying()
            .build()
    }

    fn for_mirrodin_rebel_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Rebel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rebel])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn living_weapon_germ_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Phyrexian Germ")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Germ])
            .color_indicator(ColorSet::BLACK)
            .power_toughness(PowerToughness::fixed(0, 0))
            .build()
    }

    fn mobilize_warrior_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Warrior])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    pub fn build(self) -> CardDefinition {
        let refers_to_ante = self
            .card_builder
            .oracle_text_ref()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case("ante"));
        CardDefinition {
            card: self.card_builder.build(),
            canonical_text: String::new(),
            ability_labels: Vec::new(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            has_fuse: self.has_fuse,
            optional_costs: self.optional_costs,
            additional_cost: self.additional_cost,
            refers_to_ante,
        }
    }
}
