use super::{StaticAbility, StaticAbilityId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAbilityModelConversionError {
    pub detail: String,
}

impl std::fmt::Display for StaticAbilityModelConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for StaticAbilityModelConversionError {}

impl StaticAbility {
    pub fn from_compiler_model_parts(
        id: Option<StaticAbilityId>,
        label: String,
    ) -> Result<Self, StaticAbilityModelConversionError> {
        Ok(match id {
            Some(StaticAbilityId::Flying) => Self::flying(),
            Some(StaticAbilityId::FirstStrike) => Self::first_strike(),
            Some(StaticAbilityId::DoubleStrike) => Self::double_strike(),
            Some(StaticAbilityId::Deathtouch) => Self::deathtouch(),
            Some(StaticAbilityId::Defender) => Self::defender(),
            Some(StaticAbilityId::Flash) => Self::flash(),
            Some(StaticAbilityId::Haste) => Self::haste(),
            Some(StaticAbilityId::Hexproof) => Self::hexproof(),
            Some(StaticAbilityId::Indestructible) => Self::indestructible(),
            Some(StaticAbilityId::Intimidate) => Self::intimidate(),
            Some(StaticAbilityId::Lifelink) => Self::lifelink(),
            Some(StaticAbilityId::Menace) => Self::menace(),
            Some(StaticAbilityId::Reach) => Self::reach(),
            Some(StaticAbilityId::Shroud) => Self::shroud(),
            Some(StaticAbilityId::Trample) => Self::trample(),
            Some(StaticAbilityId::Vigilance) => Self::vigilance(),
            Some(StaticAbilityId::Fear) => Self::fear(),
            Some(StaticAbilityId::Skulk) => Self::skulk(),
            Some(StaticAbilityId::Prowess) => Self::prowess(),
            Some(StaticAbilityId::Flanking) => Self::flanking(),
            Some(StaticAbilityId::UmbraArmor) => Self::umbra_armor(),
            Some(StaticAbilityId::Phasing) => Self::phasing(),
            Some(StaticAbilityId::Wither) => Self::wither(),
            Some(StaticAbilityId::Infect) => Self::infect(),
            Some(StaticAbilityId::Changeling) => Self::changeling(),
            Some(StaticAbilityId::Partner) => Self::partner(),
            Some(StaticAbilityId::DoctorsCompanion) => Self::doctors_companion(),
            Some(StaticAbilityId::Assist) => Self::assist(),
            Some(StaticAbilityId::SplitSecond) => Self::split_second(),
            Some(StaticAbilityId::Rebound) => Self::rebound(),
            Some(StaticAbilityId::Cascade) => Self::cascade(),
            Some(StaticAbilityId::Unleash) => Self::unleash(),
            Some(StaticAbilityId::Unblockable) => Self::unblockable(),
            Some(StaticAbilityId::CantBlock) => Self::cant_block(),
            Some(StaticAbilityId::CantAttack) => Self::cant_attack(),
            Some(StaticAbilityId::CantAttackItsOwner) => Self::cant_attack_its_owner(),
            Some(StaticAbilityId::CantBeCountered) => Self::cant_be_countered_ability(),
            Some(StaticAbilityId::CanBlockFlying) => Self::can_block_flying(),
            Some(StaticAbilityId::CanBlockOnlyFlying) => Self::can_block_only_flying(),
            Some(StaticAbilityId::MustAttack) => Self::must_attack(),
            Some(StaticAbilityId::MustBlock) => Self::must_block(),
            Some(StaticAbilityId::Shadow) => Self::shadow(),
            Some(StaticAbilityId::Horsemanship) => Self::horsemanship(),
            Some(StaticAbilityId::FlyingRestriction) => Self::flying_restriction(),
            Some(StaticAbilityId::FlyingOnlyRestriction) => Self::flying_only_restriction(),
            Some(StaticAbilityId::CantBeBlockedByLowerPowerThanSource) => {
                Self::cant_be_blocked_by_lower_power_than_source()
            }
            Some(StaticAbilityId::MayAssignDamageAsUnblocked) => {
                Self::may_assign_damage_as_unblocked()
            }
            Some(StaticAbilityId::DoesntUntap) => Self::doesnt_untap(),
            Some(StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness) => {
                Self::creatures_you_control_assign_combat_damage_using_toughness()
            }
            Some(StaticAbilityId::BlackManaMayBePaidWithLife) => {
                Self::krrik_black_mana_may_be_paid_with_life()
            }
            Some(StaticAbilityId::CantPayLifeOrSacrificeNonlandForCastOrActivate) => {
                Self::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate()
            }
            Some(StaticAbilityId::PreventAllDamageDealtToCreatures) => {
                Self::prevent_all_damage_dealt_to_creatures()
            }
            Some(StaticAbilityId::PreventAllCombatDamageToSelf) => {
                Self::prevent_all_combat_damage_to_self()
            }
            Some(StaticAbilityId::PreventAllDamageToSelfByCreatures) => {
                Self::prevent_all_damage_to_self_by_creatures()
            }
            Some(StaticAbilityId::ShuffleIntoLibraryFromGraveyard) => {
                Self::shuffle_into_library_from_graveyard()
            }
            Some(StaticAbilityId::AllPermanentsEnterTapped) => Self::permanents_enter_tapped(),
            Some(StaticAbilityId::PlayersCantCycle) => Self::players_cant_cycle(),
            Some(StaticAbilityId::PlayersSkipUpkeep) => Self::players_skip_upkeep(),
            Some(StaticAbilityId::AffinityForArtifacts) => Self::affinity_for_artifacts(),
            Some(StaticAbilityId::Delve) => Self::delve(),
            Some(StaticAbilityId::Convoke) => Self::convoke(),
            Some(StaticAbilityId::Improvise) => Self::improvise(),
            Some(StaticAbilityId::BloodMoon) => Self::blood_moon(),
            Some(StaticAbilityId::TophFirstMetalbender) => {
                Self::new(crate::static_abilities::TophFirstMetalbender)
            }
            Some(StaticAbilityId::NoMaximumHandSize) => Self::no_maximum_hand_size(),
            Some(StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger) => {
                Self::creatures_entering_dont_cause_abilities_to_trigger()
            }
            Some(StaticAbilityId::LibraryOfLengDiscardReplacement) => {
                Self::library_of_leng_discard_replacement()
            }
            Some(StaticAbilityId::DrawReplacementExileTopFaceDown) => {
                Self::draw_replacement_exile_top_face_down()
            }
            Some(StaticAbilityId::LookAtTopCardOfLibrary) => Self::look_at_top_card_of_library(),
            Some(StaticAbilityId::EntersTapped) => Self::enters_tapped_ability(),
            Some(StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands) => {
                Self::enters_tapped_unless_control_two_or_more_other_lands()
            }
            Some(StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands) => {
                Self::enters_tapped_unless_control_two_or_fewer_other_lands()
            }
            Some(StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands) => {
                Self::enters_tapped_unless_control_two_or_more_basic_lands()
            }
            Some(StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife) => {
                Self::enters_tapped_unless_a_player_has_13_or_less_life()
            }
            Some(StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents) => {
                Self::enters_tapped_unless_two_or_more_opponents()
            }
            Some(StaticAbilityId::CanBeCommander) => Self::can_be_commander(),
            Some(StaticAbilityId::KeywordText) => Self::keyword_text(label),
            Some(StaticAbilityId::KeywordMarker) => Self::keyword_marker(label),
            Some(StaticAbilityId::KeywordFallbackText) => Self::keyword_fallback_text(label),
            Some(StaticAbilityId::RuleFallbackText) => Self::rule_fallback_text(label),
            Some(StaticAbilityId::UnsupportedParserLine) => {
                Self::unsupported_parser_line(label, "compiler unsupported parser line")
            }
            Some(id) => {
                return Err(StaticAbilityModelConversionError {
                    detail: format!("id={id:?}, label={label}"),
                });
            }
            None => {
                return Err(StaticAbilityModelConversionError {
                    detail: format!("label={label}"),
                });
            }
        })
    }
}
