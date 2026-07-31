use super::*;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::compiled_text::unprocessed_compiled_lines;
use crate::effect::Value;
use crate::effects::CantEffect;
use crate::effects::{
    AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfLandProducedTypesEffect,
    AddScaledManaEffect, CreateTokenCopyEffect, DestroyEffect, DiscardEffect, DoubleCountersEffect,
    DrawCardsEffect, EnergyCountersEffect, ExchangeControlEffect, ExchangeValuesEffect,
    ExchangeZonesEffect, ExileInsteadOfGraveyardEffect, FatesealEffect, ForEachObject,
    ForPlayersEffect, GrantBySpecEffect, LookAtHandEffect, ModifyPowerToughnessForEachEffect,
    PutCountersEffect, RemoveCountersEffect, RemoveUpToAnyCountersEffect,
    ReturnFromGraveyardToBattlefieldEffect, SacrificeEffect, SetBasePowerToughnessEffect,
    SetLifeTotalEffect, SkipCombatPhasesEffect, SkipDrawStepEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, SurveilEffect, TaggedEffect, TapEffect,
};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::types::Subtype;
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

fn visit_nested_effects<T: 'static>(
    effects: &[crate::effect::Effect],
    mut visitor: impl FnMut(&T),
) {
    fn visit<T: 'static>(effect: &crate::effect::Effect, visitor: &mut dyn FnMut(&T)) {
        if let Some(effect) = effect.downcast_ref::<T>() {
            visitor(effect);
        }
        effect.visit_child_effects(&mut |child| visit(child, visitor));
    }

    for effect in effects {
        visit(effect, &mut visitor);
    }
}

mod dynamic_color_among;
#[cfg(ironsmith_runtime_parser_tests)]
mod shard_00;
mod shard_01;
mod shard_02;
