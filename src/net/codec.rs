use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    Eof,
    InvalidTag(u8),
    InvalidBool(u8),
    InvalidString,
    LengthTooLarge(u32),
}

const MAX_VEC_LEN: u32 = 1_000_000;

pub trait CanonicalEncode {
    fn encode(&self, out: &mut Vec<u8>);

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }
}

pub trait CanonicalDecode: Sized {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError>;
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn read_u8(input: &mut &[u8]) -> Result<u8, CodecError> {
    if input.is_empty() {
        return Err(CodecError::Eof);
    }
    let value = input[0];
    *input = &input[1..];
    Ok(value)
}

fn read_exact<const N: usize>(input: &mut &[u8]) -> Result<[u8; N], CodecError> {
    if input.len() < N {
        return Err(CodecError::Eof);
    }
    let mut buf = [0u8; N];
    buf.copy_from_slice(&input[..N]);
    *input = &input[N..];
    Ok(buf)
}

fn read_u32(input: &mut &[u8]) -> Result<u32, CodecError> {
    Ok(u32::from_be_bytes(read_exact::<4>(input)?))
}

fn read_u64(input: &mut &[u8]) -> Result<u64, CodecError> {
    Ok(u64::from_be_bytes(read_exact::<8>(input)?))
}

fn read_i32(input: &mut &[u8]) -> Result<i32, CodecError> {
    Ok(i32::from_be_bytes(read_exact::<4>(input)?))
}

fn read_len(input: &mut &[u8]) -> Result<u32, CodecError> {
    let len = read_u32(input)?;
    if len > MAX_VEC_LEN {
        return Err(CodecError::LengthTooLarge(len));
    }
    Ok(len)
}

impl CanonicalEncode for u8 {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self);
    }
}

impl CanonicalDecode for u8 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        read_u8(input)
    }
}

impl CanonicalEncode for u32 {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u32(out, *self);
    }
}

impl CanonicalDecode for u32 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        read_u32(input)
    }
}

impl CanonicalEncode for u64 {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u64(out, *self);
    }
}

impl CanonicalDecode for u64 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        read_u64(input)
    }
}

impl CanonicalEncode for i32 {
    fn encode(&self, out: &mut Vec<u8>) {
        write_i32(out, *self);
    }
}

impl CanonicalDecode for i32 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        read_i32(input)
    }
}

impl CanonicalEncode for bool {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, if *self { 1 } else { 0 });
    }
}

impl CanonicalDecode for bool {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(CodecError::InvalidBool(other)),
        }
    }
}

impl<T: CanonicalEncode> CanonicalEncode for Vec<T> {
    fn encode(&self, out: &mut Vec<u8>) {
        let len = self.len() as u32;
        write_u32(out, len);
        for item in self {
            item.encode(out);
        }
    }
}

impl<T: CanonicalDecode> CanonicalDecode for Vec<T> {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        let len = read_len(input)? as usize;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(T::decode(input)?);
        }
        Ok(items)
    }
}

impl<T: CanonicalEncode> CanonicalEncode for Option<T> {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Some(value) => {
                write_u8(out, 1);
                value.encode(out);
            }
            None => write_u8(out, 0),
        }
    }
}

impl<T: CanonicalDecode> CanonicalDecode for Option<T> {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(input)?)),
            other => Err(CodecError::InvalidBool(other)),
        }
    }
}

impl CanonicalEncode for String {
    fn encode(&self, out: &mut Vec<u8>) {
        self.as_bytes().to_vec().encode(out);
    }
}

impl CanonicalDecode for String {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        let bytes = Vec::<u8>::decode(input)?;
        String::from_utf8(bytes).map_err(|_| CodecError::InvalidString)
    }
}

impl<A: CanonicalEncode, B: CanonicalEncode> CanonicalEncode for (A, B) {
    fn encode(&self, out: &mut Vec<u8>) {
        self.0.encode(out);
        self.1.encode(out);
    }
}

impl<A: CanonicalDecode, B: CanonicalDecode> CanonicalDecode for (A, B) {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok((A::decode(input)?, B::decode(input)?))
    }
}

impl CanonicalEncode for Hash32 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl CanonicalDecode for Hash32 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(Hash32(read_exact::<32>(input)?))
    }
}

impl CanonicalEncode for PubKey {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl CanonicalDecode for PubKey {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PubKey(read_exact::<32>(input)?))
    }
}

impl CanonicalEncode for Sig64 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl CanonicalDecode for Sig64 {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(Sig64(read_exact::<64>(input)?))
    }
}

impl CanonicalEncode for PeerId {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl CanonicalDecode for PeerId {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PeerId(read_exact::<32>(input)?))
    }
}

impl CanonicalEncode for SessionId {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl CanonicalDecode for SessionId {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(SessionId(read_exact::<32>(input)?))
    }
}

impl CanonicalEncode for GamePlayerId {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, self.0);
    }
}

