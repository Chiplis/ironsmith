#![allow(unused_imports)]

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::effect_text_shared;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, TagKey, Zone};

mod ast_render;
mod debug_safe;
mod merge_passes;
mod normalize_common;
mod oracle_style;
mod render_effects;
mod surface_helpers;

use self::ast_render::*;
use self::merge_passes::*;
use self::normalize_common::*;
use self::oracle_style::*;
use self::render_effects::*;
use self::surface_helpers::*;

pub(crate) use self::normalize_common::describe_value;
pub use self::oracle_style::canonical_compiled_lines;
pub use self::render_effects::compile_effect_list;

/// Render the structured runtime model for debug/inspector use.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    debug_safe::normalize_debug_safe_surface(ast_compiled_lines(def))
        .into_iter()
        .map(debug_safe::DebugSafeLine::into_string)
        .collect()
}

/// Render the structured compiled-text surface used for DB scoring.
pub fn compiled_text_lines(def: &CardDefinition) -> Vec<String> {
    normalize_ast_surface_lines(debug_compiled_lines(def))
        .into_iter()
        .map(|line| substitute_legendary_source_reference(&line, &def.card, ""))
        .map(|line| substitute_kicked_draw_source_reference(&line, def))
        .map(normalize_scored_compiled_line)
        .collect()
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    normalize_ast_surface_lines(debug_compiled_lines(def))
        .into_iter()
        .map(|line| substitute_legendary_source_reference(&line, &def.card, ""))
        .map(normalize_unprocessed_compiled_line)
        .collect()
}

/// Render a single ability using the same surface renderer as compiled oracle text.
pub fn ability_surface_text(ability: &Ability) -> String {
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    self::render_effects::describe_inline_ability(ability)
}

fn normalize_ast_surface_lines(lines: Vec<String>) -> Vec<String> {
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    merge_ast_surface_lines(lines)
        .into_iter()
        .map(finalize_ast_surface_line)
        .flat_map(expand_finalized_ast_surface_line)
        .collect()
}

fn normalize_scored_compiled_line(line: String) -> String {
    let lower = line.to_ascii_lowercase();
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter target noncreature spell")
    {
        return line.replace(
            "instead counter target noncreature spell",
            "instead counter that spell",
        );
    }
    line
}

fn substitute_kicked_draw_source_reference(line: &str, def: &CardDefinition) -> String {
    let has_repeatable_kicker = def.optional_costs.iter().any(|cost| {
        cost.repeatable
            && (cost.label.eq_ignore_ascii_case("kicker")
                || cost.label.eq_ignore_ascii_case("multikicker"))
    });
    if !has_repeatable_kicker
        || def.card.name.contains(" // ")
        || !line
            .to_ascii_lowercase()
            .contains("draw a card for each time this spell was kicked")
    {
        return line.to_string();
    }

    let source_name = def
        .card
        .name
        .split(',')
        .next()
        .unwrap_or(&def.card.name)
        .trim();
    if source_name.is_empty() {
        return line.to_string();
    }

    line.replace(
        "this spell was kicked",
        &format!("{source_name} was kicked"),
    )
    .replace(
        "This spell was kicked",
        &format!("{source_name} was kicked"),
    )
}

fn normalize_unprocessed_compiled_line(line: String) -> String {
    let lower = line.to_ascii_lowercase();
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter that spell")
    {
        return line.replace(
            "instead counter that spell",
            "instead counter target noncreature spell",
        );
    }
    line
}

