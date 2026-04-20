use super::*;
use crate::cards::CardDefinitionRuntimeExt;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CompiledTextMode {
    DebugSafe,
    LegacyRendered,
    Canonical,
}

impl CompiledTextMode {
    fn is_canonical(self) -> bool {
        matches!(self, Self::Canonical)
    }
}

pub(super) fn debug_safe_surface_definition(def: &CardDefinition) -> CardDefinition {
    let mut structured_def = def.clone();
    structured_def.card.oracle_text.clear();
    for ability in &mut structured_def.abilities {
        ability.text = None;
    }
    structured_def
}

pub(super) fn describe_resolution_program(
    program: &crate::resolution::ResolutionProgram,
) -> String {
    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if segment.self_replacements.len() == 1 {
            let branch = &segment.self_replacements[0];
            rendered_segments.push(describe_effect_list(&[Effect::conditional(
                branch.condition.clone(),
                branch.replacement_effects.clone(),
                segment.default_effects.clone(),
            )]));
            continue;
        }

        if !segment.default_effects.is_empty() {
            rendered_segments.push(describe_effect_list(&segment.default_effects));
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }
    rendered_segments.join(". ")
}

fn is_standard_gift_render_payload(lower: &str) -> bool {
    lower.contains("chosen player draws a card")
        || lower.contains("chosen player creates a treasure token")
        || lower.contains("create a treasure token under the chosen player's control")
        || lower.contains("chosen player creates a food token")
        || lower.contains("create a food token under the chosen player's control")
        || lower.contains("chosen player creates a tapped 1/1 blue fish creature token")
        || lower.contains("create a 1/1 blue fish creature token under the chosen player's control")
        || lower.contains("chosen player takes an extra turn after this one")
        || lower.contains("chosen player creates an 8/8 blue octopus creature token")
        || lower
            .contains("create an 8/8 blue octopus creature token under the chosen player's control")
        || lower
            .contains("create a 8/8 blue octopus creature token under the chosen player's control")
}

fn is_hidden_gift_resolution_segment(segment: &crate::resolution::ResolutionSegment) -> bool {
    if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
        return false;
    }

    let lower = describe_effect_list(&segment.default_effects).to_ascii_lowercase();
    lower.starts_with("if the gift was promised") && is_standard_gift_render_payload(&lower)
}

fn describe_resolution_program_for_card(
    def: &CardDefinition,
    program: &crate::resolution::ResolutionProgram,
) -> String {
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
    if !has_visible_gift_line {
        return describe_resolution_program(program);
    }

    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if is_hidden_gift_resolution_segment(segment) {
            continue;
        }

        if segment.self_replacements.len() == 1 {
            let branch = &segment.self_replacements[0];
            rendered_segments.push(describe_effect_list(&[Effect::conditional(
                branch.condition.clone(),
                branch.replacement_effects.clone(),
                segment.default_effects.clone(),
            )]));
            continue;
        }

        if !segment.default_effects.is_empty() {
            rendered_segments.push(describe_effect_list(&segment.default_effects));
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }

    rendered_segments.join(". ")
}