impl CanonicalDecode for GamePlayerId {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(GamePlayerId(read_u8(input)?))
    }
}

impl CanonicalEncode for GameObjectId {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u64(out, self.0);
    }
}

impl CanonicalDecode for GameObjectId {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(GameObjectId(read_u64(input)?))
    }
}

impl CanonicalEncode for ZoneCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for ZoneCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(ZoneCode::Library),
            1 => Ok(ZoneCode::Hand),
            2 => Ok(ZoneCode::Battlefield),
            3 => Ok(ZoneCode::Graveyard),
            4 => Ok(ZoneCode::Stack),
            5 => Ok(ZoneCode::Exile),
            6 => Ok(ZoneCode::Command),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for PhaseCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for PhaseCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(PhaseCode::Beginning),
            1 => Ok(PhaseCode::FirstMain),
            2 => Ok(PhaseCode::Combat),
            3 => Ok(PhaseCode::NextMain),
            4 => Ok(PhaseCode::Ending),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for StepCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for StepCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(StepCode::Untap),
            1 => Ok(StepCode::Upkeep),
            2 => Ok(StepCode::Draw),
            3 => Ok(StepCode::BeginCombat),
            4 => Ok(StepCode::DeclareAttackers),
            5 => Ok(StepCode::DeclareBlockers),
            6 => Ok(StepCode::CombatDamage),
            7 => Ok(StepCode::EndCombat),
            8 => Ok(StepCode::End),
            9 => Ok(StepCode::Cleanup),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for PhaseStep {
    fn encode(&self, out: &mut Vec<u8>) {
        self.phase.encode(out);
        self.step.encode(out);
    }
}

impl CanonicalDecode for PhaseStep {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        let phase = PhaseCode::decode(input)?;
        let step = Option::<StepCode>::decode(input)?;
        Ok(PhaseStep { phase, step })
    }
}

impl CanonicalEncode for ObjectKindCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for ObjectKindCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(ObjectKindCode::Card),
            1 => Ok(ObjectKindCode::Token),
            2 => Ok(ObjectKindCode::SpellCopy),
            3 => Ok(ObjectKindCode::Emblem),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for ManaPoolSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        self.white.encode(out);
        self.blue.encode(out);
        self.black.encode(out);
        self.red.encode(out);
        self.green.encode(out);
        self.colorless.encode(out);
    }
}

impl CanonicalDecode for ManaPoolSpec {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ManaPoolSpec {
            white: u32::decode(input)?,
            blue: u32::decode(input)?,
            black: u32::decode(input)?,
            red: u32::decode(input)?,
            green: u32::decode(input)?,
            colorless: u32::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicTurnState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.active_player.encode(out);
        self.priority_player.encode(out);
        self.turn_number.encode(out);
        self.phase.encode(out);
        self.step.encode(out);
    }
}

impl CanonicalDecode for PublicTurnState {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicTurnState {
            active_player: GamePlayerId::decode(input)?,
            priority_player: Option::<GamePlayerId>::decode(input)?,
            turn_number: u32::decode(input)?,
            phase: PhaseCode::decode(input)?,
            step: Option::<StepCode>::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicPlayerState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.id.encode(out);
        self.life.encode(out);
        self.mana_pool.encode(out);
        self.poison_counters.encode(out);
        self.energy_counters.encode(out);
        self.experience_counters.encode(out);
        self.lands_played_this_turn.encode(out);
        self.land_plays_per_turn.encode(out);
        self.max_hand_size.encode(out);
        self.has_lost.encode(out);
        self.has_won.encode(out);
        self.has_left_game.encode(out);
        self.hand_size.encode(out);
        self.library_size.encode(out);
        self.graveyard.encode(out);
        self.commanders.encode(out);
        self.commander_damage.encode(out);
    }
}

