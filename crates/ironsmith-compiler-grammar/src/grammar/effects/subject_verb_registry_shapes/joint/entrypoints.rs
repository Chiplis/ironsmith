use super::*;

pub fn parse_joint_draw_shape(tokens: &[OwnedLexToken]) -> Option<JointDrawShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, joint_draw, "registry-joint-draw")
}

pub fn parse_joint_life_shape(tokens: &[OwnedLexToken]) -> Option<JointLifeShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, joint_life, "registry-joint-life")
}

pub fn parse_joint_create_shape(tokens: &[OwnedLexToken]) -> Option<JointCreateShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, joint_create, "registry-joint-create")
}

pub fn parse_joint_sacrifice_shape(tokens: &[OwnedLexToken]) -> Option<JointSacrificeShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, joint_sacrifice, "registry-joint-sacrifice")
}