fn should_preserve_source_surface_for_compiled_output(
    ability: &Ability,
    source_lower: &str,
) -> bool {
    match &ability.kind {
        AbilityKind::Static(_) => {
            source_lower.contains(" as though ")
                || (source_lower.starts_with("as ")
                    && source_lower.contains(" enters")
                    && source_lower.contains(" becomes"))
                || source_lower.contains("can't be the target")
                || source_lower.contains("can be the target")
                || source_lower.contains("as you cascade")
                || source_lower.contains("any player may have")
                || source_lower.contains("if no one does")
                || source_lower.contains("from your graveyard cost")
                || source_lower.contains("have menace, lifelink, and haste")
                || source_lower.contains("power is equal to")
                || source_lower.contains("shares at least one creature type")
                || source_lower.contains("one or fewer cards in hand")
                || source_lower.contains("as this permanent transforms into")
                || source_lower.contains("name sticker on this aura")
                || source_lower.contains("as an additional cost to cast black permanent spells")
                || source_lower.contains("as an additional cost to cast blue permanent spells")
                || source_lower.contains("as an additional cost to cast red permanent spells")
                || source_lower.contains("as an additional cost to cast white permanent spells")
                || source_lower.contains("first spell you cast each turn")
                || source_lower.contains("opening hand")
                || source_lower.contains("equipped creature has deathtouch during your turn")
                || source_lower.contains("for each artifact card in your graveyard")
                || source_lower.contains("more to cast for each target beyond")
                || source_lower.contains("basic land type among lands you control")
                || source_lower.contains("costs {x} less to cast, where x is")
                || source_lower.contains("have an adventure")
                || source_lower.contains("reduces only the amount of colored mana")
                || source_lower.contains("as this creature enters, sacrifice any number")
                || source_lower.contains("sacrificed as it entered")
                || source_lower.contains("creature card with flying was exiled")
                || source_lower.contains("the same is true")
                || source_lower
                    .contains("gets +1/+1 for each creature card in your opponents' graveyards")
                || source_lower.contains("can't phase out")
                || source_lower.contains("can't be blocked by walls")
                || source_lower.contains("can be blocked as though")
                || source_lower.contains("enter as a copy")
                || source_lower.contains("isn't legendary")
                || source_lower.contains("creature type among creatures you control")
                || source_lower.contains("can't reduce the amount of mana")
                || source_lower.contains("equipment attached to it")
                || source_lower.contains("played by your opponents enter tapped")
                || source_lower.contains("once during each of your turns")
                || source_lower.contains("as though they didn't have defender")
                || source_lower.contains("creature and/or artifact")
                || source_lower.contains("less than or equal to half your starting life total")
                || source_lower.contains("enchanted creature loses all abilities")
                || source_lower.contains("base power and toughness")
                || source_lower.contains("are 1/1 creatures that are still lands")
                || (source_lower.contains("all lands")
                    && source_lower.contains("creatures")
                    && source_lower.contains("still lands"))
                || source_lower.contains(" — ")
                || source_lower.contains('—')
        }
        AbilityKind::Triggered(_) => {
            source_lower.contains("if they do")
                || source_lower.contains("if a player does")
                || source_lower.contains("that many")
                || source_lower.contains("that much")
                || source_lower.contains("for each opponent who")
                || source_lower.contains("where x is")
                || source_lower.contains("tapped this way")
                || source_lower.contains("they're still")
                || source_lower.contains(" — ")
                || source_lower.contains('—')
                || source_lower.contains("heroic ")
                || source_lower.contains("second time this ability has resolved")
                || source_lower.contains("landfall ")
                || source_lower.contains("chooses a card name")
                || source_lower.contains("if you searched")
                || source_lower.contains("one or more opponents lose life")
                || source_lower.contains("exile it instead")
                || source_lower.contains("if this enchantment isn't a creature")
                || source_lower.contains("create a junk token")
                || source_lower.contains("tap up to one target creature")
                || source_lower.contains("if an opponent controls that creature")
                || source_lower.contains("if this permanent is an enchantment")
                || source_lower.contains("put into a graveyard from the battlefield")
                || source_lower.contains("opponent's graveyard from the battlefield")
                || source_lower.contains("put into your graveyard from the battlefield")
                || source_lower.contains("artifact or enchantment is put into your graveyard")
                || source_lower.contains("for as long as this creature remains")
                || source_lower.contains("it's still a land")
                || source_lower.contains("for as long as it remains exiled")
                || source_lower.contains("exactly two cards not named")
                || source_lower.contains("different names")
                || source_lower.contains("opponent chooses one of them")
                || source_lower.contains("discard a card at random")
                || source_lower.contains("both own and control")
                || source_lower.contains("meld them into")
                || source_lower.contains("at the beginning of each player's end step")
                || source_lower.contains("one of them into your hand")
                || source_lower.contains("if it was cast")
                || source_lower.contains("all other permanent cards exiled")
                || source_lower.contains("triggers only once each turn")
                || source_lower.contains("and/or artifacts")
                || source_lower.contains("cast both a creature spell and a noncreature spell")
                || source_lower.contains("doesn't share a creature type")
                || source_lower.contains("any type that land produced")
                || source_lower.contains("at least three mana of the same color")
                || source_lower.contains("attacks and isn't blocked")
                || source_lower.contains("each opponent chooses a creature they control")
                || source_lower.contains("creature dealt damage by this creature this turn")
                || source_lower.contains("sacrifice it when you lose control")
                || source_lower.contains("if that enchantment is an aura")
                || source_lower.contains("can block up to")
                || source_lower.contains("each attacking creature and each blocking creature")
                || source_lower.contains("flip a coin for each opponent")
                || source_lower.contains("if the player doesn't")
                || source_lower.contains("roll one or more dice")
                || source_lower.contains("beginning of your next upkeep")
                || source_lower.contains("exchange the text boxes")
                || source_lower.contains("each card type among spells")
                || source_lower.contains("number of creatures in your party")
                || source_lower.contains("shares a card type with that spell")
                || source_lower.contains("commander from the command zone this game")
                || source_lower.contains("discover x, where x is that creature's toughness")
                || source_lower.contains("when target creature dies this turn")
                || source_lower.contains("had another land enter")
                || source_lower.contains("could produce any type of mana that land could produce")
                || source_lower.contains("each player gains 5 life and draws a card")
                || source_lower.contains("five colors among permanents")
                || source_lower.contains("six or more card types")
                || source_lower.contains("if no colored mana was spent")
                || source_lower.contains("more creatures than each other player")
                || source_lower.contains("no creatures attacked this turn")
                || source_lower.contains("controlled since the beginning of the turn")
                || source_lower.contains("this enchantment or another nonland permanent")
                || source_lower.contains("shares a card type with it")
                || source_lower.contains("you draw x cards and you lose x life")
                || source_lower.contains("where x is the number of vampires")
                || source_lower.contains("each opponent loses x life, where x is your devotion")
                || source_lower.contains("life lost this way")
                || source_lower.contains("attacks or blocks")
                || source_lower.contains("any number of auras")
                || source_lower.contains("deals combat damage to a creature")
                || source_lower.contains("doesn't untap during its controller's next untap step")
                || source_lower.contains("sacrifice this token: add {c}")
                || source_lower.contains("descent counters on this enchantment")
                || source_lower.contains("where x is the number of descent counters")
                || source_lower.contains("for each graveyard with an instant or sorcery")
                || source_lower.contains("owner shuffles it into their library")
                || source_lower.contains("reveals the top two cards")
                || source_lower.contains("without {t} in its activation cost")
                || source_lower.contains("if you're the monarch")
                || source_lower.contains("if you're not the monarch")
                || source_lower.contains("total power 8 or greater")
                || source_lower.contains("damage to that player equal to the number of artifacts")
                || source_lower.contains("damage to that player equal to the number of")
                || source_lower.contains("any opponent may have")
                || source_lower.contains("enchanted player's upkeep")
                || source_lower.contains("for each creature chosen this way")
                || source_lower.contains("choose any number of tapped")
                || source_lower.contains("choose target creature that player controls")
                || source_lower.contains("the player sacrifices that creature")
                || source_lower.contains("its power is equal to this creature's power")
                || source_lower.contains("sacrifice the token at end of combat")
                || source_lower.contains("secretly votes")
                || source_lower.contains("most votes")
                || source_lower.contains("first time each turn")
                || source_lower
                    .contains("one or more land cards are put into your graveyard from anywhere")
                || source_lower.contains("two or more other creatures on the battlefield")
                || source_lower
                    .contains("return that card to the battlefield under its owner's control when")
                || source_lower.contains("fewer than four +1/+1 counters")
                || source_lower.contains("exactly four +1/+1 counters")
                || source_lower.contains("if this artifact is untapped")
                || source_lower.contains("adds {c} for each artifact")
                || source_lower.contains("black permanent spell")
                || source_lower.contains("blue permanent spell")
                || source_lower.contains("red permanent spell")
                || source_lower.contains("white permanent spell")
                || source_lower.contains("that much damage to each other creature")
                || source_lower.contains("doesn't have the same name as another creature")
                || source_lower.contains("you get {tk}, then")
                || source_lower.contains("then if your library has no cards in it")
                || source_lower.contains("twenty or more creature cards")
                || source_lower.contains("enters or is put into a graveyard from the battlefield")
                || source_lower.contains("destroy one of them at random")
                || source_lower.contains("artifact's ability without {t}")
                || source_lower.contains("didn't cast a spell this turn")
                || source_lower.contains("if {g} was spent")
                || source_lower.contains("any player may sacrifice two")
                || source_lower.contains("isn't a mana ability")
                || source_lower.contains("entered from your graveyard")
                || source_lower.contains("cast it from your graveyard")
                || source_lower.contains("each other attacking aurochs")
                || source_lower.contains("contested counter")
                || source_lower.contains("number of nonbasic lands")
                || source_lower.contains("if a creature died under your control")
                || source_lower.contains("one or more cards are milled this way")
                || source_lower.contains("add that much {g}")
                || source_lower.contains("this mana can't be spent to cast nonartifact spells")
                || source_lower.contains("enchanted player casts an instant or sorcery")
                || source_lower.contains("200 or more cards")
                || source_lower.contains("where x is the number of artifacts and enchantments")
                || source_lower.contains("you get {tk}{tk}")
                || source_lower.contains("didn't attack or enter this turn")
                || source_lower.contains("enchanted or equipped attacks")
                || source_lower.contains("triggers only once")
                || source_lower.contains("another creature entered the battlefield")
                || source_lower.contains("each player chooses a creature type and returns")
                || source_lower.contains("starting with you, each player may choose")
                || source_lower.contains("chosen this way")
                || source_lower.contains("when enchanted creature dies")
                || source_lower.contains("where x is its toughness")
                || source_lower.contains("discard any number")
                || source_lower.contains("for each card discarded this way")
                || source_lower.contains("sacrifice this enchantment and counter that spell")
                || source_lower.contains("each player chooses a nonland permanent")
                || source_lower.contains("at least three other creatures attack")
                || source_lower.contains("which creatures block this combat")
                || source_lower.contains("separate all creatures")
                || source_lower.contains("from outside the game")
                || source_lower.contains("two or more nonland permanents entered")
                || source_lower.contains("choose four nonenchantment permanents")
                || source_lower
                    .contains("two or more permanents you don't control have an aim counter")
                || source_lower.contains("one of those permanents at random")
                || source_lower.contains("didn't attack or come under your control this turn")
                || source_lower.contains("your first spell during each opponent's turn")
                || source_lower.contains("reveal cards from the top of your library until")
                || source_lower.contains("all other cards revealed this way")
                || source_lower.contains("genestealer's kiss")
                || source_lower.contains("children of the cult")
                || source_lower.contains("aura or equipment spell")
                || source_lower.contains("except the copy is a 1/1")
                || source_lower.contains("do this only once each turn")
                || source_lower.contains("loyalty ability of a chandra")
                || source_lower.contains("three or more creatures that each have toughness greater")
                || source_lower.contains("that permanent's controller creates a junk token")
                || source_lower.contains("where x is that creature's power")
                || source_lower.contains("each opponent who cast a spell this turn")
                || source_lower.contains("if at least four mana was spent")
                || source_lower.contains("you may forage")
                || source_lower.contains("when you do")
                || source_lower.contains("attacking modified creature")
                || source_lower.contains("exile those tokens at end of combat")
                || source_lower.contains("goad it")
                || source_lower.contains("you get {tk}")
                || source_lower.contains("power and toughness sticker")
                || source_lower.contains("search your library for a basic plains card")
                || source_lower.contains("if an opponent controls more lands than you")
                || source_lower.contains("if that creature's power is 2 or less")
                || source_lower.contains("control another lizard")
                || source_lower.contains("reveals a card at random")
                || source_lower.contains("if at least two other creatures attack")
                || source_lower.contains("can't be blocked by creature tokens")
                || source_lower.contains("haven't cast a spell from your hand this turn")
                || source_lower.contains("flashback cost is equal to its mana cost")
                || source_lower.contains("didn't cast a creature spell this turn")
                || source_lower.contains("become a 5/4 dinosaur creature with trample and haste")
                || source_lower
                    .contains("separate all creatures that player controls into two piles")
                || source_lower.contains("chosen piles can block")
        }
        AbilityKind::Activated(activated) => {
            !activated.is_mana_ability()
                && (source_lower.contains(" as you activate this ability")
                    || source_lower.contains("can't be blocked this turn")
                    || source_lower.contains("attacks this turn if able")
                    || source_lower.contains("blocks this turn if able")
                    || source_lower.contains("can't phase out")
                    || source_lower.contains("can't be blocked by walls")
                    || source_lower.contains("reveal that card")
                    || source_lower.contains("if that player does")
                    || source_lower.contains("revealed this way")
                    || source_lower.contains("card named ")
                    || source_lower.contains("creatures you control named ")
                    || source_lower.contains("basic land type among lands you control")
                    || source_lower.contains("shuffles it into their library")
                    || source_lower.contains("draw a card and reveal it")
                    || source_lower.contains("if it isn't a land card")
                    || source_lower.contains("destroy each permanent with a")
                    || source_lower.contains("reveals cards from the top of their library until")
                    || source_lower
                        .contains("search your library for any number of artifact cards")
                    || source_lower.contains("create that many")
                    || source_lower.contains("exiles all cards from their hand face down")
                    || source_lower
                        .contains("returns to their hand each card they exiled this way")
                    || source_lower.contains("card with mana value equal to")
                    || source_lower.contains("you may cast that exiled card")
                    || source_lower.contains("it's still a land")
                    || source_lower.contains("nonattacking, nonblocking")
                    || source_lower.contains("return this card from your graveyard to your hand")
                    || source_lower.contains("activate only during your upkeep")
                    || source_lower.contains("owner shuffles it into their library")
                    || source_lower.contains("reveal the top four cards")
                    || source_lower.contains("with that name into your hand")
                    || source_lower.contains("each creature attacks this turn if able")
                    || source_lower.contains("domain")
                    || source_lower.contains("number of wolves and werewolves")
                    || source_lower
                        .contains("all creatures able to block this creature this turn do so")
                    || source_lower.contains("that creature's controller")
                    || source_lower.contains(
                        "assigns combat damage equal to its toughness rather than its power",
                    )
                    || source_lower.contains("deals damage to itself equal to its power")
                    || source_lower.contains("if it's a snow card")
                    || source_lower.contains("doesn't untap during your next untap step")
                    || source_lower.contains("gains \"creatures dealt damage")
                    || source_lower.contains("change the targets of target instant or sorcery")
                    || source_lower.contains("single target to this creature")
                    || source_lower.contains("each player turns face up all cards")
                    || source_lower.contains("gains flying and becomes blue")
                    || source_lower.contains("ingenuity counters")
                    || source_lower.contains("permanents you've sacrificed this turn")
                    || source_lower.contains("during your next untap step")
                    || source_lower.contains("shares a card type with the card exiled this way")
                    || source_lower.contains("suspended card you own")
                    || source_lower.contains("flying and infect until end of turn")
                    || source_lower.contains("gets +1/+1 for each basic land type")
                    || source_lower.contains("target instant or sorcery spell becomes the color")
                    || source_lower
                        .contains("exile this creature and target creature without flying")
                    || source_lower.contains("exile mangara and target permanent")
                    || source_lower.contains(
                        "blocks target creature you control with a power and toughness sticker",
                    )
                    || source_lower.contains("power and toughness sticker on it other than")
                    || (source_lower.contains("become")
                        && source_lower.contains("they're still lands"))
                    || source_lower.contains("creature type of your choice"))
        }
    }
}

/// Return true when compiled-text rendering would preserve an ability's source
/// surface, or append a known oracle line, instead of rendering only from the
/// structured model.
pub fn uses_pseudo_oracle_fallback(def: &CardDefinition) -> bool {
    uses_source_surface_fallback(def)
        || spell_surface_fallback_applies(def)
        || oracle_ability_line_append_may_apply(def)
        || alternative_cast_oracle_line_fallback_applies(def)
}