impl CanonicalDecode for PublicPlayerState {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicPlayerState {
            id: GamePlayerId::decode(input)?,
            life: i32::decode(input)?,
            mana_pool: ManaPoolSpec::decode(input)?,
            poison_counters: u32::decode(input)?,
            energy_counters: u32::decode(input)?,
            experience_counters: u32::decode(input)?,
            lands_played_this_turn: u32::decode(input)?,
            land_plays_per_turn: u32::decode(input)?,
            max_hand_size: i32::decode(input)?,
            has_lost: bool::decode(input)?,
            has_won: bool::decode(input)?,
            has_left_game: bool::decode(input)?,
            hand_size: u32::decode(input)?,
            library_size: u32::decode(input)?,
            graveyard: Vec::<GameObjectId>::decode(input)?,
            commanders: Vec::<GameObjectId>::decode(input)?,
            commander_damage: Vec::<(GamePlayerId, u32)>::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicZoneIndex {
    fn encode(&self, out: &mut Vec<u8>) {
        self.battlefield.encode(out);
        self.exile.encode(out);
        self.command_zone.encode(out);
    }
}

impl CanonicalDecode for PublicZoneIndex {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicZoneIndex {
            battlefield: Vec::<GameObjectId>::decode(input)?,
            exile: Vec::<GameObjectId>::decode(input)?,
            command_zone: Vec::<GameObjectId>::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicObjectState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.id.encode(out);
        self.stable_id.encode(out);
        self.kind.encode(out);
        self.card_ref.encode(out);
        self.zone.encode(out);
        self.owner.encode(out);
        self.controller.encode(out);
        self.object_hash.encode(out);
    }
}

impl CanonicalDecode for PublicObjectState {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicObjectState {
            id: GameObjectId::decode(input)?,
            stable_id: GameObjectId::decode(input)?,
            kind: ObjectKindCode::decode(input)?,
            card_ref: Option::<u32>::decode(input)?,
            zone: ZoneCode::decode(input)?,
            owner: GamePlayerId::decode(input)?,
            controller: GamePlayerId::decode(input)?,
            object_hash: Hash32::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicStackEntry {
    fn encode(&self, out: &mut Vec<u8>) {
        self.object_id.encode(out);
        self.controller.encode(out);
        self.entry_hash.encode(out);
    }
}

impl CanonicalDecode for PublicStackEntry {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicStackEntry {
            object_id: GameObjectId::decode(input)?,
            controller: GamePlayerId::decode(input)?,
            entry_hash: Hash32::decode(input)?,
        })
    }
}

impl CanonicalEncode for PublicStateSnapshot {
    fn encode(&self, out: &mut Vec<u8>) {
        self.version.encode(out);
        self.turn.encode(out);
        self.turn_order.encode(out);
        self.players.encode(out);
        self.zones.encode(out);
        self.objects.encode(out);
        self.stack.encode(out);
        self.combat_hash.encode(out);
        self.trackers_hash.encode(out);
    }
}

impl CanonicalDecode for PublicStateSnapshot {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PublicStateSnapshot {
            version: u8::decode(input)?,
            turn: PublicTurnState::decode(input)?,
            turn_order: Vec::<GamePlayerId>::decode(input)?,
            players: Vec::<PublicPlayerState>::decode(input)?,
            zones: PublicZoneIndex::decode(input)?,
            objects: Vec::<PublicObjectState>::decode(input)?,
            stack: Vec::<PublicStackEntry>::decode(input)?,
            combat_hash: Option::<Hash32>::decode(input)?,
            trackers_hash: Hash32::decode(input)?,
        })
    }
}

impl CanonicalEncode for CommitmentScheme {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for CommitmentScheme {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(CommitmentScheme::ElgamalSecp256k1Sha256MerkleV1),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for ZoneCommitment {
    fn encode(&self, out: &mut Vec<u8>) {
        self.scheme.encode(out);
        self.root.encode(out);
        self.len.encode(out);
        self.top.encode(out);
        self.viewers.encode(out);
    }
}

impl CanonicalDecode for ZoneCommitment {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ZoneCommitment {
            scheme: CommitmentScheme::decode(input)?,
            root: Hash32::decode(input)?,
            len: u32::decode(input)?,
            top: u32::decode(input)?,
            viewers: Vec::<PeerId>::decode(input)?,
        })
    }
}

impl CanonicalEncode for HiddenZoneCommitment {
    fn encode(&self, out: &mut Vec<u8>) {
        self.zone.encode(out);
        self.owner.encode(out);
        self.commitment.encode(out);
    }
}

impl CanonicalDecode for HiddenZoneCommitment {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(HiddenZoneCommitment {
            zone: ZoneCode::decode(input)?,
            owner: GamePlayerId::decode(input)?,
            commitment: ZoneCommitment::decode(input)?,
        })
    }
}

impl CanonicalEncode for StateCommitment {
    fn encode(&self, out: &mut Vec<u8>) {
        self.version.encode(out);
        self.public_state_hash.encode(out);
        self.hidden_zones.encode(out);
    }
}

impl CanonicalDecode for StateCommitment {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(StateCommitment {
            version: u8::decode(input)?,
            public_state_hash: Hash32::decode(input)?,
            hidden_zones: Vec::<HiddenZoneCommitment>::decode(input)?,
        })
    }
}

impl CanonicalEncode for ManaSymbolCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for ManaSymbolCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(ManaSymbolCode::White),
            1 => Ok(ManaSymbolCode::Blue),
            2 => Ok(ManaSymbolCode::Black),
            3 => Ok(ManaSymbolCode::Red),
            4 => Ok(ManaSymbolCode::Green),
            5 => Ok(ManaSymbolCode::Colorless),
            6 => Ok(ManaSymbolCode::Generic),
            7 => Ok(ManaSymbolCode::Snow),
            8 => Ok(ManaSymbolCode::Life),
            9 => Ok(ManaSymbolCode::X),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for ManaSymbolSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        self.symbol.encode(out);
        self.value.encode(out);
    }
}

