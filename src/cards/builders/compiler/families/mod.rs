#![allow(dead_code, unused_imports)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineFamilyDescriptor {
    pub(crate) name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SentenceRuleFamilyDescriptor {
    pub(crate) name: &'static str,
}

pub(crate) use super::activation_and_restrictions;
pub(crate) use super::activation_helpers;
pub(crate) use super::clause_support;
pub(crate) use super::keyword_families;
pub(crate) use super::keyword_registry;
pub(crate) use super::keyword_static;
pub(crate) use super::keyword_static_helpers;
pub(crate) use super::modal_helpers;
pub(crate) use super::object_filters;
pub(crate) use super::permission_helpers;
pub(crate) use super::restriction_support;
pub(crate) use super::static_ability_helpers;

pub(crate) mod activated {
    pub(crate) const DESCRIPTOR: super::LineFamilyDescriptor =
        super::LineFamilyDescriptor { name: "activated" };
}

pub(crate) mod keywords {
    pub(crate) const DESCRIPTOR: super::LineFamilyDescriptor =
        super::LineFamilyDescriptor { name: "keywords" };
}

pub(crate) mod statements {
    pub(crate) const DESCRIPTOR: super::LineFamilyDescriptor =
        super::LineFamilyDescriptor { name: "statements" };
}

pub(crate) mod static_abilities {
    pub(crate) const DESCRIPTOR: super::LineFamilyDescriptor =
        super::LineFamilyDescriptor { name: "static" };
}

pub(crate) mod triggered {
    pub(crate) const DESCRIPTOR: super::LineFamilyDescriptor =
        super::LineFamilyDescriptor { name: "triggered" };
}