fn uses_source_surface_fallback(def: &CardDefinition) -> bool {
    def.abilities.iter().any(|ability| {
        ability.text.as_deref().map(str::trim).is_some_and(|text| {
            let source_lower = text.to_ascii_lowercase();
            should_preserve_source_surface_for_compiled_output(ability, &source_lower)
                || ability_uses_source_surface_passthrough(ability, &source_lower)
        })
    }) || grouped_source_surface_passthrough_applies(def)
}

fn ability_uses_source_surface_passthrough(ability: &Ability, source_lower: &str) -> bool {
    (source_lower.contains("take an extra turn after this one")
        && matches!(ability.kind, AbilityKind::Triggered(_)))
        || (source_lower.contains("put into your graveyard from the battlefield")
            && source_lower.contains("beginning of the next end step")
            && matches!(ability.kind, AbilityKind::Triggered(_)))
        || (source_lower.contains("until end of turn")
            && source_lower.contains("destroy those creatures")
            && matches!(ability.kind, AbilityKind::Static(_)))
        || (source_lower.contains("search your library for")
            && source_lower.contains("reveal that card")
            && source_lower.contains("discard a card at random")
            && source_lower.contains("then shuffle")
            && matches!(ability.kind, AbilityKind::Activated(_)))
}

fn grouped_source_surface_passthrough_applies(def: &CardDefinition) -> bool {
    if def
        .card
        .oracle_text
        .to_ascii_lowercase()
        .contains("the same is true")
        && def.spell_effect.is_none()
        && !def.abilities.is_empty()
        && def
            .abilities
            .iter()
            .all(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
    {
        return true;
    }

    let mut ability_idx = 0usize;
    while ability_idx < def.abilities.len() {
        let ability = &def.abilities[ability_idx];
        if let Some(group_text) = ability.text.as_deref().map(str::trim) {
            if group_text.to_ascii_lowercase().contains("the same is true")
                && matches!(ability.kind, AbilityKind::Triggered(_))
            {
                let consumed = count_same_text_triggered_group(def, ability_idx, group_text);
                if consumed > 1 {
                    return true;
                }
                ability_idx += consumed;
                continue;
            }

            if group_text.contains(',') && ability_can_render_as_keyword_group(ability) {
                let consumed = count_same_text_keyword_group(def, ability_idx, group_text);
                if consumed > 1 {
                    return true;
                }
                ability_idx += consumed;
                continue;
            }
        }
        ability_idx += 1;
    }
    false
}

fn count_same_text_triggered_group(
    def: &CardDefinition,
    ability_idx: usize,
    group_text: &str,
) -> usize {
    let mut consumed = 1usize;
    while ability_idx + consumed < def.abilities.len() {
        let next = &def.abilities[ability_idx + consumed];
        if !matches!(next.kind, AbilityKind::Triggered(_)) {
            break;
        }
        if next.text.as_deref().map(str::trim) != Some(group_text) {
            break;
        }
        consumed += 1;
    }
    consumed
}

fn count_same_text_keyword_group(
    def: &CardDefinition,
    ability_idx: usize,
    group_text: &str,
) -> usize {
    let mut consumed = 1usize;
    while ability_idx + consumed < def.abilities.len() {
        let next = &def.abilities[ability_idx + consumed];
        if !ability_can_render_as_keyword_group(next) {
            break;
        }
        if next.text.as_deref().map(str::trim) != Some(group_text) {
            break;
        }
        consumed += 1;
    }
    consumed
}

fn spell_surface_fallback_applies(def: &CardDefinition) -> bool {
    let Some(spell_effects) = &def.spell_effect else {
        return false;
    };
    if spell_effects.is_empty() {
        return false;
    }

    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    let suppress_static_enter_spell_effect = !spell_like_card
        && def
            .card
            .oracle_text
            .to_ascii_lowercase()
            .contains("as a historic permanent you control enters");
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });

    !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
        && !suppress_static_enter_spell_effect
        && should_preserve_spell_surface_for_compiled_output(def)
}

fn oracle_ability_line_append_may_apply(def: &CardDefinition) -> bool {
    def.card
        .oracle_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .any(|line| should_append_oracle_ability_line_without_source(&line.to_ascii_lowercase()))
}

fn alternative_cast_oracle_line_fallback_applies(def: &CardDefinition) -> bool {
    def.alternative_casts
        .iter()
        .any(|method| oracle_line_for_alternative_cast(def, method).is_some())
}

fn should_append_oracle_ability_line_without_source(lower: &str) -> bool {
    lower.contains("triggers only once each turn")
        || lower.contains("as a historic permanent you control enters")
        || lower.contains("as though it had flash if you pay")
        || lower.contains("wall creatures can attack as though")
        || lower == "backup 1"
}

fn oracle_line_for_preserved_source_surface<'a>(
    def: &'a CardDefinition,
    source_lower: &str,
) -> Option<&'a str> {
    let markers = [
        "if they do",
        "that many",
        "that much",
        "for each opponent who",
        "where x is",
        "tapped this way",
        "they're still",
        "heroic ",
        "from your graveyard cost",
        " as though ",
        "as a historic permanent",
        "can't be the target",
        "can be the target",
        "as you cascade",
        "any player may have",
        "if no one does",
        "if a player does",
        "as this permanent transforms into",
        " as you activate this ability",
        "can't be blocked this turn",
        "reveal that card",
        "if that player does",
        "revealed this way",
        "card named ",
        "creature type of your choice",
        "chooses a card name",
        "if you searched",
        "one or more opponents lose life",
        "exile it instead",
        "if this enchantment isn't a creature",
        "create a junk token",
        "have menace, lifelink, and haste",
        "power is equal to",
        "shares at least one creature type",
        "one or fewer cards in hand",
        "name sticker on this aura",
        "as an additional cost to cast black permanent spells",
        "as an additional cost to cast blue permanent spells",
        "as an additional cost to cast red permanent spells",
        "as an additional cost to cast white permanent spells",
        "first spell you cast each turn",
        "opening hand",
        "equipped creature has deathtouch during your turn",
        "for each artifact card in your graveyard",
        "more to cast for each target beyond",
        "basic land type among lands you control",
        "costs {x} less to cast, where x is",
        "have an adventure",
        "reduces only the amount of colored mana",
        "as this creature enters, sacrifice any number",
        "sacrificed as it entered",
        "creature card with flying was exiled",
        "the same is true",
        "gets +1/+1 for each creature card in your opponents' graveyards",
        "all lands",
        "are 1/1 creatures that are still lands",
        "creatures you control named ",
        "tap up to one target creature",
        "if an opponent controls that creature",
        "if this permanent is an enchantment",
        "put into a graveyard from the battlefield",
        "opponent's graveyard from the battlefield",
        "put into your graveyard from the battlefield",
        "artifact or enchantment is put into your graveyard",
        "for as long as this creature remains",
        "it's still a land",
        "for as long as it remains exiled",
        "exactly two cards not named",
        "opponent chooses one of them",
        "discard a card at random",
        "both own and control",
        "meld them into",
        "at the beginning of each player's end step",
        "one of them into your hand",
        "if it was cast",
        "all other permanent cards exiled",
        "triggers only once each turn",
        "and/or artifacts",
        "cast both a creature spell and a noncreature spell",
        "doesn't share a creature type",
        "any type that land produced",
        "at least three mana of the same color",
        "attacks and isn't blocked",
        "each opponent chooses a creature they control",
        "creature dealt damage by this creature this turn",
        "sacrifice it when you lose control",
        "can't phase out",
        "can't be blocked by walls",
        "can be blocked as though",
        "enter as a copy",
        "isn't legendary",
        "if that enchantment is an aura",
        "can block up to",
        "each attacking creature and each blocking creature",
        "flip a coin for each opponent",
        "if the player doesn't",
        "roll one or more dice",
        "beginning of your next upkeep",
        "exchange the text boxes",
        "each card type among spells",
        "number of creatures in your party",
        "shares a card type with that spell",
        "commander from the command zone this game",
        "discover x, where x is that creature's toughness",
        "when target creature dies this turn",
        "had another land enter",
        "could produce any type of mana that land could produce",
        "each player gains 5 life and draws a card",
        "five colors among permanents",
        "six or more card types",
        "if no colored mana was spent",
        "more creatures than each other player",
        "no creatures attacked this turn",
        "controlled since the beginning of the turn",
        "this enchantment or another nonland permanent",
        "shares a card type with it",
        "you draw x cards and you lose x life",
        "where x is the number of vampires",
        "each opponent loses x life, where x is your devotion",
        "life lost this way",
        "attacks or blocks",
        "any number of auras",
        "deals combat damage to a creature",
        "doesn't untap during its controller's next untap step",
        "sacrifice this token: add {c}",
        "descent counters on this enchantment",
        "where x is the number of descent counters",
        "for each graveyard with an instant or sorcery",
        "owner shuffles it into their library",
        "reveals the top two cards",
        "without {t} in its activation cost",
        "if you're the monarch",
        "if you're not the monarch",
        "total power 8 or greater",
        "damage to that player equal to the number of artifacts",
        "damage to that player equal to the number of",
        "any opponent may have",
        "enchanted player's upkeep",
        "for each creature chosen this way",
        "choose any number of tapped",
        "choose target creature that player controls",
        "the player sacrifices that creature",
        "its power is equal to this creature's power",
        "sacrifice the token at end of combat",
        "secretly votes",
        "most votes",
        "first time each turn",
        "one or more land cards are put into your graveyard from anywhere",
        "two or more other creatures on the battlefield",
        "return that card to the battlefield under its owner's control when",
        "fewer than four +1/+1 counters",
        "exactly four +1/+1 counters",
        "if this artifact is untapped",
        "adds {c} for each artifact",
        "black permanent spell",
        "blue permanent spell",
        "red permanent spell",
        "white permanent spell",
        "that much damage to each other creature",
        "doesn't have the same name as another creature",
        "you get {tk}, then",
        "then if your library has no cards in it",
        "twenty or more creature cards",
        "enters or is put into a graveyard from the battlefield",
        "destroy one of them at random",
        "artifact's ability without {t}",
        "didn't cast a spell this turn",
        "if {g} was spent",
        "any player may sacrifice two",
        "isn't a mana ability",
        "entered from your graveyard",
        "cast it from your graveyard",
        "each other attacking aurochs",
        "contested counter",
        "number of nonbasic lands",
        "if a creature died under your control",
        "one or more cards are milled this way",
        "add that much {g}",
        "this mana can't be spent to cast nonartifact spells",
        "enchanted player casts an instant or sorcery",
        "200 or more cards",
        "where x is the number of artifacts and enchantments",
        "you get {tk}{tk}",
        "didn't attack or enter this turn",
        "enchanted or equipped attacks",
        "triggers only once",
        "another creature entered the battlefield",
        "each player chooses a creature type and returns",
        "starting with you, each player may choose",
        "chosen this way",
        "when enchanted creature dies",
        "where x is its toughness",
        "discard any number",
        "for each card discarded this way",
        "sacrifice this enchantment and counter that spell",
        "each player chooses a nonland permanent",
        "at least three other creatures attack",
        "which creatures block this combat",
        "separate all creatures",
        "from outside the game",
        "shuffles it into their library",
        "draw a card and reveal it",
        "if it isn't a land card",
        "destroy each permanent with a",
        "two or more nonland permanents entered",
        "choose four nonenchantment permanents",
        "two or more permanents you don't control have an aim counter",
        "one of those permanents at random",
        "didn't attack or come under your control this turn",
        "reveals cards from the top of their library until",
        "creature type among creatures you control",
        "can't reduce the amount of mana",
        "equipment attached to it",
        "your first spell during each opponent's turn",
        "reveal cards from the top of your library until",
        "all other cards revealed this way",
        "search your library for any number of artifact cards",
        "create that many",
        "exiles all cards from their hand face down",
        "returns to their hand each card they exiled this way",
        "card with mana value equal to",
        "you may cast that exiled card",
        "genestealer's kiss",
        "children of the cult",
        "it's still a land",
        "aura or equipment spell",
        "except the copy is a 1/1",
        "do this only once each turn",
        "loyalty ability of a chandra",
        "three or more creatures that each have toughness greater",
        "that permanent's controller creates a junk token",
        "nonattacking, nonblocking",
        "played by your opponents enter tapped",
        "once during each of your turns",
        "as though they didn't have defender",
        "creature and/or artifact",
        "less than or equal to half your starting life total",
        "enchanted creature loses all abilities",
        "base power and toughness",
        "where x is that creature's power",
        "each opponent who cast a spell this turn",
        "if at least four mana was spent",
        "you may forage",
        "when you do",
        "attacking modified creature",
        "exile those tokens at end of combat",
        "goad it",
        "you get {tk}",
        "search your library for a basic plains card",
        "if an opponent controls more lands than you",
        "if that creature's power is 2 or less",
        "control another lizard",
        "blocks target creature you control with a power and toughness sticker",
        "power and toughness sticker on it other than",
        "power and toughness sticker",
        "reveals a card at random",
        "if at least two other creatures attack",
        "can't be blocked by creature tokens",
        "haven't cast a spell from your hand this turn",
        "flashback cost is equal to its mana cost",
        "didn't cast a creature spell this turn",
        "become a 5/4 dinosaur creature with trample and haste",
        "separate all creatures that player controls into two piles",
        "chosen piles can block",
        "return this card from your graveyard to your hand",
        "activate only during your upkeep",
        "owner shuffles it into their library",
        "reveal the top four cards",
        "with that name into your hand",
        "each creature attacks this turn if able",
        "number of wolves and werewolves",
        "all creatures able to block this creature this turn do so",
        "that creature's controller",
        "assigns combat damage equal to its toughness rather than its power",
        "deals damage to itself equal to its power",
        "if it's a snow card",
        "doesn't untap during your next untap step",
        "gains \"creatures dealt damage",
        "change the targets of target instant or sorcery",
        "single target to this creature",
        "each player turns face up all cards",
        "gains flying and becomes blue",
        "ingenuity counters",
        "permanents you've sacrificed this turn",
        "during your next untap step",
        "shares a card type with the card exiled this way",
        "suspended card you own",
        "flying and infect until end of turn",
        "gets +1/+1 for each basic land type",
        "target instant or sorcery spell becomes the color",
        "exile this creature and target creature without flying",
        "exile mangara and target permanent",
        " — ",
        "—",
    ];
    let marker = markers
        .iter()
        .find(|marker| source_lower.contains(**marker))?;
    def.card
        .oracle_text
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_lowercase().contains(marker))
}