impl CanonicalDecode for ManaSymbolSpec {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ManaSymbolSpec {
            symbol: ManaSymbolCode::decode(input)?,
            value: u8::decode(input)?,
        })
    }
}

impl CanonicalEncode for MsgType {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for MsgType {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            1 => Ok(MsgType::ActionPropose),
            2 => Ok(MsgType::ActionAck),
            3 => Ok(MsgType::ActionReject),
            4 => Ok(MsgType::ActionCommit),
            5 => Ok(MsgType::PolicyToken),
            6 => Ok(MsgType::PolicyCancel),
            7 => Ok(MsgType::ContribRequest),
            8 => Ok(MsgType::ContribShare),
            9 => Ok(MsgType::TimeoutClaim),
            10 => Ok(MsgType::ForfeitCommit),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for RejectCode {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for RejectCode {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(RejectCode::InvalidAction),
            1 => Ok(RejectCode::InvalidProof),
            2 => Ok(RejectCode::InvalidStateHash),
            3 => Ok(RejectCode::NotPriorityHolder),
            4 => Ok(RejectCode::Unauthorized),
            5 => Ok(RejectCode::Malformed),
            255 => Ok(RejectCode::Other),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for TimeoutReason {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for TimeoutReason {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(TimeoutReason::MissingAck),
            1 => Ok(TimeoutReason::MissingShare),
            2 => Ok(TimeoutReason::MissingShuffleProof),
            3 => Ok(TimeoutReason::MissingPolicyResponse),
            255 => Ok(TimeoutReason::Other),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for ActionKind {
    fn encode(&self, out: &mut Vec<u8>) {
        write_u8(out, *self as u8);
    }
}

impl CanonicalDecode for ActionKind {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(ActionKind::PassPriority),
            1 => Ok(ActionKind::CastSpell),
            2 => Ok(ActionKind::ActivateAbility),
            3 => Ok(ActionKind::ResolveTop),
            4 => Ok(ActionKind::DrawCard),
            5 => Ok(ActionKind::ShuffleLibrary),
            6 => Ok(ActionKind::SearchLibrary),
            7 => Ok(ActionKind::ReorderTopN),
            8 => Ok(ActionKind::MoveCard),
            9 => Ok(ActionKind::RevealCard),
            255 => Ok(ActionKind::Other),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for Envelope {
    fn encode(&self, out: &mut Vec<u8>) {
        self.msg_type.encode(out);
        self.sender.encode(out);
        self.session_id.encode(out);
        self.seq.encode(out);
        self.payload.encode(out);
        self.sig.encode(out);
    }
}

impl CanonicalDecode for Envelope {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(Envelope {
            msg_type: MsgType::decode(input)?,
            sender: PeerId::decode(input)?,
            session_id: SessionId::decode(input)?,
            seq: u64::decode(input)?,
            payload: Vec::<u8>::decode(input)?,
            sig: Sig64::decode(input)?,
        })
    }
}

impl Envelope {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.msg_type.encode(&mut out);
        self.sender.encode(&mut out);
        self.session_id.encode(&mut out);
        self.seq.encode(&mut out);
        self.payload.encode(&mut out);
        out
    }
}

impl CanonicalEncode for ActionPropose {
    fn encode(&self, out: &mut Vec<u8>) {
        self.action_id.encode(out);
        self.prev_state_hash.encode(out);
        self.action.encode(out);
        self.proofs.encode(out);
        self.contribs_hash.encode(out);
        self.proposer_sig.encode(out);
    }
}

impl CanonicalDecode for ActionPropose {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ActionPropose {
            action_id: Hash32::decode(input)?,
            prev_state_hash: Hash32::decode(input)?,
            action: ActionPayload::decode(input)?,
            proofs: ProofBundle::decode(input)?,
            contribs_hash: Option::<Hash32>::decode(input)?,
            proposer_sig: Sig64::decode(input)?,
        })
    }
}

impl ActionPropose {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.prev_state_hash.encode(&mut out);
        self.action.encode(&mut out);
        self.proofs.encode(&mut out);
        self.contribs_hash.encode(&mut out);
        out
    }
}

impl CanonicalEncode for ActionAck {
    fn encode(&self, out: &mut Vec<u8>) {
        self.action_id.encode(out);
        self.ack_sig.encode(out);
    }
}

impl CanonicalDecode for ActionAck {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ActionAck {
            action_id: Hash32::decode(input)?,
            ack_sig: Sig64::decode(input)?,
        })
    }
}

impl CanonicalEncode for ActionReject {
    fn encode(&self, out: &mut Vec<u8>) {
        self.action_id.encode(out);
        self.reason.encode(out);
        self.reject_sig.encode(out);
    }
}

impl CanonicalDecode for ActionReject {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ActionReject {
            action_id: Hash32::decode(input)?,
            reason: RejectCode::decode(input)?,
            reject_sig: Sig64::decode(input)?,
        })
    }
}