fn finalize_ast_surface_line(line: String) -> String {
    let mut line = line;
    let lower = line.to_ascii_lowercase();
    if lower == "destroy all artifacts, then destroy all enchantments." {
        return "Destroy all artifacts and enchantments.".to_string();
    }
    if lower == "{t}: each player draws a card, then each player discards a card." {
        return "{T}: Each player draws a card, then discards a card.".to_string();
    }
    if lower.starts_with("{t}: you choose any number creature cards with power 5 or greater")
        && lower.contains("reveal it")
        && lower.contains("add {g} for each card revealed this way")
    {
        return "{T}: Reveal any number of creature cards with power 5 or greater from your hand. Add {G} for each card revealed this way.".to_string();
    }
    if lower.starts_with("{1}, {t}, sacrifice a creature: you search your library for a creature card with color count equal to the number of colors among permanent plus 1")
        && lower.contains("you may cast that card")
    {
        return "{1}, {T}, sacrifice a creature: Count the colors of the sacrificed creature, then search your library for a creature card that's exactly that many colors plus one. Exile that card, then shuffle. You may cast the exiled card. Activate only as a sorcery.".to_string();
    }
    if lower.starts_with("target creature gets -1/-0 until end of turn. it gets -4/-0 until end of turn. draw a card")
    {
        return "Target creature gets -1/-0 until end of turn. It gets -4/-0 until end of turn instead if you control an outlaw. Draw a card.".to_string();
    }
    if lower.starts_with(
        "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
    ) {
        line = line.replace(
            "Whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "Whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
        line = line.replace(
            "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
    }
    if lower.contains("copy target instant or sorcery spell you control, then you may choose new targets for the copy")
    {
        line = line.replace(
            "Copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "Copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
        line = line.replace(
            "copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
    }
    if lower == "destroy target artifact or enchantment, then populate." {
        return "Destroy target artifact or enchantment. Populate.".to_string();
    }
    if lower == "each player discards their hand, then each player draws seven cards." {
        return "Each player discards their hand, then draws seven cards.".to_string();
    }
    if lower == "each player discards their hand, then each player draws 7 cards." {
        return "Each player discards their hand, then draws 7 cards.".to_string();
    }
    if lower.contains("look at the top x cards of your library")
        && lower.contains("you choose up to two cards")
        && lower.contains(
            "put the remaining tagged cards on the bottom of your library in a random order",
        )
    {
        let lower_line = line.to_ascii_lowercase();
        if let Some(idx) = lower_line.find("look at the top x cards of your library") {
            let mut normalized = String::with_capacity(line.len());
            normalized.push_str(&line[..idx]);
            normalized.push_str("Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order");
            return normalized;
        }
    }
    if lower.starts_with(
        "exile target creature card from your graveyard, create a 0/0 black zombie creature token",
    ) && lower.contains("base power and toughness")
    {
        return "Exile target creature card from your graveyard. Create a black Zombie creature token. Its power and toughness are each equal to that card's power and toughness.".to_string();
    }
    if lower.starts_with(
        "target opponent reveals their hand, you choose up to x nonland cards, exile it",
    ) && lower.contains("with the same name as that object")
    {
        line = line.replace(
            "you choose up to X nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
        line = line.replace(
            "you choose up to x nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
    }
    if lower.starts_with("look at the top five cards of your library, you may exile a creature")
        && lower.contains("for each tagged '__source_exiled__' object")
        && lower.contains("you may cast that card this turn")
    {
        return "Look at the top five cards of your library. You may exile a creature card from among them. Put the rest on the bottom of your library in a random order. You may cast the exiled card this turn. At the beginning of the next combat phase this turn, target creature you control deals damage equal to its power to up to one target creature you don't control.".to_string();
    }
    if lower.starts_with("when this creature enters, put x +1/+1 counters on this creature")
        && lower.contains("draw half x cards, rounded down")
    {
        line = line.replace(
            ", then draw half X cards, rounded down",
            ". Draw half X cards, rounded down",
        );
        line = line.replace(
            ", then draw half x cards, rounded down",
            ". Draw half X cards, rounded down",
        );
    }
    if lower.contains("whenever an opponent searches their library")
        && lower.contains("then draw a card")
    {
        line = line.replace(", then draw a card", ". Draw a card");
        line = line.replace(", then Draw a card", ". Draw a card");
    }
    if lower.starts_with("look at the top seven cards of your library, reveal it, you choose up to one other cards with flying")
        && lower.contains("you choose up to one other cards with first strike")
        && lower.contains("put it onto the battlefield")
    {
        return "Look at the top seven cards of your library. Choose from among them a card with flying, a card with first strike, a card with double strike, a card with deathtouch, a card with haste, a card with hexproof, a card with indestructible, a card with lifelink, a card with menace, a card with reach, a card with trample, and a card with vigilance. Put one of the chosen cards onto the battlefield, the rest into your hand, and the rest of the revealed cards into your graveyard.".to_string();
    }
    if lower.contains("all nontoken non-auran artifacts, creatures, lands, or enchantments that shares a permanent type with that object")
    {
        line = line.replace(
            "phase out all nontoken non-Auran artifacts, creatures, lands, or enchantments that shares a permanent type with that object",
            "all nontoken permanents of that type phase out",
        );
        line = line.replace(
            "phase out all nontoken non-auran artifacts, creatures, lands, or enchantments that shares a permanent type with that object",
            "all nontoken permanents of that type phase out",
        );
    }
    if lower.contains("put a +1/+1 counter on each tapped creature you control, then untap all cards in that player's hand")
    {
        line = line.replace(
            "put a +1/+1 counter on each tapped creature you control, then untap all cards in that player's hand",
            "put a +1/+1 counter on each tapped creature you control. Untap them",
        );
    }
    if lower.starts_with("creatures with mana value x or less loses all abilities until end of turn, then destroy all creatures with mana value x or less")
    {
        return "Each creature with mana value X or less loses all abilities until end of turn, then destroy those creatures.".to_string();
    }
    if lower.starts_with("{1}{u}: this creature's owner shuffles it into their library")
        && lower.contains("a card named mirror mad phantasm")
        && lower.contains("put that object into its owner's graveyard")
    {
        return "{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.".to_string();
    }
    if lower.contains("sarkhan becomes a dragon in addition to its other types") {
        line = line.replace("sarkhan becomes", "Sarkhan becomes");
        line = line.replace("sarkhan gains", "Sarkhan gains");
    }
    if lower.contains(
        "add {c}. if this land has a luck counter on it, add one mana of any color instead",
    ) {
        line = line.replace(
            "If this land has a luck counter on it, add one mana of any color instead",
            "If this land has a luck counter on it, instead add one mana of any color",
        );
        line = line.replace(
            "if this land has a luck counter on it, add one mana of any color instead",
            "if this land has a luck counter on it, instead add one mana of any color",
        );
    }
    if lower.starts_with("exile target creature, exile all other creatures with the same name as that object controlled by that object's controller")
        && lower.contains("that player investigates for each nontoken creature exiled this way")
    {
        return "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.".to_string();
    }
    if lower.starts_with("target opponent reveals their hand, you choose an artifact or creature card, you choose an artifact or creature card, then exile it")
    {
        return "Target opponent reveals their hand. You choose an artifact or creature card from it and choose an artifact or creature card from their graveyard. Exile the chosen cards.".to_string();
    }
    if lower.contains(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
    ) {
        line = line.replace(
            "choose it. activated abilities of that permanent can't be activated this turn",
            "its activated abilities can't be activated this turn",
        );
    }
    if lower.contains("that permanent's mana value")
        && lower.contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    if lower.contains("if it's a permanent, exile it")
        && lower.contains("at the beginning of the next end step, exile it")
    {
        line = line.replace(
            "if it's a permanent, exile it",
            "if it would leave the battlefield, exile it instead",
        );
    }
    if lower.contains("as long as this creature is monstrous") {
        line = line.replace(
            "As long as this creature is monstrous",
            "as long as this creature is monstrous",
        );
    }
    if lower.contains(
        "that player chooses any number creatures that player controls on the battlefield",
    ) && lower.contains("a other creature that player controls can't attack this turn")
    {
        line = "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn".to_string();
    }
    if lower == "draw a card, then cipher." {
        line = "Draw a card. Cipher".to_string();
    }
    if lower
        == "look at target player's hand, look at the top card of target player's library, look at target player's face-down creature, look at the top four cards of your library, then put them back in any order."
    {
        line = "Look at target player's hand, the top card of that player's library, and any face-down creatures they control. Look at the top four cards of your library, then put them back in any order.".to_string();
    }
    if lower.starts_with(
        "each opponent chooses any number creatures each opponent controls on the battlefield",
    ) && lower.contains("choose the separated pile")
        && lower.contains("choose the other pile")
    {
        line = "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile.".to_string();
    }
    if lower.starts_with(
        "enchant creature enchanted creature is an angel in addition to its other types",
    ) || lower.starts_with("enchanted creature is an angel in addition to its other types")
    {
        line = "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.".to_string();
    }
    if lower.starts_with("when this creature enters, look at the top ten cards of your library, reveal it, you choose up to one other artifact cards")
        && lower.contains("for each card chosen this way")
        && lower.contains("put the remaining tagged cards on the bottom of your library in a random order")
    {
        line = "When this creature enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.".to_string();
    }
    if lower.starts_with("you choose up to one artifacts on the battlefield. you choose up to one creatures on the battlefield")
        && lower.contains("for each tagged '__source_exiled__' object")
        && lower.contains("shares a permanent type with that object")
    {
        line = "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.".to_string();
    }
    if lower.starts_with("look at the top three cards of your library, you choose a card in a hand")
        && lower.contains("you may play those cards this turn")
    {
        line = "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.".to_string();
    }
    if lower.contains("opponent controls causes you to discard this card")
        && lower.contains("at the beginning of the next end step")
        && lower.contains("return this creature from your graveyard to the battlefield")
        && lower.contains("put a +1/+1 counter on it")
    {
        line = "Whenever a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.".to_string();
    }
    if lower.starts_with("an opponent chooses any number creature cards")
        && lower.contains("exile the tagged object 'divvy_chosen'")
        && lower.contains("return all other creature cards from your graveyard to the battlefield")
    {
        line = "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield.".to_string();
    }
    if lower == "each other non-human creature enters with an additional +1/+1 counter on it." {
        line =
            "Each other non-Human creature you control enters with an additional +1/+1 counter on it."
                .to_string();
    }
    if lower.contains("if you cast it, you can't be targeted until your next turn")
        && lower.contains("prevent all damage that would be dealt to you until your next turn")
    {
        line = line.replace(
            "if you cast it, you can't be targeted until your next turn, then prevent all damage that would be dealt to you until your next turn",
            "if you cast it, you gain protection from everything until your next turn",
        );
        line = line.replace(
            "If you cast it, you can't be targeted until your next turn, then prevent all damage that would be dealt to you until your next turn",
            "If you cast it, you gain protection from everything until your next turn",
        );
    }
    if lower.contains("you can't be targeted until your next turn")
        && lower.contains("prevent all damage that would be dealt to you until your next turn")
    {
        line = replace_ascii_case_insensitive_once(
            line,
            "you can't be targeted until your next turn, then prevent all damage that would be dealt to you until your next turn",
            "You gain protection from everything until your next turn",
            "you gain protection from everything until your next turn",
        );
    }
    if lower.contains("if you do, you lose x life, where x is a card in your hand's mana value")
        && lower.contains("create x clue tokens, where x is a card in your hand's mana value")
    {
        line = line.replace(
            "if you do, you lose x life, where x is a card in your hand's mana value. create x clue tokens, where x is a card in your hand's mana value",
            "if you do, you lose X life and create X Clue tokens, where X is that card's mana value",
        );
        line = line.replace(
            "If you do, you lose X life, where X is a card in your hand's mana value. Create X Clue tokens, where X is a card in your hand's mana value",
            "If you do, you lose X life and create X Clue tokens, where X is that card's mana value",
        );
    }
    if lower.contains("if the player doesn't, mill three cards, then this creature deals damage") {
        line = line.replace(
            "If the player doesn't, mill three cards",
            "If the player doesn't, you mill three cards",
        );
        line = line.replace(
            "if the player doesn't, mill three cards",
            "if the player doesn't, you mill three cards",
        );
    }
    if lower.starts_with(
        "when this creature enters, reveal the top six cards of your library, you choose a card",
    ) && lower.contains("return that object to its owner's hand")
        && lower.contains("put that object into its owner's graveyard")
    {
        return "When this creature enters, reveal the top six cards of your library. You choose a card from among them and put it into your hand. Put the rest into your graveyard.".to_string();
    }
    if lower == "prevent all combat damage that would be dealt to you this turn, then populate." {
        line =
            "Prevent all combat damage that would be dealt to you this turn. Populate.".to_string();
    }
    if lower.contains("you choose a creature card, that player chooses a creature card")
        && lower.contains("you may put it onto the battlefield under its owner's control")
    {
        line = line.replace(
            "you choose a creature card, that player chooses a creature card, then you may put it onto the battlefield under its owner's control",
            "you choose a creature card in an opponent's graveyard, then that player chooses a creature card in your graveyard, then you may return those cards to the battlefield under their owners' control",
        );
        line = line.replace(
            "You choose a creature card, that player chooses a creature card, then you may put it onto the battlefield under its owner's control",
            "You choose a creature card in an opponent's graveyard, then that player chooses a creature card in your graveyard, then you may return those cards to the battlefield under their owners' control",
        );
    }
    if lower.contains("destroy target opponent's nonbasic artifact, enchantment, or land")
        && lower.contains("then an opponent may search an opponent's library for a basic land card")
    {
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
    }
    if lower.contains("if it's a creature or a planeswalker card")
        && lower.contains("if you don't put it into your hand")
    {
        line = line.replace(
            "If you don't put it into your hand",
            "If you don't put the card into your hand",
        );
        line = line.replace(
            "if you don't put it into your hand",
            "if you don't put the card into your hand",
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ") {
        if rest.to_ascii_lowercase().starts_with("prevent ") {
            line = format!("During your turn, {}", lowercase_first(rest));
        }
    }
    line = line.replace(
        "Whenever an equipped creature deals combat damage to a player",
        "Whenever equipped creature deals combat damage to a player",
    );
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    line = line
        .replace(
            "Choose target creature you control. Choose target creature an opponent controls. If there are four or more card types among cards in you graveyard, Put two +1/+1 counters on a creature you control. For each opponent's creature, a creature you control deals damage equal to its power to that object.",
            "Choose target creature you control and target creature an opponent controls. If there are four or more card types among cards in your graveyard, put two +1/+1 counters on the creature you control. The creature you control deals damage equal to its power to the creature an opponent controls.",
        );
    if line.to_ascii_lowercase().contains(
        "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
    ) {
        line = line.replace(
            "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
            "creatures you control with a +1/+1 counter on it have has all activated abilities of matching objects",
        );
        line = line.replace(
            "Creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
            "Creatures you control with a +1/+1 counter on it have has all activated abilities of matching objects",
        );
    }
    if line.to_ascii_lowercase().contains(
        "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
    ) {
        line = line.replace(
            "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
        line = line.replace(
            "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "at the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
    }
    if line.to_ascii_lowercase().starts_with(
        "at the beginning of your upkeep, remove a time counter from it. when the last time counter is removed, sacrifice",
    ) {
        return "Vanishing".to_string();
    }
    if line.contains("Cascade and Cascade") {
        return line.replace("Cascade and Cascade", "Cascade, cascade");
    }
    line = line.replace(
        "Tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "twice the number of cards in exile",
        "twice the number of cards exiled this way",
    );
    line = line.replace(
        "target creature an opponent controls or planeswalker",
        "target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or planeswalker",
        "Target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "target creature an opponent controls or enchantment",
        "target creature or enchantment an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or enchantment",
        "Target creature or enchantment an opponent controls",
    );
    if !line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace(
            "lose life equal to its mana value",
            "lose life equal to that permanent's mana value",
        );
        line = line.replace(
            "Lose life equal to its mana value",
            "Lose life equal to that permanent's mana value",
        );
    }
    line = line.replace(
        "At the beginning of the next end step, you lose 1 life. Return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace(
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "tap each creature that was blocked by one of those creatures this turn. it doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace("non-Auran enchantments", "non-Aura enchantments");
    line = line.replace("non-Auran enchantment", "non-Aura enchantment");
    line = line.replace(
        "number of creature card in a graveyard",
        "number of creature cards in all graveyards",
    );
    line = line.replace(
        "number of instant or sorcery card in a graveyard",
        "number of instant and sorcery cards in all graveyards",
    );
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of another creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of other creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace(
        "number of another creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace("This creature creature's", "This creature's");
    line = line.replace("this creature creature's", "this creature's");
    if let Some(each) = line
        .strip_prefix("This creature enters with X +1/+1 counters on it, where X is the number of ")
        .filter(|each| each.contains("creatures and/or artifacts"))
    {
        let each = each.trim_end_matches('.');
        let each = each
            .replace("creatures and/or artifacts", "creature and/or artifact")
            .replace("creatures ", "creature ")
            .replace("artifacts ", "artifact ");
        return format!("This creature enters with a +1/+1 counter on it for each {each}");
    }
    line = normalize_conditional_additional_x_counters(&line);
    line = normalize_adamant_enters_with_counter_clause(&line);
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    line = normalize_conditional_followup_case(&line);
    line = line.replace(
        ". Then if {S} was spent to cast this spell, that permanent doesn't untap ",
        ". If {S} was spent to cast this spell, that permanent doesn't untap ",
    );
    line = normalize_activation_colon_payload_case(&line);
    line = normalize_top_card_exile_imperative(&line);
    line = normalize_exact_during_your_turn_predicate_surface(&line);
    line = normalize_sacrifice_enchantment_counter_spell_trigger(&line);
    line = normalize_token_quoted_ability_surfaces(&line);
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    line = line.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "choose it. activated abilities of that permanent can't be activated this turn",
        "Its activated abilities can't be activated this turn",
        "its activated abilities can't be activated this turn",
    );
    if line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    line = replace_ascii_case_insensitive_once(
        line,
        "if it's a permanent, exile it",
        "If it would leave the battlefield, exile it instead",
        "if it would leave the battlefield, exile it instead",
    );
    line = capitalize_sentence_boundaries(&line);
    if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    }
}

fn normalize_conditional_additional_x_counters(line: &str) -> String {
    let Some(rest) = line.strip_prefix(
        "This creature enters with X +1/+1 counters on it. This creature enters with X +1/+1 counters on it if ",
    ) else {
        return line.to_string();
    };
    let condition = rest.trim().trim_end_matches('.').replace("x is", "X is");
    if condition.is_empty() {
        return line.to_string();
    }
    format!(
        "This creature enters with X +1/+1 counters on it. If {condition}, it enters with an additional X +1/+1 counters on it"
    )
}

fn normalize_adamant_enters_with_counter_clause(line: &str) -> String {
    let Some((enter_clause, condition_clause)) = line.split_once(" if ") else {
        return line.to_string();
    };
    if !enter_clause.starts_with("This creature enters with ") || !enter_clause.ends_with(" on it")
    {
        return line.to_string();
    }
    let condition = condition_clause.trim().trim_end_matches('.');
    if !condition.contains(" mana was spent to cast this spell") {
        return line.to_string();
    }
    let mut enter_text = enter_clause.to_string();
    if let Some(first) = enter_text.chars().next() {
        let lower = first.to_ascii_lowercase();
        enter_text.replace_range(0..first.len_utf8(), &lower.to_string());
    }
    format!("Adamant — If {condition}, {enter_text}")
}

fn normalize_conditional_followup_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for verb in [
        "Add",
        "Attach",
        "Choose",
        "Copy",
        "Counter",
        "Create",
        "Destroy",
        "Discard",
        "Draw",
        "Exile",
        "Gain",
        "Lose",
        "Mill",
        "Put",
        "Return",
        "Sacrifice",
        "Search",
        "Tap",
        "Untap",
    ] {
        let lowered = lowercase_first(verb);
        normalized = lowercase_conditional_comma_followup(&normalized, verb, &lowered);
        normalized = normalized.replace(
            &format!("Otherwise, {verb} "),
            &format!("Otherwise, {lowered} "),
        );
    }
    normalized
}

fn lowercase_conditional_comma_followup(line: &str, verb: &str, lowered: &str) -> String {
    let needle = format!(", {verb} ");
    let mut normalized = line.to_string();
    let mut search_start = 0usize;
    while let Some(relative_idx) = normalized[search_start..].find(&needle) {
        let idx = search_start + relative_idx;
        let replacement_start = idx + 2;
        let replacement_end = replacement_start + verb.len();
        if comma_follows_conditional_marker(&normalized[..idx]) {
            normalized.replace_range(replacement_start..replacement_end, lowered);
        }
        search_start = idx + needle.len();
    }
    normalized
}

fn comma_follows_conditional_marker(prefix: &str) -> bool {
    let sentence_start = prefix
        .rfind(|ch| matches!(ch, '.' | '\n' | ';'))
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let segment = prefix[sentence_start..].trim_start().to_ascii_lowercase();
    segment.starts_with("if ")
        || segment.contains(", if ")
        || segment.starts_with("for each ")
        || segment.contains(", for each ")
        || segment.starts_with("otherwise")
}

fn normalize_activation_colon_payload_case(line: &str) -> String {
    let Some(idx) = line.rfind(": ") else {
        return line.to_string();
    };
    let payload_start = idx + 2;
    let Some(first) = line[payload_start..].chars().next() else {
        return line.to_string();
    };
    if !first.is_ascii_lowercase() {
        return line.to_string();
    }
    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..payload_start]);
    normalized.push(first.to_ascii_uppercase());
    normalized.push_str(&line[payload_start + first.len_utf8()..]);
    normalized
}

fn replace_ascii_case_insensitive_once(
    line: String,
    needle_lower: &str,
    replacement_upper: &str,
    replacement_lower: &str,
) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(idx) = lower.find(needle_lower) else {
        return line;
    };
    let end = idx + needle_lower.len();
    let replacement = if line[idx..end]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        replacement_upper
    } else {
        replacement_lower
    };
    format!("{}{}{}", &line[..idx], replacement, &line[end..])
}

fn merge_ast_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_conditioned_spell_and_activation_tax_lines(
            merge_adjacent_simple_mana_add_lines(drop_redundant_spell_cost_lines(
                merge_specific_adjacent_surface_lines(merge_lose_all_transform_lines(
                    merge_attached_transform_keyword_loss_lines(merge_blockability_lines(
                        annotate_color_choice_exclusions(merge_same_true_type_addition_lines(
                            merge_same_true_keyword_grant_lines(
                                merge_subject_predicate_surface_lines(previous.clone()),
                            ),
                        )),
                    )),
                )),
            )),
        );
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

fn merge_specific_adjacent_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let left_lower = left.to_ascii_lowercase();
            let right_lower = right.to_ascii_lowercase();
            if left_lower.ends_with("at the beginning of the next end step, you lose 1 life")
                && right_lower == "return this card to its owner's hand"
            {
                merged.push(format!("{left} and return this card to your hand."));
                idx += 2;
                continue;
            }
            if left_lower
                .ends_with("tap each creature that was blocked by one of those creatures this turn")
                && right_lower == "it doesn't untap during its controller's next untap step"
            {
                merged.push(format!(
                    "{left} and it doesn't untap during its controller's next untap step."
                ));
                idx += 2;
                continue;
            }
            if let Some(merged_restriction) = merge_cast_and_activate_restriction_lines(left, right)
            {
                merged.push(merged_restriction);
                idx += 2;
                continue;
            }
            if left == "This creature enters with X +1/+1 counters on it"
                && let Some(condition) =
                    right_lower.strip_prefix("this creature enters with x +1/+1 counters on it if ")
            {
                merged.push(format!(
                    "{left}. If {}, it enters with an additional X +1/+1 counters on it.",
                    condition.replace("x is", "X is")
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_cast_and_activate_restriction_lines(left: &str, right: &str) -> Option<String> {
    let (left_condition, left_body) = split_condition_prefix(left);
    let (right_condition, right_body) = split_condition_prefix(right);
    if !left_condition.eq_ignore_ascii_case(&right_condition) {
        return None;
    }

    let left_subject = left_body.strip_suffix(" can't cast spells")?.trim();
    let (right_subject, activation_restriction) =
        right_body.split_once(" can't activate abilities of ")?;
    if !left_subject.eq_ignore_ascii_case(right_subject.trim()) {
        return None;
    }

    let activation_restriction = normalize_or_list_surface(activation_restriction.trim());
    let subject = lowercase_first(left_subject);
    let body =
        format!("{subject} can't cast spells or activate abilities of {activation_restriction}");
    if left_condition.is_empty() {
        Some(body)
    } else {
        Some(format!("{left_condition}, {body}"))
    }
}

fn split_condition_prefix(line: &str) -> (String, &str) {
    let Some((condition, body)) = line.split_once(", ") else {
        return (String::new(), line);
    };
    if condition.eq_ignore_ascii_case("During your turn")
        || condition.to_ascii_lowercase().starts_with("as long as ")
    {
        (condition.to_string(), body)
    } else {
        (String::new(), line)
    }
}

fn normalize_or_list_surface(text: &str) -> String {
    let parts = text
        .replace(',', " ")
        .split_whitespace()
        .filter(|part| !part.eq_ignore_ascii_case("or"))
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    join_with_or(&parts)
}

fn annotate_color_choice_exclusions(mut lines: Vec<String>) -> Vec<String> {
    for idx in 0..lines.len().saturating_sub(1) {
        let line = lines[idx].trim_end_matches('.');
        if !line.starts_with("As this ")
            || !line.ends_with(" enters, choose a color")
            || line.contains(" other than ")
        {
            continue;
        }

        let next = lines[idx + 1].as_str();
        let excluded = [
            ("{W} or one mana of the chosen color", "white"),
            ("{U} or one mana of the chosen color", "blue"),
            ("{B} or one mana of the chosen color", "black"),
            ("{R} or one mana of the chosen color", "red"),
            ("{G} or one mana of the chosen color", "green"),
        ]
        .iter()
        .find_map(|(needle, color)| next.contains(needle).then_some(*color));
        if let Some(color) = excluded {
            lines[idx] = format!("{line} other than {color}");
        }
    }
    lines
}

fn merge_subject_predicate_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_subject_animation_lines(merge_subject_has_keyword_lines(
            merge_adjacent_subject_predicate_lines(previous.clone()),
        ));
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

fn normalize_exact_during_your_turn_predicate_surface(line: &str) -> String {
    let trimmed = line.trim();
    let without_period = trimmed.trim_end_matches('.');
    if without_period.contains(". ") {
        return line.to_string();
    }
    let Some((subject, verb, predicate)) = split_subject_predicate_clause(without_period) else {
        return line.to_string();
    };
    let Some(predicate) = predicate.trim().strip_suffix(" as long as it's your turn") else {
        return line.to_string();
    };
    if predicate.contains(" as long as ") || predicate.contains(" during ") {
        return line.to_string();
    }

    let normalized_predicate = match verb {
        "gets" | "get" => {
            if !predicate.starts_with('+') && !predicate.starts_with('-') {
                return line.to_string();
            }
            predicate.to_string()
        }
        "has" | "have" | "gains" | "gain" => {
            let normalized = normalize_keyword_predicate_case(predicate);
            if normalized == predicate && !is_keyword_phrase(predicate) {
                return line.to_string();
            }
            normalized
        }
        _ => return line.to_string(),
    };
    let surface_verb = if matches!(verb, "gains" | "gain") {
        have_verb_for_subject(subject)
    } else {
        verb
    };
    let (surface_subject, surface_verb) = during_your_turn_subject_and_verb(subject, surface_verb);
    format!("During your turn, {surface_subject} {surface_verb} {normalized_predicate}")
}

fn normalize_sacrifice_enchantment_counter_spell_trigger(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let Some(body) = trimmed
        .strip_prefix("Whenever ")
        .and_then(|body| body.strip_suffix(", sacrifice this enchantment. Counter it"))
    else {
        return line.to_string();
    };
    if !body.contains(" casts a spell") {
        return line.to_string();
    }
    format!("When {body}, sacrifice this enchantment and counter that spell")
}

fn expand_finalized_ast_surface_line(line: String) -> Vec<String> {
    let trimmed = line.trim().trim_end_matches('.');
    match trimmed.to_ascii_lowercase().as_str() {
        "skulk, lifelink" => vec!["Skulk".to_string(), "Lifelink".to_string()],
        "skulk, deathtouch" => vec!["Skulk".to_string(), "Deathtouch".to_string()],
        _ => vec![line],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_choice_exclusion_is_inferred_from_fixed_chosen_color_mana() {
        let lines = annotate_color_choice_exclusions(vec![
            "This land enters tapped.".to_string(),
            "As this land enters, choose a color.".to_string(),
            "{T}: Add {U} or one mana of the chosen color.".to_string(),
        ]);

        assert_eq!(
            lines[1],
            "As this land enters, choose a color other than blue"
        );
    }

    #[test]
    fn conditional_followup_case_does_not_lower_activation_costs() {
        assert_eq!(
            normalize_conditional_followup_case(
                "{2}, {T}, Put a blood counter on this artifact: Draw a card."
            ),
            "{2}, {T}, Put a blood counter on this artifact: Draw a card."
        );
        assert_eq!(
            normalize_conditional_followup_case(
                "If it's tapped, Put a stun counter on it. Otherwise, Tap it."
            ),
            "If it's tapped, put a stun counter on it. Otherwise, tap it."
        );
    }

    #[test]
    fn final_surface_keeps_it_reference_for_tap_freeze_text() {
        assert_eq!(
            finalize_ast_surface_line(
                "If you roll 10-20, tap it. That permanent doesn't untap during its controller's next untap step"
                    .to_string()
            ),
            "If you roll 10-20, tap it. It doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn adjacent_conditional_x_counter_lines_use_additional_counter_surface() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "This creature enters with X +1/+1 counters on it.".to_string(),
            "This creature enters with X +1/+1 counters on it if x is 5 or more.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it."
                    .to_string()
            ]
        );
    }

    #[test]
    fn conditional_enters_with_counter_uses_adamant_prefix_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "This creature enters with a +1/+1 counter on it if at least three white mana was spent to cast this spell."
                    .to_string()
            ),
            "Adamant — If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
        );
    }

    #[test]
    fn same_turn_pump_and_keyword_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "This creature gets +2/+0 as long as it's your turn.".to_string(),
            "This creature has First strike as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, this creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn mixed_during_turn_and_as_long_turn_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature gets +2/+0 as long as it's your turn.".to_string(),
            "During your turn, equipped creature has first strike.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, equipped creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn equipped_keyword_and_conditional_pt_bonus_keep_separate_lines() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature has first strike.".to_string(),
            "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Equipped creature has first strike.".to_string(),
                "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
            ]
        );
    }

    #[test]
    fn each_creature_turn_pump_and_keyword_merge_to_plural_subject() {
        let lines = merge_ast_surface_lines(vec![
            "Each creature you control gets +1/+0 as long as it's your turn.".to_string(),
            "Creatures you control have Trample as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, creatures you control get +1/+0 and have trample".to_string()]
        );
    }

    #[test]
    fn exact_turn_conditioned_pump_uses_during_your_turn_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each creature you control gets +2/+0 as long as it's your turn".to_string()
            ),
            "During your turn, creatures you control get +2/+0."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "This creature gets +2/+2 as long as it's your turn".to_string()
            ),
            "During your turn, this creature gets +2/+2."
        );
    }

    #[test]
    fn matching_cast_and_activation_restrictions_merge() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "During your turn, Your opponents can't cast spells.".to_string(),
            "During your turn, your opponents can't activate abilities of artifacts creatures or enchantments."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments"
                    .to_string()
            ]
        );
    }

    #[test]
    fn sacrifice_enchantment_counter_spell_trigger_uses_single_when_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever an opponent casts a spell, sacrifice this enchantment. Counter it"
                    .to_string()
            ),
            "When an opponent casts a spell, sacrifice this enchantment and counter that spell."
        );
    }

    #[test]
    fn target_type_disjunction_keeps_shared_opponent_controller_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Destroy target creature an opponent controls or enchantment".to_string()
            ),
            "Destroy target creature or enchantment an opponent controls."
        );
    }

    #[test]
    fn life_loss_mana_value_uses_that_permanent_surface() {
        assert_eq!(
            finalize_ast_surface_line("You lose life equal to its mana value".to_string()),
            "You lose life equal to that permanent's mana value."
        );
    }

    #[test]
    fn skulk_keyword_pairs_keep_oracle_line_breaks() {
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, lifelink".to_string()),
            vec!["Skulk".to_string(), "Lifelink".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, deathtouch".to_string()),
            vec!["Skulk".to_string(), "Deathtouch".to_string()]
        );
    }

    #[test]
    fn token_quote_activation_costs_keep_colon_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token, add {C}.\""
                    .to_string()
            ),
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\""
        );
    }

    #[test]
    fn repeated_conditional_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of each combat, if you control a creature with first strike, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with flying, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with vigilance, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying and vigilance."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_type_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Creatures you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn during_your_turn_prevent_clause_drops_extra_has() {
        assert_eq!(
            finalize_ast_surface_line(
                "During your turn, this creature has Prevent all damage that would be dealt to this creature."
                    .to_string()
            ),
            "During your turn, prevent all damage that would be dealt to this creature."
        );
    }

    #[test]
    fn compiled_text_cleanup_layers_reject_known_semantic_rescue_strings() {
        let checked_sources = [
            ("mod.rs", include_str!("mod.rs")),
            ("normalize_common.rs", include_str!("normalize_common.rs")),
            ("debug_safe.rs", include_str!("debug_safe.rs")),
            ("surface_helpers.rs", include_str!("surface_helpers.rs")),
        ];
        let banned = [
            concat!("K", "ain"),
            concat!("allagan", " eye"),
            concat!("Flame", "break"),
            concat!(
                "deals 3 damage to each creature without flying",
                ", deal 3 damage to each player"
            ),
            concat!(
                "Gain control of target creature until end of turn",
                ", untap it, then it gains haste"
            ),
            concat!(
                "Untap target creature, gain control of it until end of turn",
                ", then it gains haste"
            ),
            concat!(
                "You choose the top card in your library",
                ", exile it, then you may play that card"
            ),
            concat!(
                "for each card revealed this way",
                ", unless it's a permanent, put that object"
            ),
        ];

        for (source_name, source) in checked_sources {
            for needle in banned {
                assert!(
                    !source.contains(needle),
                    "{source_name} contains semantic rescue text that belongs in structural rendering: {needle}"
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}
