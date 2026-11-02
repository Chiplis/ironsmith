//! "When this permanent transforms" trigger.

use crate::events::EventKind;
use crate::events::other::TransformedEvent;
use crate::target::SourceReferenceSurface;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TransformsTrigger {
    pub this_object_surface: Option<SourceReferenceSurface>,
    pub destination_name: Option<String>,
    /// Watch every permanent matching this filter instead of only the
    /// trigger's source ("Whenever a permanent you control transforms").
    pub permanent_filter: Option<crate::target::ObjectFilter>,
    pub destination_filter: Option<crate::target::ObjectFilter>,
    pub activation_label: Option<String>,
    pub activation_source_must_match: bool,
    pub activation_player: Option<crate::PlayerFilter>,
}

impl TransformsTrigger {
    pub fn new() -> Self {
        Self {
            this_object_surface: None,
            destination_name: None,
            permanent_filter: None,
            destination_filter: None,
            activation_label: None,
            activation_source_must_match: false,
            activation_player: None,
        }
    }

    pub fn permanent_filter(mut self, filter: crate::target::ObjectFilter) -> Self {
        self.permanent_filter = Some(filter);
        self
    }

    pub fn destination_filter(mut self, filter: crate::target::ObjectFilter) -> Self {
        self.destination_filter = Some(filter);
        self
    }

    pub fn this_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.this_object_surface = Some(surface);
        self
    }

    pub fn destination_name(mut self, destination_name: Option<String>) -> Self {
        self.destination_name = destination_name;
        self
    }

    fn watches_transforms_into(&self) -> bool {
        self.destination_name.is_some() || self.destination_filter.is_some()
    }

    pub(crate) fn this_subject_text(&self) -> String {
        match &self.this_object_surface {
            Some(surface) => surface.display_text(),
            None => "this creature".to_string(),
        }
    }

    pub(crate) fn destination_text(&self) -> String {
        self.destination_name
            .clone()
            .unwrap_or_else(|| self.this_subject_text())
    }
}

impl Default for TransformsTrigger {
    fn default() -> Self {
        Self::new()
    }
}

fn destination_is_self_reference(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "this" || lower.starts_with("this ")
}