impl ActionReject {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.action_id.encode(&mut out);
        self.reason.encode(&mut out);
        out
    }
}

impl CanonicalEncode for ActionCommit {
    fn encode(&self, out: &mut Vec<u8>) {
        self.action_id.encode(out);
        self.ack_sigs.encode(out);
        self.commit_sig.encode(out);
    }
}

impl ActionCommit {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.action_id.encode(&mut out);
        self.ack_sigs.encode(&mut out);
        out
    }
}

impl CanonicalDecode for ActionCommit {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ActionCommit {
            action_id: Hash32::decode(input)?,
            ack_sigs: Vec::<(PeerId, Sig64)>::decode(input)?,
            commit_sig: Sig64::decode(input)?,
        })
    }
}

impl CanonicalEncode for ContribRequest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.request_id.encode(out);
        self.prev_state_hash.encode(out);
        self.action_kind.encode(out);
        self.required_from.encode(out);
        self.deadline_ms.encode(out);
        self.request_sig.encode(out);
    }
}

impl CanonicalDecode for ContribRequest {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ContribRequest {
            request_id: Hash32::decode(input)?,
            prev_state_hash: Hash32::decode(input)?,
            action_kind: ActionKind::decode(input)?,
            required_from: Vec::<PeerId>::decode(input)?,
            deadline_ms: u64::decode(input)?,
            request_sig: Sig64::decode(input)?,
        })
    }
}

impl ContribRequest {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.prev_state_hash.encode(&mut out);
        self.action_kind.encode(&mut out);
        self.required_from.encode(&mut out);
        self.deadline_ms.encode(&mut out);
        out
    }
}

impl CanonicalEncode for ContribShare {
    fn encode(&self, out: &mut Vec<u8>) {
        self.request_id.encode(out);
        self.contributor.encode(out);
        self.share_payload.encode(out);
        self.share_proof.encode(out);
        self.share_sig.encode(out);
    }
}

impl CanonicalDecode for ContribShare {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ContribShare {
            request_id: Hash32::decode(input)?,
            contributor: PeerId::decode(input)?,
            share_payload: Vec::<u8>::decode(input)?,
            share_proof: Vec::<u8>::decode(input)?,
            share_sig: Sig64::decode(input)?,
        })
    }
}

impl ContribShare {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.request_id.encode(&mut out);
        self.contributor.encode(&mut out);
        self.share_payload.encode(&mut out);
        self.share_proof.encode(&mut out);
        out
    }
}

impl CanonicalEncode for PolicyToken {
    fn encode(&self, out: &mut Vec<u8>) {
        self.policy_id.encode(out);
        self.owner.encode(out);
        self.active_from_state_hash.encode(out);
        self.expires_at.encode(out);
        self.conditions.encode(out);
        self.owner_sig.encode(out);
    }
}

impl CanonicalDecode for PolicyToken {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PolicyToken {
            policy_id: Hash32::decode(input)?,
            owner: PeerId::decode(input)?,
            active_from_state_hash: Hash32::decode(input)?,
            expires_at: PhaseStep::decode(input)?,
            conditions: PolicyConditions::decode(input)?,
            owner_sig: Sig64::decode(input)?,
        })
    }
}

impl PolicyToken {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.owner.encode(&mut out);
        self.active_from_state_hash.encode(&mut out);
        self.expires_at.encode(&mut out);
        self.conditions.encode(&mut out);
        out
    }
}

impl CanonicalEncode for PolicyCancel {
    fn encode(&self, out: &mut Vec<u8>) {
        self.policy_id.encode(out);
        self.cancel_sig.encode(out);
    }
}

impl CanonicalDecode for PolicyCancel {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PolicyCancel {
            policy_id: Hash32::decode(input)?,
            cancel_sig: Sig64::decode(input)?,
        })
    }
}

impl PolicyCancel {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.policy_id.encode(&mut out);
        out
    }
}

impl CanonicalEncode for PolicyConditions {
    fn encode(&self, out: &mut Vec<u8>) {
        self.stop_on_stack_event.encode(out);
        self.stop_if_targets_me.encode(out);
        self.stop_if_attackers_declared.encode(out);
        self.stop_if_blockers_declared.encode(out);
        self.until_phase.encode(out);
    }
}

impl CanonicalDecode for PolicyConditions {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(PolicyConditions {
            stop_on_stack_event: bool::decode(input)?,
            stop_if_targets_me: bool::decode(input)?,
            stop_if_attackers_declared: bool::decode(input)?,
            stop_if_blockers_declared: bool::decode(input)?,
            until_phase: PhaseStep::decode(input)?,
        })
    }
}

impl CanonicalEncode for TimeoutClaim {
    fn encode(&self, out: &mut Vec<u8>) {
        self.action_id.encode(out);
        self.missing_peer.encode(out);
        self.reason.encode(out);
        self.claimer_sig.encode(out);
    }
}