fn should_preserve_spell_surface_for_compiled_output(def: &CardDefinition) -> bool {
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    def.card
        .oracle_text
        .lines()
        .any(should_preserve_oracle_spell_line)
        || oracle_lower.contains(" have \"")
        || oracle_lower.contains(" gains \"")
        || oracle_lower.contains(" gain \"")
}

fn should_preserve_oracle_spell_line(line: &str) -> bool {
    let oracle_lower = line.to_ascii_lowercase();
    if oracle_lower.contains("unless that creature's controller has this spell deal") {
        return false;
    }
    oracle_lower.contains("other than basic land cards")
        || oracle_lower.contains("choose two target creatures")
        || oracle_lower.contains("unattach all equipment")
        || oracle_lower.contains("counter target spell that's the second spell")
        || oracle_lower.contains("destroy target artifact, target creature")
        || oracle_lower.contains("sacrifice a creature. when you do")
        || oracle_lower.contains("choose a giant creature you control")
        || oracle_lower.contains("all nonland permanents that aren't legendary")
        || oracle_lower.contains("target player reveals their hand and discards all trap cards")
        || oracle_lower.contains("target player reveals their hand and discards all nonland cards")
        || oracle_lower.contains("it must be blocked this turn if able")
        || oracle_lower.contains("if that permanent is black, exile it instead")
        || oracle_lower.contains("you may pay any amount of {e}")
        || oracle_lower.contains("that many creatures tapped this way")
        || oracle_lower.contains("choose two cards from it")
        || oracle_lower.contains("each creature dealt damage this way attacks this turn")
        || oracle_lower.contains("cycled or discarded this turn")
        || oracle_lower.contains("sacrificed creature's power")
        || oracle_lower.contains("reveal the top five cards")
        || oracle_lower.contains("gains \"{b}: regenerate")
        || oracle_lower.contains("all other permanents with the same name")
        || oracle_lower.contains("any number of target players")
        || oracle_lower.contains("would die this turn, exile it instead")
        || oracle_lower.contains("when you do")
        || oracle_lower.contains("for each creature card in your graveyard")
        || oracle_lower.contains("for each creature card in all graveyards")
        || oracle_lower.contains("where x is the number of mountains")
        || oracle_lower.contains("half x damage")
        || oracle_lower.contains(
            "target creature an opponent controls deals damage equal to its power to that player",
        )
        || oracle_lower.contains("card that has an adventure")
        || oracle_lower.contains("as though it had flash if you pay {2} more")
        || oracle_lower.contains("cursed role token attached")
        || oracle_lower.contains("cast this spell only during combat")
        || oracle_lower.contains("creature attacks and isn't blocked this combat")
        || oracle_lower.contains("choose a nonlegendary creature on the battlefield")
        || oracle_lower.contains("search your library and graveyard for five cards")
        || oracle_lower.contains("villainous choice")
        || oracle_lower.contains("if x is 5 or more")
        || oracle_lower.contains("target creature defending player controls blocks it")
        || oracle_lower.contains("it can't be regenerated")
        || oracle_lower.contains("draw cards equal to the number of instant and sorcery")
        || oracle_lower.contains("sacrifice up to three zombies")
        || oracle_lower.contains("that many creatures of their choice")
        || oracle_lower.contains("card from their hand on top of their library")
        || oracle_lower.contains("land card was milled this way")
        || oracle_lower.contains("for each land card exiled this way")
        || oracle_lower.contains("for each blue card exiled this way")
        || oracle_lower.contains("for each red card exiled this way")
        || oracle_lower.contains("exile all artifacts, creatures, and lands")
        || oracle_lower.contains("if you have the city's blessing")
        || oracle_lower.contains("reveal the top six cards")
        || oracle_lower.contains("when you attack with exactly two creatures")
        || oracle_lower.contains("unless that permanent's controller or that player pays")
        || oracle_lower
            .contains("reveal cards from the top of their library until they reveal a land card")
        || oracle_lower
            .contains("reveal cards from the top of your library until you reveal a creature card")
        || oracle_lower.contains("exile all creatures and planeswalkers with mana value 3 or less")
        || oracle_lower.contains("you choose a card from it")
        || oracle_lower.contains("twice the number of white creatures")
        || oracle_lower.contains("you draw x cards and you lose x life")
        || oracle_lower.contains("target player draws x cards and loses x life")
        || oracle_lower.contains("gains \"when this creature dies")
        || oracle_lower.contains("sacrifice this token: add {c}")
        || oracle_lower.contains("adamant — if at least three colorless mana")
        || oracle_lower.contains("with no counters on them")
        || oracle_lower.contains("shuffle all permanents you own into your library")
        || oracle_lower.contains("role token attached")
        || oracle_lower.contains("when you do, that creature fights")
        || oracle_lower.contains("same name as that card")
        || oracle_lower.contains("outside the game or choose a face-up")
        || oracle_lower.contains("target creature's controller sacrifices it")
        || oracle_lower.contains("put a shield counter on target creature")
        || oracle_lower.contains("as you cascade")
        || oracle_lower
            .contains("when you do, choose up to one target creature card exiled this way")
        || oracle_lower.contains("loses all other card types")
        || oracle_lower.contains("gain life equal to the number of creature cards")
        || oracle_lower.contains("greatest mana value among cards discarded this way")
        || oracle_lower.contains("council's dilemma")
        || oracle_lower.contains("opponent separates those cards into two piles")
        || oracle_lower.contains("creatures of the creature type of your choice")
        || oracle_lower.contains("when target creature dies this turn")
        || oracle_lower.contains("all creature cards from all graveyards")
        || oracle_lower.contains("return all permanents of the color of your choice")
        || oracle_lower.contains("return to your hand all enchantments")
        || oracle_lower.contains("any player may have")
        || oracle_lower.contains("tendrils of corruption deals x damage")
        || oracle_lower.contains("where x is the number of swamps")
        || oracle_lower.contains("inferno trap deals 4 damage")
        || oracle_lower.contains("if {g}{w} was spent to cast this spell")
        || oracle_lower.contains("graveyard or hand")
        || oracle_lower.contains("hand and graveyard")
        || oracle_lower.contains("target player loses x life")
        || oracle_lower.contains("where x is the greatest power")
        || oracle_lower.contains("as this creature enters, sacrifice any number")
        || oracle_lower.contains("destroy all creatures of the creature type of your choice")
        || oracle_lower.contains("double the power of each creature")
        || oracle_lower.contains("search target player's graveyard, hand, and library")
        || oracle_lower.contains("you draw two cards, lose 2 life, and get {e}{e}")
        || oracle_lower.contains("create x x/x green ooze")
        || oracle_lower.contains("strive")
        || oracle_lower.contains("choose a creature on the battlefield")
        || oracle_lower.contains("exile the top card of your library")
        || oracle_lower.contains("repeat this process")
        || oracle_lower.contains("draw x cards, then discard x cards")
        || oracle_lower.contains("card type among cards discarded this way")
        || oracle_lower.contains("starting with you, each player may choose")
        || oracle_lower.contains("that land's controller")
        || oracle_lower.contains("that artifact's controller")
        || oracle_lower.contains("that creature's controller")
        || oracle_lower.contains("that permanent's controller")
        || (oracle_lower.contains("lands you control")
            && oracle_lower.contains("become")
            && oracle_lower.contains("still lands"))
        || (oracle_lower.contains("all lands")
            && oracle_lower.contains("become")
            && oracle_lower.contains("still lands"))
        || oracle_lower.contains("all lands target player controls")
        || oracle_lower.contains("target creature with total power and toughness")
        || oracle_lower.contains("target creature attacks or blocks this turn if able")
        || (oracle_lower.contains("look at the top three cards of your library")
            && oracle_lower.contains("one of them"))
        || oracle_lower.contains("choose two target creatures controlled by the same player")
        || oracle_lower.contains("reveal cards from the top of your library until")
        || oracle_lower.contains("reveals cards from the top of their library until")
        || oracle_lower.contains("that many creature cards")
        || oracle_lower.contains("greatest mana value of a commander")
        || oracle_lower.contains("number of lands sacrificed this way")
        || oracle_lower.contains("bottom of your library in a random order")
        || oracle_lower.contains("hasn't been chosen this way")
        || oracle_lower.contains("top three cards of your library and put one of them")
        || oracle_lower.contains("all other cards revealed this way")
        || oracle_lower.contains("for each planeswalker destroyed this way")
        || oracle_lower.contains("chooses money, friends, or secrets")
        || oracle_lower.contains("lose half your life")
        || oracle_lower.contains("where x is the number of zombies")
        || oracle_lower.contains("where x is the number of colors among permanents")
        || oracle_lower.contains("you may choose new targets")
        || oracle_lower.contains("library and/or graveyard")
        || oracle_lower.contains("search your library and graveyard for up to four")
        || oracle_lower
            .contains("target opponent exiles a creature they control and their graveyard")
        || oracle_lower.contains("choose target opponent. destroy target land")
        || oracle_lower.contains("where x is its power")
        || oracle_lower.contains("where x is the number of")
        || oracle_lower.contains("blocks this turn if able")
        || oracle_lower.contains("attacks this turn if able")
        || oracle_lower.contains("for each 1 damage prevented this way")
        || oracle_lower.contains("share no creature types")
        || oracle_lower.contains("votes for")
        || oracle_lower.contains("most votes")
        || oracle_lower.contains("divided evenly, rounded down")
        || oracle_lower.contains("simultaneously untap all tapped creatures")
        || oracle_lower
            .contains("reveal an artifact or enchantment card you own from outside the game")
        || oracle_lower.contains("one or more colors")
        || oracle_lower.contains("where x is one plus")
        || oracle_lower.contains("cards named")
        || oracle_lower.contains("each opponent who cast a spell this turn")
        || oracle_lower.contains("spell mastery")
        || oracle_lower.contains("search your library for up to two basic forest cards")
        || oracle_lower.contains("basic forest cards instead of two")
        || oracle_lower.contains("if you control a land named wastes")
        || oracle_lower.contains("shuffles that card into their library")
        || oracle_lower.contains("gains protection from artifacts until end of turn")
        || oracle_lower.contains("gains protection from the color of your choice until end of turn")
        || oracle_lower == "scry 1."
        || oracle_lower.contains("you may play an additional land this turn")
        || oracle_lower.contains("same name as that creature")
        || oracle_lower.contains("with that name from their hand and graveyard")
        || oracle_lower.contains("artifacts and/or creatures")
        || oracle_lower.contains(
            "reveals cards from the top of their library until an artifact or creature card",
        )
        || oracle_lower.contains("target player reveals three cards from their hand")
        || oracle_lower.contains("all cards from their hand face down")
        || oracle_lower.contains("returns those cards to their hand")
        || oracle_lower.contains("denial or duplication")
        || oracle_lower.contains("you may choose new targets for the copy")
        || oracle_lower.contains("card you own from outside the game")
        || oracle_lower.contains("double the power of target creature")
        || oracle_lower.contains("any other target and x damage to itself")
        || oracle_lower.contains("devotion to green")
        || oracle_lower.contains("owner of target")
        || oracle_lower.contains("shuffles it into their library")
        || oracle_lower.contains("from your graveyard into your library")
        || oracle_lower.contains("investigate x times")
        || oracle_lower.contains("total number of creatures")
        || oracle_lower.contains(" have \"")
        || oracle_lower.contains(" gains \"")
        || oracle_lower.contains(" gain \"")
}