impl TriggerMatcher for TransformsTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        // CR 701.27e: a "transforms into" ability also triggers when the
        // object converts (CR 701.28a); a bare "transforms" ability doesn't.
        let e = match event.kind() {
            EventKind::Transformed => match event.downcast::<TransformedEvent>() {
                Some(e) => e.clone(),
                None => return false,
            },
            EventKind::Converted if self.watches_transforms_into() => {
                match event.downcast::<crate::events::other::ConvertedEvent>() {
                    Some(e) => TransformedEvent {
                        snapshot: e.snapshot.clone(),
                        ..TransformedEvent::new(e.permanent)
                    },
                    None => return false,
                }
            }
            _ => return false,
        };
        let e = &e;
        if let Some(label) = &self.activation_label {
            let Some(cause) = &e.cause else {return false;};
            if cause.activation_definition.is_none() || cause.activation_label.as_ref() != Some(label)
                || cause.resolving_ability != Some(ironsmith_core::CauseAbilityKind::Activated) {return false;}
            if self.activation_source_must_match && cause.source != Some(e.permanent) { return false; }
            if let Some(player) = &self.activation_player {
                use crate::filter::PlayerFilterExt as _;
                if cause.source_controller.is_none_or(|actor| !player.matches_player(actor,&ctx.filter_ctx)) { return false; }
            }
        }
        if let Some(filter) = &self.permanent_filter {
            let matches = |snapshot| {
                super::permanent_lifecycle::matches_completed(filter, snapshot, ctx)
                    && self.destination_filter.as_ref().is_none_or(|destination| {
                        super::permanent_lifecycle::matches_completed(destination, snapshot, ctx)
                    })
            };
            return e.snapshot.as_ref().is_some_and(matches);
        }
        if e.permanent != ctx.source_id {
            return false;
        }
        // "transforms into [this card's own name]" is compiled as a
        // self-reference ("this creature"): the face that has this ability.
        // Triggers are checked against the transformed permanent's current
        // abilities, so reaching this check already means it transformed
        // into that face.
        if let Some(destination_name) = self
            .destination_name
            .as_ref()
            .filter(|name| !destination_is_self_reference(name))
        {
            return e
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.name == *destination_name)
                .unwrap_or_else(|| {
                    ctx.game
                        .object(e.permanent)
                        .is_some_and(|object| object.name == *destination_name)
                });
        }
        true
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        match event.kind() {
            EventKind::Transformed => event.downcast::<TransformedEvent>().is_some_and(|event| event.complete_source_lookback),
            EventKind::Converted if self.watches_transforms_into() => event.downcast::<crate::events::ConvertedEvent>().is_some_and(|event| event.complete_source_lookback),
            _ => false,
        }
    }

    fn uses_snapshot(&self) -> bool {
        self.permanent_filter.is_some()
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        if self.watches_transforms_into() {
            Some(vec![EventKind::Transformed, EventKind::Converted])
        } else {
            Some(vec![EventKind::Transformed])
        }
    }

    fn display(&self) -> String {
        let mut display = if let Some(filter) = &self.permanent_filter {
            format!("Whenever {} transforms{}", filter.description(),
                self.destination_filter.as_ref().map(|destination|format!(" into {}",destination.description())).unwrap_or_default())
        } else if self.destination_name.is_some() {
            format!("Whenever {} transforms into {}", self.this_subject_text(),self.destination_text())
        } else {format!("When {} transforms",self.this_subject_text())};
        if let Some(label)=&self.activation_label {
            display.push_str(&format!(" caused by {} {label} activated ability",if self.activation_source_must_match {"its"} else {"a"}));
            if self.activation_player == Some(crate::PlayerFilter::You) {display.push_str(" you activated");}
        }
        display
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    #[test]
    fn test_display() {
        let trigger = TransformsTrigger::new();
        assert!(trigger.display().contains("transforms"));
    }

    #[test]
    fn destination_name_filters_transformed_face() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(98_701), "Back Face").build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(
            TransformedEvent::new(source),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);

        assert!(
            TransformsTrigger::new()
                .destination_name(Some("Back Face".to_string()))
                .matches(&event, &ctx)
        );
        assert!(
            !TransformsTrigger::new()
                .destination_name(Some("Front Face".to_string()))
                .matches(&event, &ctx)
        );
    }
}

#[cfg(test)]
mod own_activation_cause_tests {
    use super::*;
    #[test]
    fn own_named_activation_uses_retained_cause_identity_not_another_producer_or_current_marker() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::PlayerId(0);let bob=crate::PlayerId(1);
        let card=crate::card::CardBuilder::new(crate::CardId::from_raw(98_702),"Back Face").build();
        let source=game.create_object_from_card(&card,alice,crate::Zone::Battlefield);
        let other=game.create_object_from_card(&card,alice,crate::Zone::Battlefield);
        let snapshot=crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(),&game);
        let ctx=TriggerContext::for_source(source,alice,&game);
        let mut trigger=TransformsTrigger::new().permanent_filter(crate::ObjectFilter::source());
        trigger.activation_label=Some("glimmer".into());trigger.activation_source_must_match=true;
        trigger.activation_player=Some(crate::PlayerFilter::You);
        for negative in 0..7 {
            let mut cause=crate::events::EventCause::effect();
            cause.source=Some(if negative==1 {other}else{source});cause.source_controller=Some(if negative==5 {bob}else{alice});
            cause.activation_label=Some(if negative==2 {"other"}else{"glimmer"}.into());
            cause.activation_definition=if negative==3 {None}else{Some(ironsmith_core::LinkedExileDefinition([7;32]))};
            cause.resolving_ability=Some(if negative==4 {ironsmith_core::CauseAbilityKind::Triggered}else{ironsmith_core::CauseAbilityKind::Activated});
            let event=TransformedEvent::new(source).with_snapshot(Some(snapshot.clone()));
            let event=if negative==6 {event}else{event.with_cause(cause)};
            let event=TriggerEvent::new_with_provenance(event,Default::default());
            assert_eq!(trigger.matches(&event,&ctx),negative==0,"negative={negative}");
        }
    }
}