impl CanonicalDecode for TimeoutClaim {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(TimeoutClaim {
            action_id: Hash32::decode(input)?,
            missing_peer: PeerId::decode(input)?,
            reason: TimeoutReason::decode(input)?,
            claimer_sig: Sig64::decode(input)?,
        })
    }
}

impl TimeoutClaim {
    pub fn encode_for_hash(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.action_id.encode(&mut out);
        self.missing_peer.encode(&mut out);
        self.reason.encode(&mut out);
        out
    }
}

impl CanonicalEncode for ForfeitCommit {
    fn encode(&self, out: &mut Vec<u8>) {
        self.missing_peer.encode(out);
        self.reason.encode(out);
        self.claim_sigs.encode(out);
    }
}

impl CanonicalDecode for ForfeitCommit {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ForfeitCommit {
            missing_peer: PeerId::decode(input)?,
            reason: TimeoutReason::decode(input)?,
            claim_sigs: Vec::<(PeerId, Sig64)>::decode(input)?,
        })
    }
}

impl CanonicalEncode for TargetSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            TargetSpec::Object(obj) => {
                write_u8(out, 0);
                obj.encode(out);
            }
            TargetSpec::Player(player) => {
                write_u8(out, 1);
                player.encode(out);
            }
        }
    }
}

impl CanonicalDecode for TargetSpec {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(TargetSpec::Object(GameObjectId::decode(input)?)),
            1 => Ok(TargetSpec::Player(GamePlayerId::decode(input)?)),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for CostPayment {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            CostPayment::Tap { objects } => {
                write_u8(out, 0);
                objects.encode(out);
            }
            CostPayment::Untap { objects } => {
                write_u8(out, 1);
                objects.encode(out);
            }
            CostPayment::Sacrifice { objects } => {
                write_u8(out, 2);
                objects.encode(out);
            }
            CostPayment::Discard { objects } => {
                write_u8(out, 3);
                objects.encode(out);
            }
            CostPayment::Exile { objects, from_zone } => {
                write_u8(out, 4);
                objects.encode(out);
                from_zone.encode(out);
            }
            CostPayment::Reveal { objects } => {
                write_u8(out, 5);
                objects.encode(out);
            }
            CostPayment::ReturnToHand { objects } => {
                write_u8(out, 6);
                objects.encode(out);
            }
            CostPayment::ActivateManaAbility {
                source,
                ability_index,
            } => {
                write_u8(out, 10);
                source.encode(out);
                ability_index.encode(out);
            }
            CostPayment::Life { amount } => {
                write_u8(out, 7);
                amount.encode(out);
            }
            CostPayment::Energy { amount } => {
                write_u8(out, 8);
                amount.encode(out);
            }
            CostPayment::Mill { count } => {
                write_u8(out, 9);
                count.encode(out);
            }
            CostPayment::Other { tag, data } => {
                write_u8(out, 255);
                tag.encode(out);
                data.encode(out);
            }
        }
    }
}