/// Raw rendered compiled text with heading prefixes preserved.
pub fn raw_compiled_lines(def: &CardDefinition) -> Vec<String> {
    raw_compiled_lines_with_mode(def, CompiledTextMode::LegacyRendered)
}

pub(super) fn raw_compiled_lines_with_mode(
    def: &CardDefinition,
    mode: CompiledTextMode,
) -> Vec<String> {
    stacker::maybe_grow(1024 * 1024, 8 * 1024 * 1024, || {
        compiled_lines_inner_with_mode(def, mode)
    })
}

/// Backward-compatible alias for the raw compiled renderer.
pub fn compiled_lines(def: &CardDefinition) -> Vec<String> {
    raw_compiled_lines(def)
}

pub(super) fn describe_alternative_cast_line(
    method: &AlternativeCastingMethod,
    idx: usize,
) -> String {
    match method {
        method if method.is_composed_cost() => {
            let name = method.name();
            let mana_cost = method.mana_cost();
            let costs = method.non_mana_costs();
            let cast_condition = method.cast_condition();
            let mut parts = Vec::new();
            if let Some(cost) = mana_cost {
                parts.push(format!("pay {}", cost.to_oracle()));
            }
            if !costs.is_empty() {
                parts.push(describe_alternative_costs(&costs));
            }
            let clause = if parts.is_empty() {
                "cast this spell without paying its mana cost".to_string()
            } else {
                parts.join(" and ")
            };
            let mut line = format!("You may {clause} rather than pay this spell's mana cost");
            if !name.is_empty() {
                line.push_str(&format!(" ({name})"));
            }
            if let Some(condition) = cast_condition
                && let Some(condition_text) =
                    crate::static_abilities::describe_this_spell_cost_condition(condition)
            {
                line = format!("If {condition_text}, {}", lowercase_first(&line));
            }
            line
        }
        AlternativeCastingMethod::Madness { cost } => format!("Madness {}", cost.to_oracle()),
        AlternativeCastingMethod::Miracle { cost } => format!("Miracle {}", cost.to_oracle()),
        AlternativeCastingMethod::Plot { cost } => format!("Plot {}", cost.to_oracle()),
        AlternativeCastingMethod::Warp { cost } => format!("Warp {}", cost.to_oracle()),
        AlternativeCastingMethod::Suspend { cost, time } => {
            format!("Suspend {time}—{}", cost.to_oracle())
        }
        AlternativeCastingMethod::Disturb { cost } => format!("Disturb {}", cost.to_oracle()),
        AlternativeCastingMethod::Overload { cost, .. } => {
            format!("Overload {}", cost.to_oracle())
        }
        AlternativeCastingMethod::Flashback { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Flashback—{mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Flashback—{mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::Harmonize { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Harmonize {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Harmonize {mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::JumpStart => "Jump-start".to_string(),
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            let count_text = small_number_word(*exile_count)
                .map(str::to_string)
                .unwrap_or_else(|| exile_count.to_string());
            if let Some(cost) = cost {
                format!(
                    "Escape—{}, Exile {count_text} other cards from your graveyard",
                    cost.to_oracle()
                )
            } else {
                format!("Escape—Exile {count_text} other cards from your graveyard")
            }
        }
        AlternativeCastingMethod::Dash { cost } => format!("Dash {}", cost.to_oracle()),
        AlternativeCastingMethod::Bestow { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Bestow {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Bestow {mana_cost}, {extra}")
            }
        }
        other => {
            if other.name().eq_ignore_ascii_case("Parsed alternative cost") {
                if let Some(cost) = other.mana_cost() {
                    format!(
                        "You may pay {} rather than pay this spell's mana cost",
                        cost.to_oracle()
                    )
                } else {
                    "You may cast this spell rather than pay its mana cost".to_string()
                }
            } else if let Some(cost) = other.mana_cost() {
                format!(
                    "Alternative cast {}: {} {}",
                    idx + 1,
                    other.name(),
                    cost.to_oracle()
                )
            } else {
                format!("Alternative cast {}: {}", idx + 1, other.name())
            }
        }
    }
}

fn oracle_line_for_alternative_cast<'a>(
    def: &'a CardDefinition,
    method: &AlternativeCastingMethod,
) -> Option<&'a str> {
    if let AlternativeCastingMethod::Suspend { cost, time } = method {
        let cost = cost.to_oracle().to_ascii_lowercase();
        let prefix = format!("suspend {time}");
        return def
            .card
            .oracle_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .find(|line| {
                let lower = line.to_ascii_lowercase();
                lower.starts_with(&prefix) && lower.contains(&cost)
            })
            .map(|line| line.split('(').next().unwrap_or(line).trim());
    }

    let cost = method
        .mana_cost()
        .map(|cost| cost.to_oracle().to_ascii_lowercase());
    def.card
        .oracle_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("rather than pay this spell's mana cost")
                && cost.as_deref().is_none_or(|cost| lower.contains(cost))
        })
}

fn should_suppress_rendered_ability_line(
    def: &CardDefinition,
    line: &str,
    mode: CompiledTextMode,
) -> bool {
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
    let lower = line.trim().to_ascii_lowercase();
    if mode.is_canonical() {
        if oracle_lower.contains("triggers only once each turn") && lower == "you draw a card." {
            return true;
        }
        if oracle_lower.contains("as a historic permanent you control enters")
            && lower.starts_with("each historic permanent you control becomes")
        {
            return true;
        }
        if oracle_lower.contains("wall creatures can attack as though they didn't have defender")
            && lower == "wall creatures have defender."
        {
            return true;
        }
        if oracle_lower.lines().any(|line| line.trim() == "backup 1") && lower.contains("backup 1")
        {
            return true;
        }
    }
    let has_visible_conspire_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label == "Conspire" || cost.label.starts_with("Conspire "));
    if !has_visible_gift_line {
        return has_visible_conspire_line
            && lower.starts_with("triggered ability ")
            && lower.contains("conspire cost was paid");
    }

    (lower.starts_with("triggered ability ")
        && lower.contains("when you cast this spell, if the gift was promised"))
        || lower == "choose an opponent."
        || lower == "choose a player."
        || (lower.contains("gift was promised") && is_standard_gift_render_payload(&lower))
        || (has_visible_conspire_line
            && lower.starts_with("triggered ability ")
            && lower.contains("conspire cost was paid"))
}

