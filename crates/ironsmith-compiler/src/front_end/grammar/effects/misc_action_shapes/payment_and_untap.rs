use winnow::combinator::alt;
use winnow::prelude::*;

use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::mana::ManaSymbol;

use super::super::super::{leaf, permission_shapes, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedXMaximumShape {
    TriggeringLifeGained,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedXPaymentShape {
    pub cost: crate::mana::ManaCost,
    pub maximum: BoundedXMaximumShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UntapActionShape<'a> {
    All {
        filter_tokens: &'a [OwnedLexToken],
    },
    Tagged {
        filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    Explicit {
        target_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConjoinedUntapAllShape<'a> {
    pub left_filter_tokens: &'a [OwnedLexToken],
    pub right_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatedTaggedManaPayment {
    pub pip_groups: Vec<Vec<ManaSymbol>>,
}

#[cfg(test)]
#[path = "payment_and_untap_inline_tests.rs"]
mod tests;

#[path = "payment_and_untap/reference_programs.rs"]
mod reference_programs;
pub use reference_programs::parse_repeated_tagged_mana_payment_tokens;
#[path = "payment_and_untap/choice_programs.rs"]
mod choice_programs;
pub use choice_programs::parse_chosen_object_set_filter_tokens;
#[path = "payment_and_untap/object_action_programs.rs"]
mod object_action_programs;
pub use object_action_programs::{parse_conjoined_untap_all_tokens, parse_untap_action_tokens};
#[path = "payment_and_untap/resource_programs.rs"]
mod resource_programs;
pub use resource_programs::parse_bounded_x_payment_tokens;