impl CanonicalDecode for CostPayment {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(CostPayment::Tap {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            1 => Ok(CostPayment::Untap {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            2 => Ok(CostPayment::Sacrifice {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            3 => Ok(CostPayment::Discard {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            4 => Ok(CostPayment::Exile {
                objects: Vec::<GameObjectId>::decode(input)?,
                from_zone: ZoneCode::decode(input)?,
            }),
            5 => Ok(CostPayment::Reveal {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            6 => Ok(CostPayment::ReturnToHand {
                objects: Vec::<GameObjectId>::decode(input)?,
            }),
            10 => Ok(CostPayment::ActivateManaAbility {
                source: GameObjectId::decode(input)?,
                ability_index: u32::decode(input)?,
            }),
            7 => Ok(CostPayment::Life {
                amount: u32::decode(input)?,
            }),
            8 => Ok(CostPayment::Energy {
                amount: u32::decode(input)?,
            }),
            9 => Ok(CostPayment::Mill {
                count: u32::decode(input)?,
            }),
            255 => Ok(CostPayment::Other {
                tag: u8::decode(input)?,
                data: Vec::<u8>::decode(input)?,
            }),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for CostStep {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            CostStep::Mana(symbol) => {
                write_u8(out, 0);
                symbol.encode(out);
            }
            CostStep::Payment(payment) => {
                write_u8(out, 1);
                payment.encode(out);
            }
        }
    }
}

impl CanonicalDecode for CostStep {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        match read_u8(input)? {
            0 => Ok(CostStep::Mana(ManaSymbolSpec::decode(input)?)),
            1 => Ok(CostStep::Payment(CostPayment::decode(input)?)),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }
}

impl CanonicalEncode for CostSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        self.payment_trace.encode(out);
        self.optional_costs.encode(out);
        self.x_value.encode(out);
    }
}

impl CanonicalDecode for CostSpec {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(CostSpec {
            payment_trace: Vec::<CostStep>::decode(input)?,
            optional_costs: Vec::<u32>::decode(input)?,
            x_value: Option::<u32>::decode(input)?,
        })
    }
}

impl CanonicalEncode for ActionPayload {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            ActionPayload::PassPriority { policy_id } => {
                ActionKind::PassPriority.encode(out);
                policy_id.encode(out);
            }
            ActionPayload::CastSpell {
                card_ref,
                targets,
                costs,
            } => {
                ActionKind::CastSpell.encode(out);
                card_ref.encode(out);
                targets.encode(out);
                costs.encode(out);
            }
            ActionPayload::ActivateAbility {
                source_ref,
                costs,
                targets,
            } => {
                ActionKind::ActivateAbility.encode(out);
                source_ref.encode(out);
                costs.encode(out);
                targets.encode(out);
            }
            ActionPayload::ResolveTop => {
                ActionKind::ResolveTop.encode(out);
            }
            ActionPayload::DrawCard { count } => {
                ActionKind::DrawCard.encode(out);
                count.encode(out);
            }
            ActionPayload::ShuffleLibrary => {
                ActionKind::ShuffleLibrary.encode(out);
            }
            ActionPayload::SearchLibrary {
                predicate_root,
                reveal,
            } => {
                ActionKind::SearchLibrary.encode(out);
                predicate_root.encode(out);
                reveal.encode(out);
            }
            ActionPayload::ReorderTopN { n, new_commitment } => {
                ActionKind::ReorderTopN.encode(out);
                n.encode(out);
                new_commitment.encode(out);
            }
            ActionPayload::MoveCard {
                from_zone,
                to_zone,
                card_ref,
            } => {
                ActionKind::MoveCard.encode(out);
                from_zone.encode(out);
                to_zone.encode(out);
                card_ref.encode(out);
            }
            ActionPayload::RevealCard { card_ref } => {
                ActionKind::RevealCard.encode(out);
                card_ref.encode(out);
            }
        }
    }
}

impl CanonicalDecode for ActionPayload {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        let tag = ActionKind::decode(input)?;
        match tag {
            ActionKind::PassPriority => Ok(ActionPayload::PassPriority {
                policy_id: Option::<Hash32>::decode(input)?,
            }),
            ActionKind::CastSpell => Ok(ActionPayload::CastSpell {
                card_ref: GameObjectId::decode(input)?,
                targets: Vec::<TargetSpec>::decode(input)?,
                costs: CostSpec::decode(input)?,
            }),
            ActionKind::ActivateAbility => Ok(ActionPayload::ActivateAbility {
                source_ref: GameObjectId::decode(input)?,
                costs: CostSpec::decode(input)?,
                targets: Vec::<TargetSpec>::decode(input)?,
            }),
            ActionKind::ResolveTop => Ok(ActionPayload::ResolveTop),
            ActionKind::DrawCard => Ok(ActionPayload::DrawCard {
                count: u8::decode(input)?,
            }),
            ActionKind::ShuffleLibrary => Ok(ActionPayload::ShuffleLibrary),
            ActionKind::SearchLibrary => Ok(ActionPayload::SearchLibrary {
                predicate_root: Hash32::decode(input)?,
                reveal: bool::decode(input)?,
            }),
            ActionKind::ReorderTopN => Ok(ActionPayload::ReorderTopN {
                n: u8::decode(input)?,
                new_commitment: Hash32::decode(input)?,
            }),
            ActionKind::MoveCard => Ok(ActionPayload::MoveCard {
                from_zone: ZoneCode::decode(input)?,
                to_zone: ZoneCode::decode(input)?,
                card_ref: GameObjectId::decode(input)?,
            }),
            ActionKind::RevealCard => Ok(ActionPayload::RevealCard {
                card_ref: GameObjectId::decode(input)?,
            }),
            ActionKind::Other => Err(CodecError::InvalidTag(255)),
        }
    }
}

impl CanonicalEncode for ProofBundle {
    fn encode(&self, out: &mut Vec<u8>) {
        self.shuffle_proof.encode(out);
        self.decrypt_shares.encode(out);
        self.decrypt_share_proofs.encode(out);
        self.plaintext_commitment.encode(out);
        self.plaintext_eq_proof.encode(out);
        self.membership_stark.encode(out);
        self.permutation_stark.encode(out);
    }
}

impl CanonicalDecode for ProofBundle {
    fn decode(input: &mut &[u8]) -> Result<Self, CodecError> {
        Ok(ProofBundle {
            shuffle_proof: Option::<Vec<u8>>::decode(input)?,
            decrypt_shares: Vec::<Vec<u8>>::decode(input)?,
            decrypt_share_proofs: Vec::<Vec<u8>>::decode(input)?,
            plaintext_commitment: Option::<Hash32>::decode(input)?,
            plaintext_eq_proof: Option::<Vec<u8>>::decode(input)?,
            membership_stark: Option::<Vec<u8>>::decode(input)?,
            permutation_stark: Option::<Vec<u8>>::decode(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T>(value: &T)
    where
        T: CanonicalEncode + CanonicalDecode + PartialEq + std::fmt::Debug,
    {
        let bytes = value.to_bytes();
        let mut slice = bytes.as_slice();
        let decoded = T::decode(&mut slice).expect("decode failed");
        assert!(slice.is_empty(), "decoder should consume all bytes");
        assert_eq!(&decoded, value);
    }

    #[test]
    fn action_payload_round_trip() {
        let payload = ActionPayload::CastSpell {
            card_ref: GameObjectId(42),
            targets: vec![
                TargetSpec::Object(GameObjectId(7)),
                TargetSpec::Player(GamePlayerId(2)),
            ],
            costs: CostSpec {
                payment_trace: vec![
                    CostStep::Mana(ManaSymbolSpec {
                        symbol: ManaSymbolCode::Generic,
                        value: 2,
                    }),
                    CostStep::Mana(ManaSymbolSpec {
                        symbol: ManaSymbolCode::Red,
                        value: 0,
                    }),
                    CostStep::Payment(CostPayment::Discard {
                        objects: vec![GameObjectId(99)],
                    }),
                    CostStep::Payment(CostPayment::Life { amount: 2 }),
                ],
                optional_costs: vec![1],
                x_value: Some(3),
            },
        };

        round_trip(&payload);
    }

    #[test]
    fn policy_token_round_trip() {
        let token = PolicyToken {
            policy_id: Hash32([1u8; 32]),
            owner: PeerId([2u8; 32]),
            active_from_state_hash: Hash32([3u8; 32]),
            expires_at: PhaseStep {
                phase: PhaseCode::Combat,
                step: Some(StepCode::DeclareAttackers),
            },
            conditions: PolicyConditions {
                stop_on_stack_event: true,
                stop_if_targets_me: false,
                stop_if_attackers_declared: true,
                stop_if_blockers_declared: true,
                until_phase: PhaseStep {
                    phase: PhaseCode::Ending,
                    step: Some(StepCode::End),
                },
            },
            owner_sig: Sig64([9u8; 64]),
        };

        round_trip(&token);
    }

    #[test]
    fn envelope_round_trip() {
        let env = Envelope {
            msg_type: MsgType::ActionPropose,
            sender: PeerId([4u8; 32]),
            session_id: SessionId([5u8; 32]),
            seq: 123,
            payload: vec![0xAA, 0xBB, 0xCC],
            sig: Sig64([7u8; 64]),
        };

        round_trip(&env);
    }

    #[test]
    fn action_propose_round_trip() {
        let propose = ActionPropose {
            action_id: Hash32([1u8; 32]),
            prev_state_hash: Hash32([2u8; 32]),
            action: ActionPayload::DrawCard { count: 2 },
            proofs: ProofBundle {
                shuffle_proof: None,
                decrypt_shares: vec![vec![1, 2, 3]],
                decrypt_share_proofs: vec![vec![4, 5, 6]],
                plaintext_commitment: Some(Hash32([9u8; 32])),
                plaintext_eq_proof: None,
                membership_stark: None,
                permutation_stark: None,
            },
            contribs_hash: Some(Hash32([7u8; 32])),
            proposer_sig: Sig64([3u8; 64]),
        };

        round_trip(&propose);
    }

    #[test]
    fn contrib_share_round_trip() {
        let share = ContribShare {
            request_id: Hash32([8u8; 32]),
            contributor: PeerId([6u8; 32]),
            share_payload: vec![10, 11, 12],
            share_proof: vec![13, 14, 15],
            share_sig: Sig64([4u8; 64]),
        };

        round_trip(&share);
    }

    #[test]
    fn invalid_bool_rejected() {
        let mut data = [2u8].as_slice();
        let err = bool::decode(&mut data).expect_err("expected invalid bool");
        assert!(matches!(err, CodecError::InvalidBool(2)));
    }

    #[test]
    fn invalid_tag_rejected() {
        let mut data = [9u8].as_slice();
        let err = ZoneCode::decode(&mut data).expect_err("expected invalid tag");
        assert!(matches!(err, CodecError::InvalidTag(9)));
    }

    #[test]
    fn length_too_large_rejected() {
        let len = super::MAX_VEC_LEN + 1;
        let mut bytes = Vec::new();
        len.encode(&mut bytes);
        let mut slice = bytes.as_slice();
        let err = Vec::<u8>::decode(&mut slice).expect_err("expected length error");
        assert!(matches!(err, CodecError::LengthTooLarge(_)));
    }

    #[test]
    fn invalid_string_rejected() {
        let mut bytes = Vec::new();
        1u32.encode(&mut bytes);
        bytes.push(0xFF);
        let mut slice = bytes.as_slice();
        let err = String::decode(&mut slice).expect_err("expected invalid string");
        assert!(matches!(err, CodecError::InvalidString));
    }
}
