use super::*;

pub fn parse_joint_draw_shape(tokens: &[OwnedLexToken]) -> Option<JointDrawShape<'_>> {
    primitives::parse_all(tokens, joint_draw, "registry-joint-draw").ok()
}

pub fn parse_joint_life_shape(tokens: &[OwnedLexToken]) -> Option<JointLifeShape<'_>> {
    primitives::parse_all(tokens, joint_life, "registry-joint-life").ok()
}

pub fn parse_joint_create_shape(tokens: &[OwnedLexToken]) -> Option<JointCreateShape<'_>> {
    primitives::parse_all(tokens, joint_create, "registry-joint-create").ok()
}

pub fn parse_joint_sacrifice_shape(tokens: &[OwnedLexToken]) -> Option<JointSacrificeShape<'_>> {
    primitives::parse_all(tokens, joint_sacrifice, "registry-joint-sacrifice").ok()
}
