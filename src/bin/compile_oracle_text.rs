use std::env;
use std::io::{self, Read};

use ironsmith::ability::{Ability, AbilityKind, ActivationTiming};
use ironsmith::alternative_cast::AlternativeCastingMethod;
use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardDefinition, Effect, ManaSymbol, Zone};

fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "You".to_string(),
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::Specific(_) => "that player".to_string(),
        PlayerFilter::Active => "the active player".to_string(),
        PlayerFilter::Defending => "the defending player".to_string(),
        PlayerFilter::Attacking => "the attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "the damaged player".to_string(),
        PlayerFilter::Teammate => "a teammate".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::ControllerOf(_) => "its controller".to_string(),
        PlayerFilter::OwnerOf(_) => "its owner".to_string(),
    }
}

fn describe_choose_spec(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Target(inner) => format!("target {}", describe_choose_spec(inner)),
        ChooseSpec::AnyTarget => "any target".to_string(),
        ChooseSpec::Object(filter) => filter.description(),
        ChooseSpec::Player(filter) => describe_player_filter(filter),
        ChooseSpec::Source => "this source".to_string(),
        ChooseSpec::SourceController => "you".to_string(),
        ChooseSpec::SourceOwner => "this source's owner".to_string(),
        ChooseSpec::Tagged(tag) => format!("tagged {}", tag.as_str()),
        ChooseSpec::All(filter) => format!("all {}", filter.description()),
        ChooseSpec::EachPlayer(filter) => format!("each {}", describe_player_filter(filter)),
        ChooseSpec::SpecificObject(_) => "that object".to_string(),
        ChooseSpec::SpecificPlayer(_) => "that player".to_string(),
        ChooseSpec::Iterated => "that object".to_string(),
        ChooseSpec::WithCount(inner, _) => describe_choose_spec(inner),
    }
}

fn describe_mana_symbol(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        ManaSymbol::Generic(v) => format!("{{{v}}}"),
        ManaSymbol::Snow => "{S}".to_string(),
        ManaSymbol::Life(_) => "{P}".to_string(),
        ManaSymbol::X => "{X}".to_string(),
    }
}

fn describe_cost_component(cost: &ironsmith::costs::Cost) -> String {
    if let Some(mana_cost) = cost.mana_cost_ref() {
        return format!("Pay {}", mana_cost.to_oracle());
    }
    if cost.requires_tap() {
        return "Tap this source".to_string();
    }
    if cost.requires_untap() {
        return "Untap this source".to_string();
    }
    if let Some(amount) = cost.life_amount() {
        return if amount == 1 {
            "Pay 1 life".to_string()
        } else {
            format!("Pay {amount} life")
        };
    }
    if cost.is_sacrifice_self() {
        return "Sacrifice this source".to_string();
    }
    let display = cost.display().trim().to_string();
    if display.is_empty() {
        format!("{cost:?}")
    } else {
        display
    }
}