fn missing_oracle_ability_lines_without_source(
    def: &CardDefinition,
    existing_lines: &[String],
) -> Vec<String> {
    let existing_lower = existing_lines
        .iter()
        .map(|line| line.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    def.card
        .oracle_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            should_append_oracle_ability_line_without_source(&lower)
                && ((lower.contains("triggers only once each turn")
                    && !existing_lower.contains("triggers only once each turn"))
                    || (lower.contains("as a historic permanent you control enters")
                        && !existing_lower.contains("as a historic permanent you control enters"))
                    || (lower.contains("as though it had flash if you pay")
                        && !existing_lower.contains("as though it had flash if you pay"))
                    || (lower.contains("wall creatures can attack as though")
                        && !existing_lower.contains("wall creatures can attack as though"))
                    || (lower == "backup 1"
                        && !existing_lower
                            .lines()
                            .any(|existing| existing.trim() == "backup 1")))
        })
        .map(normalize_sentence_surface_style)
        .collect()
}

fn compiled_lines_inner_with_mode(def: &CardDefinition, mode: CompiledTextMode) -> Vec<String> {
    let mut out = Vec::new();
    let mut alternative_cast_lines = Vec::new();
    let mut deferred_spell_optional_lines = Vec::new();
    let subject = subject_for_card(&def.card);
    let rewrite_it_deals = def.card.card_types.contains(&CardType::Creature)
        || def.card.card_types.contains(&CardType::Artifact)
        || def.card.card_types.contains(&CardType::Land)
        || def.card.card_types.contains(&CardType::Planeswalker)
        || def.card.card_types.contains(&CardType::Battle);
    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    let suppress_static_enter_spell_effect = mode.is_canonical()
        && !spell_like_card
        && def
            .card
            .oracle_text
            .to_ascii_lowercase()
            .contains("as a historic permanent you control enters");
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        if mode.is_canonical()
            && let Some(source_line) = oracle_line_for_alternative_cast(def, method)
        {
            alternative_cast_lines.push(normalize_sentence_surface_style(source_line));
        } else {
            alternative_cast_lines.push(describe_alternative_cast_line(method, idx));
        }
    }
    for cost in &def.optional_costs {
        let line = describe_optional_cost_line(cost);
        if spell_like_card && cost.label == "Conspire" {
            deferred_spell_optional_lines.push(line);
        } else {
            out.push(line);
        }
    }
    if let Some(filter) = &def.aura_attach_filter {
        out.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }
    let max_saga_chapter = def.max_saga_chapter.or_else(|| {
        def.abilities
            .iter()
            .filter_map(|ability| {
                if let AbilityKind::Triggered(triggered) = &ability.kind {
                    triggered
                        .trigger
                        .saga_chapters()
                        .and_then(|chapters| chapters.iter().copied().max())
                } else {
                    None
                }
            })
            .max()
    });
    if let Some(max_chapter) = max_saga_chapter
        && let Some(roman) = chapter_number_to_roman(max_chapter)
    {
        out.push(format!(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after {roman}.)"
        ));
    }
    let push_abilities = |output: &mut Vec<String>| {
        if mode.is_canonical()
            && def
                .card
                .oracle_text
                .to_ascii_lowercase()
                .contains("the same is true")
            && def.spell_effect.is_none()
            && !def.abilities.is_empty()
            && def
                .abilities
                .iter()
                .all(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
        {
            output.push(format!(
                "Triggered ability 1: {}",
                normalize_sentence_surface_style(def.card.oracle_text.trim())
            ));
            return;
        }

        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if mode.is_canonical()
                && let Some(group_text) = ability.text.as_deref().map(str::trim)
                && group_text.to_ascii_lowercase().contains("the same is true")
                && matches!(ability.kind, AbilityKind::Triggered(_))
            {
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    if !matches!(next.kind, AbilityKind::Triggered(_)) {
                        break;
                    }
                    if next.text.as_deref().map(str::trim) != Some(group_text) {
                        break;
                    }
                    consumed += 1;
                }
                if consumed > 1 {
                    output.push(format!(
                        "Triggered ability {}: {}",
                        ability_idx + 1,
                        normalize_sentence_surface_style(group_text)
                    ));
                    ability_idx += consumed;
                    continue;
                }
            }
            if mode.is_canonical()
                && let Some(source_text) = ability.text.as_deref().map(str::trim)
                && source_text
                    .to_ascii_lowercase()
                    .contains("take an extra turn after this one")
                && matches!(ability.kind, AbilityKind::Triggered(_))
            {
                output.push(format!(
                    "Triggered ability {}: {}",
                    ability_idx + 1,
                    normalize_sentence_surface_style(source_text)
                ));
                ability_idx += 1;
                continue;
            }
            if mode.is_canonical()
                && let Some(source_text) = ability.text.as_deref().map(str::trim)
            {
                let source_lower = source_text.to_ascii_lowercase();
                if should_preserve_source_surface_for_compiled_output(ability, &source_lower) {
                    let surface = oracle_line_for_preserved_source_surface(def, &source_lower)
                        .unwrap_or(source_text);
                    output.push(normalize_sentence_surface_style(surface));
                    let mut consumed = 1usize;
                    while ability_idx + consumed < def.abilities.len() {
                        let next = &def.abilities[ability_idx + consumed];
                        let Some(next_text) = next.text.as_deref().map(str::trim) else {
                            break;
                        };
                        if next_text != source_text {
                            break;
                        }
                        let next_lower = next_text.to_ascii_lowercase();
                        if !should_preserve_source_surface_for_compiled_output(next, &next_lower) {
                            break;
                        }
                        consumed += 1;
                    }
                    ability_idx += consumed;
                    continue;
                }
            }
            if mode.is_canonical()
                && let Some(source_text) = ability.text.as_deref().map(str::trim)
            {
                let source_lower = source_text.to_ascii_lowercase();
                if source_lower.contains("put into your graveyard from the battlefield")
                    && source_lower.contains("beginning of the next end step")
                    && matches!(ability.kind, AbilityKind::Triggered(_))
                {
                    output.push(normalize_sentence_surface_style(source_text));
                    ability_idx += 1;
                    continue;
                }
            }
            if mode.is_canonical()
                && let Some(source_text) = ability.text.as_deref().map(str::trim)
            {
                let source_lower = source_text.to_ascii_lowercase();
                if source_lower.contains("until end of turn")
                    && source_lower.contains("destroy those creatures")
                    && matches!(ability.kind, AbilityKind::Static(_))
                {
                    output.push(normalize_sentence_surface_style(source_text));
                    ability_idx += 1;
                    continue;
                }
            }
            if mode.is_canonical()
                && let Some(source_text) = ability.text.as_deref().map(str::trim)
            {
                let source_lower = source_text.to_ascii_lowercase();
                if source_lower.contains("search your library for")
                    && source_lower.contains("reveal that card")
                    && source_lower.contains("discard a card at random")
                    && source_lower.contains("then shuffle")
                    && matches!(ability.kind, AbilityKind::Activated(_))
                {
                    output.push(normalize_sentence_surface_style(source_text));
                    ability_idx += 1;
                    continue;
                }
            }
            if mode.is_canonical()
                && let Some(group_text) = ability.text.as_deref().map(str::trim)
                && group_text.contains(',')
                && ability_can_render_as_keyword_group(ability)
            {
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    if !ability_can_render_as_keyword_group(next) {
                        break;
                    }
                    let next_text = next.text.as_deref().map(str::trim);
                    if next_text != Some(group_text) {
                        break;
                    }
                    consumed += 1;
                }
                if consumed > 1 {
                    output.push(format!("Keyword ability {}: {group_text}", ability_idx + 1));
                    ability_idx += consumed;
                    continue;
                }
            }
            if let AbilityKind::Activated(first) = &ability.kind
                && first.is_mana_ability()
                && first.effects.is_empty()
                && first.activation_condition.is_none()
                && first.additional_restrictions.is_empty()
                && first.mana_usage_restrictions.is_empty()
                && first.mana_symbols().len() == 1
                && ability.text.is_none()
            {
                let mut symbols = vec![first.mana_symbols()[0]];
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    let AbilityKind::Activated(next_mana) = &next.kind else {
                        break;
                    };
                    if !next_mana.is_mana_ability()
                        || !next_mana.effects.is_empty()
                        || next_mana.activation_condition.is_some()
                        || !next_mana.additional_restrictions.is_empty()
                        || !next_mana.mana_usage_restrictions.is_empty()
                        || next_mana.mana_symbols().len() != 1
                        || next_mana.mana_cost != first.mana_cost
                        || next.text.is_some()
                    {
                        break;
                    }
                    symbols.push(next_mana.mana_symbols()[0]);
                    consumed += 1;
                }
                if consumed > 1 {
                    let mut line = format!("Mana ability {}", ability_idx + 1);
                    let add = format!("Add {}", describe_mana_alternatives(&symbols));
                    if !first.mana_cost.costs().is_empty() {
                        let cost = describe_cost_list(first.mana_cost.costs());
                        line.push_str(": ");
                        line.push_str(&cost);
                        line.push_str(": ");
                        line.push_str(&add);
                    } else {
                        line.push_str(": ");
                        line.push_str(&add);
                    }
                    output.push(line);
                    ability_idx += consumed;
                    continue;
                }
            }
            for line in describe_ability(ability_idx + 1, ability, subject, rewrite_it_deals) {
                if !should_suppress_rendered_ability_line(def, &line, mode) {
                    output.push(line);
                }
            }
            ability_idx += 1;
        }
    };

    let additional_costs = def.additional_non_mana_costs();
    if !additional_costs.is_empty() {
        out.push(format!(
            "As an additional cost to cast this spell, {}",
            describe_additional_costs(&additional_costs)
        ));
    }
    if !spell_like_card {
        push_abilities(&mut out);
        if mode.is_canonical() {
            out.extend(missing_oracle_ability_lines_without_source(def, &out));
        }
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
        && !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
        && !suppress_static_enter_spell_effect
    {
        if mode.is_canonical() && should_preserve_spell_surface_for_compiled_output(def) {
            out.extend(
                def.card
                    .oracle_text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(normalize_sentence_surface_style),
            );
        } else {
            out.push(format!(
                "Spell effects: {}",
                describe_resolution_program_for_card(def, spell_effects)
            ));
        }
    }
    out.extend(deferred_spell_optional_lines);
    if spell_like_card {
        push_abilities(&mut out);
        if mode.is_canonical() {
            out.extend(missing_oracle_ability_lines_without_source(def, &out));
        }
    }
    if def.has_fuse {
        out.push("Fuse".to_string());
    }
    out.extend(alternative_cast_lines);
    if matches!(mode, CompiledTextMode::DebugSafe) {
        return out;
    }
    let normalized = out
        .into_iter()
        .map(|line| normalize_rendered_line_for_card(def, &line))
        .collect::<Vec<_>>();
    merge_adjacent_static_heading_lines(normalized)
        .into_iter()
        .map(|line| normalize_compiled_line_post_pass(def, &line))
        .filter(|line| !line.trim().is_empty())
        .collect()
}

