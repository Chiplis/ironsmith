pub(crate) use crate::ability::ActivationTiming;
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
pub use crate::cards::CardDefinition;
use crate::color::ColorSet;
pub use crate::cost::OptionalCost;
use crate::cost::TotalCost;
pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub(crate) use crate::effect::EffectPredicate;
pub use crate::effect::{ChoiceCount, EventValueSpec, Value};
use crate::mana::ManaCost;
pub use crate::model::reference::RefState;
pub use crate::model::{
    AdditionalCostChoiceOptionAst, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use crate::object::AuraAttachmentFilter;
pub use crate::payload::{IfResultPredicate, KeywordAction};
use crate::resolution::ResolutionProgram;
pub(crate) use crate::runtime_backend::semantic::{
    GiftTimingAst, LineAst, ParsedAbility, ParsedCardItem, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
};
pub(crate) use crate::runtime_backend::util::SubjectAst;
pub(crate) use crate::runtime_backend::{
    CarryContext, EffectLoweringContext, IdGenContext, LineInfo, LoweringFrame, MetadataLine,
    NormalizedLine, TokenCopyFollowup, Verb, parse_object_filter_lexed,
};
pub(crate) use crate::runtime_backend::{
    PermissionClauseSpec, PermissionLifetime, ReferenceEnv, ReferenceImports,
};
#[cfg(test)]
pub(crate) use crate::runtime_backend::{
    find_verb, parse_effect_sentence_lexed, parse_shared_color_target_fanout_sentence,
};
pub(crate) use crate::tag::TagKey;
pub use crate::target::{ObjectFilter, PlayerFilter};
pub use crate::types::CardType;
use crate::types::{Subtype, Supertype};
pub use ironsmith_core::CardId;
use ironsmith_core::CostComponent as _;

#[cfg(test)]
pub(crate) mod document_parser {
    pub(crate) use crate::runtime_backend::cst::KeywordLineKindCst;
}

#[derive(Debug, Clone)]
pub(crate) enum GrantedAbilityAst {
    KeywordAction(KeywordAction),
    MustAttack,
    MustBlock,
    CanAttackAsThoughNoDefender,
    CanBlockAdditionalCreatureEachCombat {
        additional: usize,
    },
    ParsedObjectAbility {
        ability: ParsedAbility,
        display: String,
    },
}

impl From<KeywordAction> for GrantedAbilityAst {
    fn from(action: KeywordAction) -> Self {
        Self::KeywordAction(action)
    }
}

pub(crate) fn replace_whole_word_case_insensitive(
    input: &str,
    needle: &str,
    replacement: &str,
) -> String {
    input.replace(needle, replacement)
}

fn cost_to_payment_effect(cost: &crate::costs::Cost) -> Option<crate::effect::Effect> {
    match cost {
        crate::costs::Cost::Mana(mana_cost) => Some(crate::effect::Effect::new(
            crate::effects::PayManaEffect::new(
                mana_cost.clone(),
                crate::target::ChooseSpec::SourceController,
            ),
        )),
        crate::costs::Cost::Discard { count, card_types } => {
            let filter = if card_types.is_empty() {
                None
            } else {
                let mut filter = crate::target::ObjectFilter::default();
                filter.card_types = card_types.clone();
                filter.zone = Some(crate::zone::Zone::Hand);
                Some(filter)
            };
            Some(crate::effect::Effect::discard_player_filtered(
                *count,
                crate::target::PlayerFilter::You,
                false,
                filter,
            ))
        }
        crate::costs::Cost::DiscardHand => Some(crate::effect::Effect::discard_hand()),
        crate::costs::Cost::Effect(effect) => Some(effect.clone()),
        _ => None,
    }
}

fn total_cost_to_payment_effects(total_cost: &TotalCost) -> Vec<crate::effect::Effect> {
    total_cost
        .costs()
        .iter()
        .map(|cost| {
            cost_to_payment_effect(cost)
                .unwrap_or_else(|| panic!("unsupported echo cost component: {}", cost.display()))
        })
        .collect()
}

pub(crate) use crate::runtime_backend::lexer::OwnedLexToken;

pub(crate) use crate::runtime_backend::ast::{
    EffectAst, PredicateAst, StaticAbilityAst, TriggerSpec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsteadSemantics {
    SelfReplacement,
    FutureReplacement,
    NonReplacement,
}

#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    pub(crate) card_builder: CardBuilder,
    pub(crate) abilities: Vec<crate::ability::Ability>,
    pub(crate) spell_effect: Option<ResolutionProgram>,
    pub(crate) alternative_casts: Vec<crate::alternative_cast::AlternativeCastingMethod>,
    pub(crate) optional_costs: Vec<OptionalCost>,
    pub(crate) max_saga_chapter: Option<u32>,
    pub(crate) additional_cost: TotalCost,
    pub(crate) aura_attach_filter: Option<AuraAttachmentFilter>,
    pub(crate) has_fuse: bool,
}

impl CardDefinitionBuilder {
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            additional_cost: TotalCost::free(),
            aura_attach_filter: None,
            has_fuse: false,
        }
    }

    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
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

    pub fn saga(mut self, max_chapters: u32) -> Self {
        self.max_saga_chapter = Some(max_chapters);
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
        self.aura_attach_filter = Some(filter);
        self
    }

    pub fn apply_keyword_action(self, action: KeywordAction) -> Self {
        match action {
            KeywordAction::Flying => self.flying(),
            KeywordAction::Defender => self.defender(),
            KeywordAction::Vigilance => self.vigilance(),
            KeywordAction::Prowess => self.prowess(),
            KeywordAction::Trample => self.trample(),
            KeywordAction::Lifelink => self.lifelink(),
            KeywordAction::Deathtouch => self.deathtouch(),
            KeywordAction::Haste => self.haste(),
            KeywordAction::Menace => self.menace(),
            KeywordAction::Reach => self.reach(),
            KeywordAction::Hexproof => self.hexproof(),
            KeywordAction::Indestructible => self.indestructible(),
            KeywordAction::Toxic(amount) => self.toxic(amount),
            KeywordAction::FirstStrike => self.first_strike(),
            KeywordAction::DoubleStrike => self.double_strike(),
            KeywordAction::Riot => self.riot(),
            KeywordAction::Outlast(cost) => self.outlast(cost),
            KeywordAction::Unearth(cost) => self.unearth(cost),
            KeywordAction::Vanishing(amount) => self.vanishing(amount),
            KeywordAction::Bloodthirst(amount) => self.bloodthirst(amount),
            KeywordAction::Ninjutsu(cost) => self.ninjutsu(cost),
            KeywordAction::Ward(amount) => self.ward_generic(amount),
            KeywordAction::Undying => self.undying(),
            KeywordAction::Persist => self.persist(),
            KeywordAction::Renown(amount) => self.renown(amount),
            KeywordAction::Cipher => self.cipher(),
            KeywordAction::Suspend { time, cost } => self.suspend(time, cost),
            KeywordAction::Overload(cost) => self.overload(cost),
            KeywordAction::Echo { total_cost, text } => self.echo(total_cost, text),
            KeywordAction::Enlist => self.enlist(),
            other if other.lowers_to_static_ability() => self.with_ability(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::keyword_marker(other.display_text()),
                )
                .with_text(&other.display_text()),
            ),
            other => self.with_ability(
                crate::ability::Ability::triggered(
                    crate::triggers::Trigger::custom("compiler-keyword", other.display_text()),
                    crate::resolution::ResolutionProgram::default(),
                )
                .with_text(&other.display_text()),
            ),
        }
    }

    pub fn apply_metadata(
        mut self,
        meta: impl Into<crate::front_end::MetadataLine>,
    ) -> Result<Self, CardTextError> {
        let meta = meta.into();
        match meta {
            crate::front_end::MetadataLine::ManaCost(raw) => {
                let cost = crate::runtime_backend::parse_scryfall_mana_cost(&raw)?;
                if !cost.is_empty() {
                    self.card_builder = self.card_builder.mana_cost(cost);
                }
            }
            crate::front_end::MetadataLine::TypeLine(raw) => {
                let (supertypes, card_types, subtypes) =
                    crate::runtime_backend::parse_type_line(&raw)?;
                if !supertypes.is_empty() {
                    self.card_builder = self.card_builder.supertypes(supertypes);
                }
                if !card_types.is_empty() {
                    self.card_builder = self.card_builder.card_types(card_types);
                }
                if !subtypes.is_empty() {
                    self.card_builder = self.card_builder.subtypes(subtypes);
                }
            }
            crate::front_end::MetadataLine::PowerToughness(raw) => {
                if let Some(pt) = crate::runtime_backend::parse_power_toughness(&raw) {
                    self.card_builder = self.card_builder.power_toughness(pt);
                }
            }
            crate::front_end::MetadataLine::Loyalty(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.loyalty(value);
                }
            }
            crate::front_end::MetadataLine::Defense(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.defense(value);
                }
            }
        }
        Ok(self)
    }

    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        crate::runtime_backend::parse_card_text(self, text)
    }

    pub fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        crate::runtime_backend::parse_card_text_allow_unsupported(self, text)
    }

    pub fn flying(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::flying(),
            )
            .with_text("Flying"),
        )
    }

    pub fn defender(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::defender(),
            )
            .with_text("Defender"),
        )
    }

    pub fn vigilance(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::vigilance(),
            )
            .with_text("Vigilance"),
        )
    }

    pub fn prowess(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::prowess(),
            )
            .with_text("Prowess"),
        )
    }

    pub fn trample(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::trample(),
            )
            .with_text("Trample"),
        )
    }

    pub fn lifelink(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::lifelink(),
            )
            .with_text("Lifelink"),
        )
    }

    pub fn deathtouch(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::deathtouch(),
            )
            .with_text("Deathtouch"),
        )
    }

    pub fn haste(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        ).with_text("Haste"))
    }

    pub fn menace(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::menace(),
            )
            .with_text("Menace"),
        )
    }

    pub fn reach(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::reach(),
        ).with_text("Reach"))
    }

    pub fn hexproof(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::hexproof(),
            )
            .with_text("Hexproof"),
        )
    }

    pub fn indestructible(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::indestructible(),
            )
            .with_text("Indestructible"),
        )
    }

    pub fn toxic(self, amount: u32) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::toxic(
                amount,
            ))
            .with_text(&format!("Toxic {amount}")),
        )
    }

    pub fn ward_generic(self, amount: u32) -> Self {
        let mana = ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(amount as u8)]);
        self.with_ability(
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::ward(
                TotalCost::mana(mana),
            ))
            .with_text(&format!("Ward {{{amount}}}")),
        )
    }

    pub fn first_strike(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::first_strike(),
            )
            .with_text("First strike"),
        )
    }

    pub fn double_strike(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::double_strike(),
            )
            .with_text("Double strike"),
        )
    }

    pub fn enlist(self) -> Self {
        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::this_attacks(),
                crate::resolution::ResolutionProgram::default(),
            )
            .with_text("Enlist"),
        )
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
                "This creature gains haste until end of turn",
                vec![crate::effect::Effect::grant_object_ability_to_source(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::haste(),
                    )
                    .with_text("Haste"),
                )],
            ),
        ];

        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::this_enters_battlefield(),
                vec![crate::effect::Effect::choose_one(modes)],
            )
            .with_text("Riot"),
        )
    }

    pub fn outlast(self, cost: ManaCost) -> Self {
        let text = format!("Outlast {}", cost.to_oracle());
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::tap(),
        ]);

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                )],
                crate::ability::ActivationTiming::SorcerySpeed,
            )
            .with_text(&text),
        )
    }

    pub fn unearth(self, cost: ManaCost) -> Self {
        let text = format!("Unearth {}", cost.to_oracle());
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::unearth()],
                crate::ability::ActivationTiming::SorcerySpeed,
            )
            .in_zones(vec![crate::zone::Zone::Graveyard])
            .with_text(&text),
        )
    }

    pub fn ninjutsu(self, cost: ManaCost) -> Self {
        let text = format!("Ninjutsu {}", cost.to_oracle());
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
            .in_zones(vec![crate::zone::Zone::Hand])
            .with_text(&text),
        )
    }

    pub fn renown(self, amount: u32) -> Self {
        let text = format!("Renown {amount}");
        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::this_deals_combat_damage_to_player(),
                vec![crate::effect::Effect::renown_source(amount)],
            )
            .with_text(&text),
        )
    }

    pub fn cipher(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::keyword_marker("Cipher"),
            )
            .with_text("Cipher"),
        )
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
                }),
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
            text: None,
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
                            .require_exile(),
                    ),
                ])]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasNoCounter(
                    crate::object::CounterType::Time,
                )),
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
            text: None,
        })
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

    pub fn echo(self, total_cost: TotalCost, text: String) -> Self {
        let payment_effects = total_cost_to_payment_effects(&total_cost);

        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::enters_with_counters_value(
                    crate::object::CounterType::Echo,
                    1.into(),
                ),
            )
            .with_text(&text),
        )
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::remove_counters(
                            crate::object::CounterType::Echo,
                            1,
                            crate::target::ChooseSpec::Source,
                        ),
                    ),
                    crate::effect::Effect::if_then(
                        crate::effect::EffectId(0),
                        crate::effect::EffectPredicate::Happened,
                        vec![crate::effect::Effect::unless_action(
                            vec![crate::effect::Effect::sacrifice_source()],
                            payment_effects,
                            crate::target::PlayerFilter::You,
                        )],
                    ),
                ]
                .into(),
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
            text: None,
        })
    }

    pub fn vanishing(self, amount: u32) -> Self {
        let text = if amount == 0 {
            "Vanishing".to_string()
        } else {
            format!("Vanishing {amount}")
        };

        let mut builder = self;
        if amount > 0 {
            builder = builder.with_ability(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::enters_with_counters_value(
                        crate::object::CounterType::Time,
                        amount.into(),
                    ),
                )
                .with_text(&text),
            );
        }

        builder
            .with_ability(
                crate::ability::Ability::triggered(
                    crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
                    vec![crate::effect::Effect::remove_counters(
                        crate::object::CounterType::Time,
                        1,
                        crate::target::ChooseSpec::Source,
                    )],
                )
                .with_text(
                    "At the beginning of your upkeep, remove a time counter from this permanent.",
                ),
            )
            .with_ability(
                crate::ability::Ability::triggered(
                    crate::triggers::Trigger::custom(
                        "vanishing-last-time-counter-removed",
                        "when the last time counter is removed".to_string(),
                    ),
                    vec![crate::effect::Effect::sacrifice_source()],
                )
                .with_text(
                    "When the last time counter is removed from this permanent, sacrifice it.",
                ),
            )
    }

    pub fn bloodthirst(self, amount: u32) -> Self {
        let text = format!("Bloodthirst {amount}");
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::bloodthirst(amount),
            )
            .with_text(&text),
        )
    }

    pub fn undying(self) -> Self {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::choose_objects(
                        filter,
                        1,
                        crate::target::PlayerFilter::You,
                        return_tag,
                    ),
                    crate::effect::Effect::move_to_zone(
                        crate::target::ChooseSpec::Tagged(return_tag.into()),
                        crate::zone::Zone::Battlefield,
                        true,
                    )
                    .tag(returned_tag),
                    crate::effect::Effect::for_each_tagged(
                        returned_tag,
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
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Undying".to_string()),
        })
    }

    pub fn persist(self) -> Self {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::choose_objects(
                        filter,
                        1,
                        crate::target::PlayerFilter::You,
                        return_tag,
                    ),
                    crate::effect::Effect::move_to_zone(
                        crate::target::ChooseSpec::Tagged(return_tag.into()),
                        crate::zone::Zone::Battlefield,
                        true,
                    )
                    .tag(returned_tag),
                    crate::effect::Effect::for_each_tagged(
                        returned_tag,
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
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Persist".to_string()),
        })
    }

    pub fn build(self) -> CardDefinition {
        CardDefinition {
            card: self.card_builder.build(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            has_fuse: self.has_fuse,
            optional_costs: self.optional_costs,
            max_saga_chapter: self.max_saga_chapter,
            additional_cost: self.additional_cost,
        }
    }
}

pub const IT_TAG: &str = crate::host::IT_TAG;