fn describe_effect(effect: &Effect) -> String {
    if let Some(sequence) = effect.downcast_ref::<ironsmith::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .map(describe_effect)
            .collect::<Vec<_>>()
            .join(". ");
    }
    if let Some(for_each) = effect.downcast_ref::<ironsmith::effects::ForEachObject>() {
        return format!(
            "For each {}, {}",
            for_each.filter.description(),
            for_each
                .effects
                .iter()
                .map(describe_effect)
                .collect::<Vec<_>>()
                .join(". ")
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<ironsmith::effects::ForEachTaggedEffect>() {
        return format!(
            "For each tagged '{}' object, {}",
            for_each_tagged.tag.as_str(),
            for_each_tagged
                .effects
                .iter()
                .map(describe_effect)
                .collect::<Vec<_>>()
                .join(". ")
        );
    }
    if let Some(choose) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() {
        return format!(
            "{} chooses {:?} {} in {:?} and tags it as '{}'",
            describe_player_filter(&choose.chooser),
            choose.count,
            choose.filter.description(),
            choose.zone,
            choose.tag.as_str()
        );
    }
    if let Some(move_to_zone) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>() {
        let target = describe_choose_spec(&move_to_zone.target);
        return match move_to_zone.zone {
            Zone::Exile => format!("Exile {target}"),
            Zone::Graveyard => format!("Put {target} into its owner's graveyard"),
            Zone::Hand => format!("Return {target} to its owner's hand"),
            Zone::Library => {
                if move_to_zone.to_top {
                    format!("Put {target} on top of its owner's library")
                } else {
                    format!("Put {target} on the bottom of its owner's library")
                }
            }
            Zone::Battlefield => format!("Put {target} onto the battlefield"),
            Zone::Stack => format!("Put {target} on the stack"),
            Zone::Command => format!("Move {target} to the command zone"),
        };
    }
    if let Some(deal_damage) = effect.downcast_ref::<ironsmith::effects::DealDamageEffect>() {
        return format!(
            "Deal {:?} damage to {}",
            deal_damage.amount,
            describe_choose_spec(&deal_damage.target)
        );
    }
    if let Some(put_counters) = effect.downcast_ref::<ironsmith::effects::PutCountersEffect>() {
        return format!(
            "Put {:?} {:?} counter(s) on {}",
            put_counters.count,
            put_counters.counter_type,
            describe_choose_spec(&put_counters.target)
        );
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<ironsmith::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return format!(
            "Return {} from graveyard to the battlefield{}",
            describe_choose_spec(&return_to_battlefield.target),
            if return_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(draw) = effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>() {
        let player = describe_player_filter(&draw.player);
        let verb = if player == "You" { "draw" } else { "draws" };
        return format!("{player} {verb} {:?}", draw.count);
    }
    if let Some(gain) = effect.downcast_ref::<ironsmith::effects::GainLifeEffect>() {
        return format!("{} gains {:?}", describe_choose_spec(&gain.player), gain.amount);
    }
    if let Some(tap) = effect.downcast_ref::<ironsmith::effects::TapEffect>() {
        return format!("Tap {}", describe_choose_spec(&tap.spec));
    }
    if let Some(untap) = effect.downcast_ref::<ironsmith::effects::UntapEffect>() {
        return format!("Untap {}", describe_choose_spec(&untap.spec));
    }
    if let Some(attach) = effect.downcast_ref::<ironsmith::effects::AttachToEffect>() {
        return format!("Attach this source to {}", describe_choose_spec(&attach.target));
    }
    if let Some(sacrifice) = effect.downcast_ref::<ironsmith::effects::SacrificeEffect>() {
        return format!(
            "{} sacrifices {:?} {}",
            describe_player_filter(&sacrifice.player),
            sacrifice.count,
            sacrifice.filter.description()
        );
    }
    if let Some(return_to_hand) = effect.downcast_ref::<ironsmith::effects::ReturnToHandEffect>() {
        return format!(
            "Return {} to its owner's hand",
            describe_choose_spec(&return_to_hand.spec)
        );
    }
    if let Some(grant_all) = effect.downcast_ref::<ironsmith::effects::GrantAbilitiesAllEffect>() {
        return format!(
            "{} gains {} {:?}",
            grant_all.filter.description(),
            grant_all
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            grant_all.duration
        );
    }
    if let Some(grant_target) =
        effect.downcast_ref::<ironsmith::effects::GrantAbilitiesTargetEffect>()
    {
        return format!(
            "{} gains {} {:?}",
            describe_choose_spec(&grant_target.target),
            grant_target
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            grant_target.duration
        );
    }
    if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
        return format!(
            "Tag '{}' then {}",
            tagged.tag.as_str(),
            describe_effect(&tagged.effect)
        );
    }
    format!("{effect:?}")
}

fn describe_timing(timing: &ActivationTiming) -> &'static str {
    match timing {
        ActivationTiming::AnyTime => "any time",
        ActivationTiming::SorcerySpeed => "sorcery speed",
        ActivationTiming::DuringCombat => "during combat",
        ActivationTiming::OncePerTurn => "once per turn",
        ActivationTiming::DuringYourTurn => "during your turn",
        ActivationTiming::DuringOpponentsTurn => "during opponents turn",
    }
}

fn describe_ability(index: usize, ability: &Ability) -> Vec<String> {
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            vec![format!("Static ability {index}: {}", static_ability.display())]
        }
        AbilityKind::Triggered(triggered) => {
            let mut line = format!("Triggered ability {index}: {}", triggered.trigger.display());
            let mut clauses = Vec::new();
            if !triggered.choices.is_empty() {
                let choices = triggered
                    .choices
                    .iter()
                    .map(describe_choose_spec)
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("choose {choices}"));
            }
            if !triggered.effects.is_empty() {
                let effects = triggered
                    .effects
                    .iter()
                    .map(describe_effect)
                    .collect::<Vec<_>>()
                    .join(". ");
                clauses.push(effects);
            }
            if !clauses.is_empty() {
                line.push_str(": ");
                line.push_str(&clauses.join(": "));
            }
            vec![line]
        }
        AbilityKind::Activated(activated) => {
            let mut line = format!("Activated ability {index}: timing {}", describe_timing(&activated.timing));
            let mut pre = Vec::new();
            if !activated.mana_cost.costs().is_empty() {
                pre.push(
                    activated
                        .mana_cost
                        .costs()
                        .iter()
                        .map(describe_cost_component)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            if !activated.choices.is_empty() {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                line.push_str(": ");
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                line.push_str(": ");
                line.push_str(
                    &activated
                        .effects
                        .iter()
                        .map(describe_effect)
                        .collect::<Vec<_>>()
                        .join(". "),
                );
            }
            vec![line]
        }
        AbilityKind::Mana(mana_ability) => {
            let mut line = format!("Mana ability {index}");
            let mut parts = Vec::new();
            if !mana_ability.mana_cost.costs().is_empty() {
                parts.push(
                    mana_ability
                        .mana_cost
                        .costs()
                        .iter()
                        .map(describe_cost_component)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            if !mana_ability.mana.is_empty() {
                parts.push(format!(
                    "Add {}",
                    mana_ability
                        .mana
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join("")
                ));
            }
            if !parts.is_empty() {
                line.push_str(": ");
                line.push_str(&parts.join(", "));
            }
            if let Some(extra_effects) = &mana_ability.effects
                && !extra_effects.is_empty()
            {
                line.push_str(": ");
                line.push_str(
                    &extra_effects
                        .iter()
                        .map(describe_effect)
                        .collect::<Vec<_>>()
                        .join(". "),
                );
            }
            vec![line]
        }
    }
}

fn compiled_lines(def: &CardDefinition) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        match method {
            AlternativeCastingMethod::AlternativeCost {
                name,
                mana_cost,
                cost_effects,
            } => {
                let mut parts = Vec::new();
                if let Some(cost) = mana_cost {
                    parts.push(format!("Pay {}", cost.to_oracle()));
                }
                if !cost_effects.is_empty() {
                    parts.push(
                        cost_effects
                            .iter()
                            .map(describe_effect)
                            .collect::<Vec<_>>()
                            .join(". "),
                    );
                }
                if parts.is_empty() {
                    out.push(format!("Alternative cast {} ({}): free", idx + 1, name));
                } else {
                    out.push(format!(
                        "Alternative cast {} ({}): {}",
                        idx + 1,
                        name,
                        parts.join(": ")
                    ));
                }
            }
            other => out.push(format!("Alternative cast {}: {}", idx + 1, other.name())),
        }
    }
    for (idx, ability) in def.abilities.iter().enumerate() {
        out.extend(describe_ability(idx + 1, ability));
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
    {
        out.push(format!(
            "Spell effects: {}",
            spell_effects
                .iter()
                .map(describe_effect)
                .collect::<Vec<_>>()
                .join(". ")
        ));
    }
    if !def.cost_effects.is_empty() {
        out.push(format!(
            "Spell cost effects: {}",
            def.cost_effects
                .iter()
                .map(describe_effect)
                .collect::<Vec<_>>()
                .join(". ")
        ));
    }
    out
}

fn read_input_text(text_arg: Option<String>) -> Result<String, String> {
    if let Some(text) = text_arg {
        return Ok(text);
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if input.trim().is_empty() {
        return Err("missing oracle text (pass --text or stdin)".to_string());
    }
    Ok(input)
}

fn main() -> Result<(), String> {
    let mut name = "Parser Probe".to_string();
    let mut text_arg: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => {
                name = args
                    .next()
                    .ok_or_else(|| "--name requires a value".to_string())?;
            }
            "--text" => {
                text_arg = Some(
                    args.next()
                        .ok_or_else(|| "--text requires a value".to_string())?,
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --name <value> and/or --text <value>"
                ));
            }
        }
    }

    let text = read_input_text(text_arg)?;
    let builder = CardDefinitionBuilder::new(CardId::new(), &name);
    let def = builder
        .parse_text(text)
        .map_err(|err| format!("parse failed: {err:?}"))?;

    println!("Name: {}", def.card.name);
    println!("Type: {}", def.card.card_types.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(" "));
    println!("Compiled abilities/effects");
    let lines = compiled_lines(&def);
    if lines.is_empty() {
        println!("- <none>");
    } else {
        for line in lines {
            println!("- {}", line.trim());
        }
    }

    Ok(())
}