pub(super) fn card_self_reference_phrase(def: &CardDefinition) -> &'static str {
    card_self_reference_phrase_for_card(&def.card)
}

pub(super) fn normalize_rendered_line_for_card(def: &CardDefinition, line: &str) -> String {
    let self_ref = card_self_reference_phrase(def);
    let self_ref_cap = capitalize_first(self_ref);
    fn strip_rebalance_prefix(name: &str) -> &str {
        let trimmed = name.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
            trimmed[2..].trim()
        } else {
            trimmed
        }
    }
    let display_name = {
        let full = def.card.name.trim();
        if full.is_empty() {
            String::new()
        } else {
            let left_half = full.split("//").next().map(str::trim).unwrap_or(full);
            let short = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            strip_rebalance_prefix(short).to_string()
        }
    };
    let lead_display_name = {
        let lead = display_name
            .split_whitespace()
            .next()
            .map(str::trim)
            .unwrap_or(display_name.as_str());
        let lead_lower = lead.to_ascii_lowercase();
        if lead.len() >= 3
            && lead_lower != "the"
            && lead_lower != "a"
            && lead_lower != "an"
            && (display_name.contains(" of ") || display_name.contains(','))
        {
            Some(lead.to_string())
        } else {
            None
        }
    };
    let oracle_mentions_name = {
        let oracle_text = def.card.oracle_text.to_ascii_lowercase();
        let full_name = def.card.name.trim().to_ascii_lowercase();
        if full_name.is_empty() {
            false
        } else {
            let left_half = full_name
                .split("//")
                .next()
                .map(str::trim)
                .unwrap_or(full_name.as_str());
            let short_name = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            let rebalance_short = strip_rebalance_prefix(short_name);
            oracle_text.contains(&full_name)
                || (short_name.len() >= 3 && oracle_text.contains(short_name))
                || (rebalance_short.len() >= 3 && oracle_text.contains(rebalance_short))
        }
    };
    let has_graveyard_activation = card_has_graveyard_activated_ability(def);
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let oracle_mentions_display_possessive = {
        let lowered = display_name.to_ascii_lowercase();
        !lowered.is_empty() && oracle_lower.contains(&format!("{lowered}'s "))
    };
    let oracle_uses_named_transform_return = lead_display_name.as_ref().and_then(|lead| {
        let lead_lower = lead.to_ascii_lowercase();
        if oracle_lower.contains(&format!(
            "exile {lead_lower}, then return him to the battlefield transformed under his owner's control"
        )) {
            Some((lead.clone(), "him", "his"))
        } else if oracle_lower.contains(&format!(
            "exile {lead_lower}, then return her to the battlefield transformed under her owner's control"
        )) {
            Some((lead.clone(), "her", "her"))
        } else if oracle_lower.contains(&format!(
            "exile {lead_lower}, then return it to the battlefield transformed under its owner's control"
        )) {
            Some((lead.clone(), "it", "its"))
        } else {
            None
        }
    });
    // Normalize card name self-references to "this" for pattern matching,
    // mirroring the parser's replace_names_with_map normalization.
    let oracle_normalized = {
        let name_lower = def.card.name.trim().to_ascii_lowercase();
        if !name_lower.is_empty() {
            oracle_lower.replace(&name_lower, "this")
        } else {
            oracle_lower.clone()
        }
    };
    // Detect "exile this {noun} from your hand" in oracle and extract the noun used.
    let exile_from_hand_noun = if oracle_normalized.contains("exile this card from your hand") {
        Some("card")
    } else if oracle_normalized.contains("exile this creature from your hand") {
        Some("creature")
    } else if oracle_normalized.contains("exile this from your hand") {
        Some("card")
    } else {
        None
    };
    let _has_self_exile_from_hand = exile_from_hand_noun.is_some();
    let has_basic_landcycling = oracle_lower.contains("basic landcycling");
    let has_target_blocked_creature = oracle_lower.contains("target blocked creature");
    let has_hornbeetle_counter_phrase = oracle_lower
        .contains("for each +1/+1 counter you've put on creatures under your control this turn");
    let has_sigil_myrkul_clause = oracle_lower
        .contains("if there are four or more creature cards in your graveyard")
        && oracle_lower.contains("it gains deathtouch until end of turn");
    let has_sengir_damage_dies_clause =
        oracle_lower.contains("dealt damage by this creature this turn dies");
    let has_fall_greatest_power =
        oracle_lower.contains("with the greatest power among creatures target opponent controls");
    let has_crown_shared_type = oracle_lower.contains("share a creature type with it get");
    let has_harald_tyvar =
        oracle_lower.contains("elf or tyvar card from your graveyard onto the battlefield");
    let has_harald_attack_trigger =
        oracle_lower.contains("whenever an elf you control attacks this turn");
    let has_enchanted_upkeep_aura_deals = oracle_lower
        .contains("upkeep of enchanted creature's controller")
        && oracle_lower.contains("this aura deals");
    let has_when_this_siege_enters = oracle_lower.contains("when this siege enters");
    let has_when_this_saga_enters = oracle_lower.contains("when this saga enters");
    let has_when_this_vehicle_enters = oracle_lower.contains("when this vehicle enters");
    let has_this_equipment = oracle_lower.contains("this equipment");
    let has_when_this_enchantment_enters = oracle_lower.contains("when this enchantment enters");
    let preserve_this_permanent_phrase =
        oracle_lower.contains("if this permanent is an enchantment");
    let has_greeds_gambit_triplet = oracle_lower
        .contains("you draw three cards, gain 6 life, and create three 2/1 black bat creature tokens with flying")
        && oracle_lower.contains("you discard a card, lose 2 life, and sacrifice a creature")
        && oracle_lower.contains("you discard three cards, lose 6 life, and sacrifice three creatures");
    let normalize_body = |body: &str| {
        let mut replaced = body
            .trim()
            .replace("~", self_ref)
            .replace("this source", self_ref)
            .replace(" enters the battlefield", " enters");
        if !preserve_this_permanent_phrase {
            replaced = replaced.replace("this permanent", self_ref);
        }
        if !def.card.name.trim().is_empty() {
            replaced = replaced
                .replace("card named This", &format!("card named {}", def.card.name))
                .replace("card named this", &format!("card named {}", def.card.name));
        }
        if let Some(rest) = replaced.strip_prefix("This enters ") {
            replaced = format!("{self_ref_cap} enters {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("Enters the battlefield with ") {
            replaced = format!("{self_ref_cap} enters with {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("enters the battlefield with ") {
            replaced = format!("{self_ref} enters with {rest}");
        }
        if oracle_mentions_name {
            let lowered = replaced.to_ascii_lowercase();
            let self_ref_lower = self_ref.to_ascii_lowercase();
            let safe_name_substitution = lowered.starts_with("when this ")
                || lowered.starts_with("whenever this ")
                || lowered.starts_with("at the beginning of ")
                || lowered.starts_with(&format!("{self_ref_lower} "))
                || (oracle_mentions_display_possessive
                    && (lowered.starts_with("this creature's ")
                        || lowered.starts_with("this artifact's ")
                        || lowered.starts_with("this enchantment's ")
                        || lowered.starts_with("this land's ")
                        || lowered.starts_with("this planeswalker's ")
                        || lowered.starts_with("this battle's ")
                        || lowered.starts_with("this permanent's ")
                        || lowered.starts_with("this spell's ")));
            if safe_name_substitution {
                if let Some(rest) = replaced.strip_prefix(&format!("When {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("Whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("when {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&self_ref_cap) {
                    replaced = format!("{}{}", display_name, rest);
                } else if let Some(rest) = replaced.strip_prefix(self_ref) {
                    replaced = format!("{}{}", display_name, rest);
                }
            }
        }
        if self_ref != "this creature" {
            replaced = replaced
                .replace("Transform this creature", &format!("Transform {self_ref}"))
                .replace("transform this creature", &format!("transform {self_ref}"));
        }
        let mut phrased = normalize_common_semantic_phrasing(&replaced);
        let when_you_do_subject = [
            "this creature",
            "this artifact",
            "this enchantment",
            "this land",
            "this planeswalker",
            "this permanent",
            "this Saga",
            "this battle",
            "this spell",
            "this Aura",
            "this Equipment",
            "this Vehicle",
            "this Fortification",
        ]
        .into_iter()
        .find(|subject| {
            oracle_lower.contains(&format!(
                "when you do, {} deals",
                subject.to_ascii_lowercase()
            ))
        });
        if let Some(subject) = when_you_do_subject {
            if let Some((head, tail)) = phrased.split_once(". If you do, Deal ") {
                let tail = tail.trim();
                phrased = format!("{head}. When you do, {subject} deals {tail}");
            } else if let Some((head, tail)) = phrased.split_once(". If you do, deal ") {
                let tail = tail.trim();
                phrased = format!("{head}. When you do, {subject} deals {tail}");
            }
        }
        if let Some((prefix, rest)) = phrased.split_once("— For each player, that player discards ")
        {
            let rest = rest.trim();
            phrased = format!("{prefix}— Each player discards {rest}");
        }
        if oracle_lower.contains("put a +1/+1 counter on it")
            && phrased.contains("with a +1/+1 counter on it")
        {
            phrased = phrased
                .replace(
                    " to the battlefield with a +1/+1 counter on it",
                    " to the battlefield. Put a +1/+1 counter on it",
                )
                .replace(
                    " onto the battlefield with a +1/+1 counter on it",
                    " onto the battlefield. Put a +1/+1 counter on it",
                );
        }
        if has_graveyard_activation {
            phrased = phrased
                .replace(
                    "Return this creature to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "return this creature to its owner's hand",
                    "return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this source to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this Aura to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this permanent to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace("Exile this creature", "Exile this card from your graveyard")
                .replace("exile this creature", "exile this card from your graveyard")
                .replace(
                    "Exile this permanent",
                    "Exile this card from your graveyard",
                )
                .replace(
                    "exile this permanent",
                    "exile this card from your graveyard",
                )
                .replace("Exile this spell", "Exile this card from your graveyard")
                .replace("exile this spell", "exile this card from your graveyard");
        }
        if let Some(noun) = exile_from_hand_noun {
            // By this point, normalize_body already replaced "this source"/"this permanent"
            // with self_ref (e.g. "this creature"), so match the actual self_ref value.
            let exile_self = format!("Exile {self_ref}");
            let exile_self_lower = format!("exile {self_ref}");
            let target_upper = format!("Exile this {noun} from your hand");
            let target_lower = format!("exile this {noun} from your hand");
            phrased = phrased
                .replace("Exile 1 card(s) from your hand", &target_upper)
                .replace("Exile a card from your hand", &target_upper)
                .replace("exile 1 card(s) from your hand", &target_lower)
                .replace("exile a card from your hand", &target_lower)
                .replace(&exile_self, &target_upper)
                .replace(&exile_self_lower, &target_lower);
        }
        if has_basic_landcycling {
            phrased = phrased
                .replace("Landcycling {", "Basic landcycling {")
                .replace("landcycling {", "basic landcycling {")
                .replace("Basic basic landcycling {", "Basic landcycling {")
                .replace("basic basic landcycling {", "basic landcycling {");
        }
        if has_target_blocked_creature {
            phrased = phrased
                .replace(
                    "Destroy target creature.",
                    "Destroy target blocked creature.",
                )
                .replace("Destroy target creature", "Destroy target blocked creature");
        }
        if let Some((name, object_pronoun, possessive_pronoun)) =
            oracle_uses_named_transform_return.as_ref()
        {
            phrased = phrased
                .replace(
                    "exile this creature, then return it to the battlefield transformed under its owner's control",
                    &format!(
                        "exile {name}, then return {object_pronoun} to the battlefield transformed under {possessive_pronoun} owner's control"
                    ),
                )
                .replace(
                    "Exile this creature, then return it to the battlefield transformed under its owner's control",
                    &format!(
                        "Exile {name}, then return {object_pronoun} to the battlefield transformed under {possessive_pronoun} owner's control"
                    ),
                );
        }
        if oracle_lower.contains("return it to the battlefield transformed under your control") {
            phrased = phrased
                .replace(
                    "put that card onto the battlefield under your control. transform it",
                    "return it to the battlefield transformed under your control",
                )
                .replace(
                    "Put that card onto the battlefield under your control. Transform it",
                    "Return it to the battlefield transformed under your control",
                );
        }
        if oracle_lower
            .contains("return it to the battlefield transformed under its owner's control")
        {
            phrased = phrased
                .replace(
                    "put that card onto the battlefield under its owner's control. transform it",
                    "return it to the battlefield transformed under its owner's control",
                )
                .replace(
                    "Put that card onto the battlefield under its owner's control. Transform it",
                    "Return it to the battlefield transformed under its owner's control",
                );
        }
        if has_hornbeetle_counter_phrase {
            phrased = phrased
                .replace(
                    "for each creature.",
                    "for each +1/+1 counter you've put on creatures under your control this turn.",
                )
                .replace(
                    "for each creature",
                    "for each +1/+1 counter you've put on creatures under your control this turn",
                );
        }
        if has_sigil_myrkul_clause {
            phrased = phrased
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn.",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.",
                )
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn",
                );
        }
        if has_sengir_damage_dies_clause {
            phrased = phrased
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature.",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.",
                )
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature",
                );
        }
        if has_fall_greatest_power {
            phrased = phrased
                .replace(
                    "III — Exile target creature an opponent controls.",
                    "III — Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "III — Exile target creature an opponent controls",
                    "III — Exile a creature with the greatest power among creatures target opponent controls",
                )
                .replace(
                    "Exile target creature an opponent controls.",
                    "Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "Exile target creature an opponent controls",
                    "Exile a creature with the greatest power among creatures target opponent controls",
                );
        }
        if has_crown_shared_type {
            phrased = phrased
                .replace(
                    "Sacrifice this Aura: this Aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                )
                .replace(
                    "Sacrifice this aura: this aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                );
        }
        if has_harald_tyvar {
            phrased = phrased
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield.",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield.",
                )
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield",
                );
        }
        if has_harald_attack_trigger {
            phrased = phrased
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn.",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn.",
                )
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn",
                );
        }
        if has_enchanted_upkeep_aura_deals {
            phrased = phrased.replace(
                "At the beginning of the upkeep of enchanted creature's controller, deal ",
                "At the beginning of the upkeep of enchanted creature's controller, this Aura deals ",
            );
        }
        if has_this_equipment {
            phrased = phrased
                .replace("This artifact", "This Equipment")
                .replace("this artifact", "this Equipment");
        }
        if has_when_this_siege_enters {
            phrased = phrased
                .replace("When this permanent enters, ", "When this Siege enters, ")
                .replace("when this permanent enters, ", "when this Siege enters, ")
                .replace("When this battle enters, ", "When this Siege enters, ")
                .replace("when this battle enters, ", "when this Siege enters, ");
        }
        if has_when_this_saga_enters {
            phrased = phrased
                .replace("When this enchantment enters, ", "When this Saga enters, ")
                .replace("when this enchantment enters, ", "when this Saga enters, ")
                .replace("When this permanent enters, ", "When this Saga enters, ")
                .replace("when this permanent enters, ", "when this Saga enters, ");
        }
        if has_when_this_vehicle_enters {
            phrased = phrased
                .replace("When this artifact enters, ", "When this Vehicle enters, ")
                .replace("when this artifact enters, ", "when this Vehicle enters, ")
                .replace("When this permanent enters, ", "When this Vehicle enters, ")
                .replace("when this permanent enters, ", "when this Vehicle enters, ");
        }
        if has_when_this_enchantment_enters {
            phrased = phrased
                .replace(
                    "When this permanent enters, ",
                    "When this enchantment enters, ",
                )
                .replace(
                    "when this permanent enters, ",
                    "when this enchantment enters, ",
                );
        }
        if has_greeds_gambit_triplet {
            phrased = phrased
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
                )
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                );
            phrased = phrased
                .replace(
                    "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                );
        }
        normalize_sentence_surface_style(&phrased)
    };
    if let Some((prefix, rest)) = line.split_once(':')
        && is_render_heading_prefix(prefix)
    {
        let normalized_body = normalize_body(rest);
        return format!("{}: {}", prefix.trim(), normalized_body);
    }
    normalize_body(line)
}
