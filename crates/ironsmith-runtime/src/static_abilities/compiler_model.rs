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
    fn parse_draw_replacement_exile_top_and_play_count(label: &str) -> Option<u32> {
        let prefix = "draw replacement exile top ";
        let suffix = " and play";
        label
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix))
            .and_then(|count| count.parse::<u32>().ok())
    }

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
            Some(StaticAbilityId::Banding) => Self::banding(),
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
            Some(StaticAbilityId::Daybound) => Self::daybound(),
            Some(StaticAbilityId::Nightbound) => Self::nightbound(),
            Some(StaticAbilityId::DayNightStartsDayAsEnters) => {
                Self::day_night_starts_day_as_enters()
            }
            Some(StaticAbilityId::Partner) => {
                if label.trim().eq_ignore_ascii_case("partner") {
                    Self::partner()
                } else {
                    Self::partner_variant(label)
                }
            }
            Some(StaticAbilityId::PartnerWith) => Self::partner_with(label),
            Some(StaticAbilityId::StartYourEngines) => Self::start_your_engines(),
            Some(StaticAbilityId::DoctorsCompanion) => Self::doctors_companion(),
            Some(StaticAbilityId::Assist) => Self::assist(),
            Some(StaticAbilityId::SplitSecond) => Self::split_second(),
            Some(StaticAbilityId::Rebound) => Self::rebound(),
            Some(StaticAbilityId::Cascade) => Self::cascade(),
            Some(StaticAbilityId::ReadAhead) => Self::read_ahead(),
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
            Some(StaticAbilityId::CanAttackAsThoughNoDefender) => {
                Self::can_attack_as_though_no_defender()
            }
            Some(StaticAbilityId::MayAssignDamageAsUnblocked) => {
                Self::may_assign_damage_as_unblocked()
            }
            Some(StaticAbilityId::DoesntUntap) => Self::doesnt_untap(),
            Some(StaticAbilityId::BoastTwiceEachTurn) => Self::boast_twice_each_turn(),
            Some(StaticAbilityId::EquipAbilitiesAnyTime) => Self::equip_abilities_any_time(),
            Some(StaticAbilityId::VoteAdditionalTimeWhileVoting) => {
                Self::vote_additional_time_while_voting()
            }
            Some(StaticAbilityId::VoteAdditionalVoteWhileVoting) => {
                Self::vote_additional_vote_while_voting()
            }
            Some(StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn) => {
                Self::cant_attack_unless_controller_cast_creature_spell_this_turn()
            }
            Some(StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn) => {
                Self::cant_attack_unless_controller_cast_noncreature_spell_this_turn()
            }
            Some(StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness) => {
                Self::creatures_you_control_assign_combat_damage_using_toughness()
            }
            Some(StaticAbilityId::CreaturesAssignCombatDamageUsingToughness) => {
                Self::creatures_assign_combat_damage_using_toughness()
            }
            Some(StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness) => {
                Self::this_creature_assigns_combat_damage_using_toughness()
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
            Some(StaticAbilityId::PreventAllDamageDealtByThisPermanent) => {
                Self::prevent_all_damage_dealt_by_this_permanent()
            }
            Some(StaticAbilityId::PreventAllCombatDamageDealtByThisPermanent) => {
                Self::prevent_all_combat_damage_dealt_by_this_permanent()
            }
            Some(StaticAbilityId::PreventAllDamageToSelf) => Self::prevent_all_damage_to_self(),
            Some(StaticAbilityId::PreventAllCombatDamageToSelf) => {
                Self::prevent_all_combat_damage_to_self()
            }
            Some(StaticAbilityId::PreventAllDamageToSelfByCreatures) => {
                Self::prevent_all_damage_to_self_by_creatures()
            }
            Some(StaticAbilityId::PreventAllNoncombatDamageToOtherCreaturesYouControl) => {
                Self::prevent_all_noncombat_damage_to_other_creatures_you_control()
            }
            Some(StaticAbilityId::RedirectDamageToSource) => {
                Self::redirect_damage_from_you_and_other_permanents_to_source()
            }
            Some(StaticAbilityId::DamageNotRemovedDuringCleanup) => {
                Self::damage_not_removed_during_cleanup()
            }
            Some(StaticAbilityId::PlayersCantGainLife) => Self::players_cant_gain_life(),
            Some(StaticAbilityId::PlayersCantSearch) => Self::players_cant_search(),
            Some(StaticAbilityId::DamageCantBePrevented) => Self::damage_cant_be_prevented(),
            Some(StaticAbilityId::YouCantLoseGame) => Self::you_cant_lose_game(),
            Some(StaticAbilityId::OpponentsCantWinGame) => Self::opponents_cant_win_game(),
            Some(StaticAbilityId::YourLifeTotalCantChange) => Self::your_life_total_cant_change(),
            Some(StaticAbilityId::OpponentsCantCastSpells) => Self::opponents_cant_cast_spells(),
            Some(StaticAbilityId::OpponentsCantDrawExtraCards) => {
                Self::opponents_cant_draw_extra_cards()
            }
            Some(StaticAbilityId::ControlOpponentsWhileSearchingLibraries) => {
                Self::control_opponents_while_searching_libraries()
            }
            Some(StaticAbilityId::OpponentSearchExileFoundCards) => {
                Self::opponent_search_exile_found_cards()
            }
            Some(StaticAbilityId::CastThisCardFromLibraryWhileSearching) => {
                Self::cast_this_card_from_library_while_searching()
            }
            Some(StaticAbilityId::CantHaveCountersPlaced) => Self::cant_have_counters_placed(),
            Some(StaticAbilityId::PermanentsCantBeSacrificed) => {
                Self::permanents_you_control_cant_be_sacrificed()
            }
            Some(StaticAbilityId::StartingLifeBonus) => Self::starting_life_bonus(0),
            Some(StaticAbilityId::BuybackCostReduction) => Self::buyback_cost_reduction(0),
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
            Some(StaticAbilityId::ChooseColorAsBecomesAttached) => {
                Self::choose_color_as_becomes_attached(label)
            }
            Some(StaticAbilityId::ChoosePowerToughnessAsEntersOrTurnsFaceUp) => {
                Self::rule_fallback_text(label)
            }
            Some(StaticAbilityId::NoMaximumHandSize) => Self::no_maximum_hand_size(),
            Some(StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger) => {
                Self::creatures_entering_dont_cause_abilities_to_trigger()
            }
            Some(StaticAbilityId::EffectDiscardToLibraryReplacement) => {
                Self::effect_discard_to_library_replacement()
            }
            Some(StaticAbilityId::DrawReplacementExileTopFaceDown) => {
                Self::draw_replacement_exile_top_face_down()
            }
            Some(StaticAbilityId::DrawReplacementExileTopAndPlay) => {
                Self::draw_replacement_exile_top_and_play(
                    Self::parse_draw_replacement_exile_top_and_play_count(&label).unwrap_or(2),
                )
            }
            Some(StaticAbilityId::DrawReplacementDouble) => Self::draw_replacement_double(),
            Some(StaticAbilityId::DrawReplacementSkipEmptyLibrary) => {
                Self::draw_replacement_skip_empty_library()
            }
            Some(StaticAbilityId::ExileWouldDieInstead) => {
                Self::exile_would_die_instead(crate::target::ObjectFilter::creature())
            }
            Some(StaticAbilityId::LookAtTopCardOfLibrary) => Self::look_at_top_card_of_library(),
            Some(StaticAbilityId::LookAtFaceDownCreaturesYouDontControl) => {
                Self::look_at_face_down_creatures_you_dont_control()
            }
            Some(StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries) => {
                Self::all_players_look_at_top_cards_of_libraries()
            }
            Some(StaticAbilityId::AllPlayersLookAtYourTopLibraryCard) => {
                Self::all_players_look_at_your_top_library_card()
            }
            Some(StaticAbilityId::OpponentsPlayWithHandsRevealed) => {
                Self::opponents_play_with_hands_revealed()
            }
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
            Some(StaticAbilityId::DraftRuleText) => Self::draft_rule_text(label),
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
