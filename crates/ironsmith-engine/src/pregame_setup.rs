//! Chosen outside-game setup cards. This does not use official companion state.
//!
//! A setup importer supplies the immutable original printed text alongside the
//! physical starting-deck ids before shuffling. Expanded or rendered programs
//! are deliberately not accepted as a substitute for that source evidence.
use std::collections::{HashMap, HashSet};
use crate::{GameState,ObjectId,PlayerId,Zone,CardType};
use crate::ability::AbilityKind;
use ironsmith_core::{PregameActionKind,PregameChosenCardSpec,PregameChosenCardAction,PregameDeckCondition};

#[derive(Debug,Clone)]
pub struct PrintedStartingDeckCard {
    pub object: ObjectId,
    pub original_printed_rules_text: String,
}
#[derive(Debug,Clone)]
struct FrozenDeckCard { types: Vec<CardType>, text: String }
#[derive(Debug,Clone)]
struct ChosenCard { source: ObjectId, spec: PregameChosenCardSpec, used: bool }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum Stage { Choosing, OpeningHand, Finished }
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum PregameSetupError {
    WrongStage, MissingPlayer, IncompleteStartingDeck, InvalidSource,
    MissingSetupAbility, ConditionNotFulfilled, AlreadyChosen, NotChosen,
    AlreadyUsed, InvalidOpeningHandCard, MovementFailed,
    SourceProgramFailed(String), SourceProgramPending,
}
/// Setup state is owned by the pregame coordinator and ends before turn one.
/// Each designation is independently limited to one chosen card per player;
/// ordinary per-source setup programs must not use this designation operation.
#[derive(Debug,Clone)]
pub struct PregameSetupSession {
    decks: HashMap<PlayerId,Vec<FrozenDeckCard>>,
    chosen: HashMap<(PlayerId,String),ChosenCard>,
    stage: Stage,
    source_programs_used: HashSet<(ObjectId,usize)>,
}
impl PregameSetupSession {
    /// Import the complete pre-shuffle library and designated commanders for every player. Reject missing,
    /// duplicate, wrong-owner and nonlibrary ids before retaining any facts.
    pub fn new(game:&GameState, evidence:Vec<(PlayerId,Vec<PrintedStartingDeckCard>)>)->Result<Self,PregameSetupError>{
        let mut decks=HashMap::new();
        for (player,cards) in evidence {
            let state=game.player(player).ok_or(PregameSetupError::MissingPlayer)?;
            let expected:HashSet<_>=state.library.iter().chain(state.commanders.iter()).copied().collect();
            let actual:HashSet<_>=cards.iter().map(|card|card.object).collect();
            if actual!=expected || actual.len()!=cards.len() || decks.contains_key(&player){return Err(PregameSetupError::IncompleteStartingDeck);}
            let facts=cards.into_iter().map(|card|{
                let object=game.object(card.object).ok_or(PregameSetupError::IncompleteStartingDeck)?;
                if object.owner!=player||!(object.zone==Zone::Library||(object.zone==Zone::Command&&state.commanders.contains(&card.object))){return Err(PregameSetupError::IncompleteStartingDeck);}
                Ok(FrozenDeckCard{types:object.card_types.to_vec(),text:card.original_printed_rules_text})
            }).collect::<Result<Vec<_>,_>>()?;
            decks.insert(player,facts);
        }
        Ok(Self{decks,chosen:HashMap::new(),stage:Stage::Choosing,source_programs_used:HashSet::new()})
    }
    pub fn choose(&mut self,game:&GameState,player:PlayerId,source:ObjectId,ability_index:usize)->Result<(),PregameSetupError>{
        if self.stage!=Stage::Choosing{return Err(PregameSetupError::WrongStage);}
        let object=game.object(source).ok_or(PregameSetupError::InvalidSource)?;
        if object.owner!=player||object.zone!=Zone::OutsideGame{return Err(PregameSetupError::InvalidSource);}
        let spec=match object.abilities.get(ability_index).map(|ability|&ability.kind){
            Some(AbilityKind::Static(ability))=>match ability.pregame_action_kind(){Some(PregameActionKind::ChosenOutsideGame(spec))=>spec,_=>return Err(PregameSetupError::MissingSetupAbility)},
            _=>return Err(PregameSetupError::MissingSetupAbility),
        };
        let designation=spec.designation.trim().to_ascii_lowercase();
        if designation.is_empty(){return Err(PregameSetupError::MissingSetupAbility);}
        let key=(player,designation);
        if self.chosen.contains_key(&key){return Err(PregameSetupError::AlreadyChosen);}
        let deck=self.decks.get(&player).ok_or(PregameSetupError::MissingPlayer)?;
        let fulfilled=match &spec.deck_condition {
            PregameDeckCondition::Unrestricted=>true,
            PregameDeckCondition::EveryCardPrintedTextContainsAny{exempt_types,phrases}=>!phrases.is_empty()&&deck.iter().all(|card|card.types.iter().any(|ty|exempt_types.contains(ty))||phrases.iter().any(|phrase|!phrase.is_empty()&&card.text.to_lowercase().contains(&phrase.to_lowercase()))),
        };
        if !fulfilled{return Err(PregameSetupError::ConditionNotFulfilled);}
        self.chosen.insert(key,ChosenCard{source,spec,used:false});
        Ok(())
    }
    /// Called after normal mulligans, before opening-hand setup actions.
    pub fn begin_opening_hand_actions(&mut self)->Result<(),PregameSetupError>{
        if self.stage!=Stage::Choosing{return Err(PregameSetupError::WrongStage);}
        self.stage=Stage::OpeningHand;Ok(())
    }
    /// A caller may decline by never invoking this optional operation. Both
    /// recipients are validated first, and only actual selected objects move.
    pub fn exchange_with_opening_hand(&mut self,game:&mut GameState,player:PlayerId,designation:&str,hand_card:ObjectId)->Result<(ObjectId,ObjectId),PregameSetupError>{
        if self.stage!=Stage::OpeningHand{return Err(PregameSetupError::WrongStage);}
        let key=(player,designation.trim().to_ascii_lowercase());
        let chosen=self.chosen.get(&key).ok_or(PregameSetupError::NotChosen)?;
        if chosen.used{return Err(PregameSetupError::AlreadyUsed);}
        if !matches!(chosen.spec.action,PregameChosenCardAction::ExchangeWithOpeningHand){return Err(PregameSetupError::MissingSetupAbility);}
        let source=game.object(chosen.source).ok_or(PregameSetupError::InvalidSource)?;
        if source.owner!=player||source.zone!=Zone::OutsideGame{return Err(PregameSetupError::InvalidSource);}
        let hand=game.object(hand_card).ok_or(PregameSetupError::InvalidOpeningHandCard)?;
        if hand.owner!=player||hand.zone!=Zone::Hand||!game.player(player).is_some_and(|p|p.hand.contains(&hand_card)){return Err(PregameSetupError::InvalidOpeningHandCard);}
        let source_id=chosen.source;
        let outgoing=game.move_object_by_effect(hand_card,Zone::OutsideGame).ok_or(PregameSetupError::MovementFailed)?;
        let incoming=game.move_object_by_effect(source_id,Zone::Hand).ok_or(PregameSetupError::MovementFailed)?;
        self.chosen.get_mut(&key).unwrap().used=true;
        Ok((incoming,outgoing))
    }
    /// Run one source-owned setup program. Different selected sources are
    /// independent; existing effect composition owns every choice and move.
    /// A browser caller may replay against cloned game/session state to expose
    /// the program's ordinary decision contexts before committing it.
    pub fn run_source_setup_program(&mut self,game:&mut GameState,player:PlayerId,source:ObjectId,ability_index:usize,dm:&mut dyn crate::decision::DecisionMaker)->Result<(),PregameSetupError>{
        if self.stage!=Stage::Choosing{return Err(PregameSetupError::WrongStage);}
        if self.source_programs_used.contains(&(source,ability_index)){return Err(PregameSetupError::AlreadyUsed);}
        let object=game.object(source).ok_or(PregameSetupError::InvalidSource)?;
        let Some(AbilityKind::Static(ability))=object.abilities.get(ability_index).map(|a|&a.kind) else{return Err(PregameSetupError::MissingSetupAbility);};
        let Some(PregameActionKind::SourceSetup{source_zone})=ability.pregame_action_kind() else{return Err(PregameSetupError::MissingSetupAbility);};
        if object.owner!=player||object.zone!=source_zone{return Err(PregameSetupError::InvalidSource);}
        let effects=ability.pregame_action_effects().ok_or(PregameSetupError::MissingSetupAbility)?.to_vec();
        let checkpoint=game.clone();
        let mut ctx=crate::effects::EffectContext::new(source,player,dm);
        for effect in &effects {
            let outcome=match crate::effects::execute_effect(game,effect,&mut ctx) {
                Ok(outcome)=>outcome,
                Err(error)=>{game.restore_execution_checkpoint(checkpoint,false);return Err(PregameSetupError::SourceProgramFailed(error.to_string()));}
            };
            if ctx.decision_maker.awaiting_choice() {
                game.restore_execution_checkpoint(checkpoint, false);
                return Err(PregameSetupError::SourceProgramPending);
            }
            for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
        }
        self.source_programs_used.insert((source,ability_index));
        Ok(())
    }
    pub fn available_exchanges(&self, game:&GameState, player:PlayerId)->Vec<(ObjectId,String)> {
        if self.stage!=Stage::OpeningHand||game.player(player).is_none_or(|p|p.hand.is_empty()){return vec![];}
        self.chosen.iter().filter_map(|((owner,label),choice)|{
            (*owner==player&&!choice.used&&game.object(choice.source).is_some_and(|o|o.owner==player&&o.zone==Zone::OutsideGame))
                .then(||(choice.source,label.clone()))
        }).collect()
    }
    /// Deck-evidence preparation may add chosen-card predicates after source
    /// setup has completed. Keep every already completed per-source receipt.
    pub fn retain_completed_source_programs_from(&mut self, previous:&Self) {
        self.source_programs_used.extend(previous.source_programs_used.iter().copied());
    }
    pub fn finish(&mut self){self.stage=Stage::Finished;}
}
