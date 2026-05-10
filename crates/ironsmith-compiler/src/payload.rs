use crate::ability::ActivationTiming;
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::filter::ObjectFilter;
use crate::mana::ManaCost;
use crate::static_abilities::LandwalkKind;
use crate::types::{CardType, Subtype};

#[derive(Debug, Clone, PartialEq)]
pub enum KeywordAction {
    Flying,
    Menace,
    Banding,
    Hexproof,
    Haste,
    Improvise,
    Convoke,
    AffinityForArtifacts,
    Delve,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Lifelink,
    Vigilance,
    Trample,
    Reach,
    Defender,
    Decayed,
    Flash,
    Phasing,
    Indestructible,
    Shroud,
    Ward(u32),
    Wither,
    Afflict(u32),
    Afterlife(u32),
    Fabricate(u32),
    Infect,
    Undying,
    Persist,
    Prowess,
    Exalted,
    Cascade,
    Storm,
    Toxic(u32),
    BattleCry,
    Dethrone,
    Evolve,
    Ingest,
    Mentor,
    Skulk,
    Training,
    Myriad,
    Riot,
    Unleash,
    Renown(u32),
    Modular(u32),
    ModularSunburst,
    Graft(u32),
    Soulbond,
    Soulshift(u32),
    Outlast(ManaCost),
    Scavenge(ManaCost),
    Unearth(ManaCost),
    Ninjutsu(ManaCost),
    Backup(u32),
    Cipher,
    Dash(ManaCost),
    Blitz(ManaCost),
    BlitzFromGraveyard,
    Warp(ManaCost),
    Plot(ManaCost),
    Melee,
    Mobilize(u32),
    Suspend {
        time: u32,
        cost: ManaCost,
    },
    Disturb(ManaCost),
    Overload(ManaCost),
    Awaken {
        amount: u32,
        cost: ManaCost,
    },
    Spectacle(ManaCost),
    Foretell(ManaCost),
    Echo {
        total_cost: TotalCost,
        text: String,
    },
    CumulativeUpkeep {
        total_cost: TotalCost,
        text: String,
    },
    Casualty(u32),
    Conspire,
    Amplify(u32),
    AuraSwap(ManaCost),
    Devour(u32),
    Ravenous,
    Ascend,
    Daybound,
    Nightbound,
    Haunt,
    Provoke,
    Undaunted,
    Enlist,
    Extort,
    Partner,
    StartYourEngines,
    Assist,
    SplitSecond,
    Rebound,
    Sunburst,
    ReadAhead,
    Fading(u32),
    Vanishing(u32),
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Flanking,
    UmbraArmor,
    Landwalk(LandwalkKind),
    Bloodthirst(u32),
    Rampage(u32),
    Bushido(u32),
    Changeling,
    HexproofFrom(ObjectFilter),
    ProtectionFrom(ColorSet),
    ProtectionFromAllColors,
    ProtectionFromColorless,
    ProtectionFromEverything,
    ProtectionFromChosenPlayer,
    ProtectionFromChosenColor,
    ProtectionFromFilter(ObjectFilter),
    ProtectionFromCardType(CardType),
    ProtectionFromSubtype(Subtype),
    Unblockable,
    Devoid,
    Annihilator(u32),
    ForMirrodin,
    LivingWeapon,
    Crew {
        amount: u32,
        timing: ActivationTiming,
        additional_restrictions: Vec<String>,
    },
    Saddle {
        amount: u32,
        timing: ActivationTiming,
        additional_restrictions: Vec<String>,
    },
    Marker(&'static str),
    MarkerText(String),
}

impl KeywordAction {
    pub fn lowers_to_static_ability(&self) -> bool {
        matches!(
            self,
            Self::Flying
                | Self::Menace
                | Self::Banding
                | Self::Hexproof
                | Self::Haste
                | Self::Improvise
                | Self::Convoke
                | Self::AffinityForArtifacts
                | Self::Delve
                | Self::FirstStrike
                | Self::DoubleStrike
                | Self::Deathtouch
                | Self::Lifelink
                | Self::Vigilance
                | Self::Trample
                | Self::Reach
                | Self::Defender
                | Self::Decayed
                | Self::Flash
                | Self::Phasing
                | Self::Indestructible
                | Self::Shroud
                | Self::Ward(_)
                | Self::Wither
                | Self::Afterlife(_)
                | Self::Fabricate(_)
                | Self::Infect
                | Self::Undying
                | Self::Persist
                | Self::Prowess
                | Self::Exalted
                | Self::Cascade
                | Self::Storm
                | Self::Toxic(_)
                | Self::BattleCry
                | Self::Dethrone
                | Self::Evolve
                | Self::Ingest
                | Self::Mentor
                | Self::Skulk
                | Self::Training
                | Self::Riot
                | Self::Unleash
                | Self::Renown(_)
                | Self::Modular(_)
                | Self::Graft(_)
                | Self::Soulbond
                | Self::Soulshift(_)
                | Self::Outlast(_)
                | Self::Unearth(_)
                | Self::Ninjutsu(_)
                | Self::Extort
                | Self::Partner
                | Self::StartYourEngines
                | Self::Assist
                | Self::SplitSecond
                | Self::Rebound
                | Self::Sunburst
                | Self::ReadAhead
                | Self::Fading(_)
                | Self::Vanishing(_)
                | Self::Fear
                | Self::Intimidate
                | Self::Shadow
                | Self::Horsemanship
                | Self::Flanking
                | Self::UmbraArmor
                | Self::Landwalk(_)
                | Self::Bloodthirst(_)
                | Self::Rampage(_)
                | Self::Bushido(_)
                | Self::Changeling
                | Self::HexproofFrom(_)
                | Self::ProtectionFrom(_)
                | Self::ProtectionFromAllColors
                | Self::ProtectionFromColorless
                | Self::ProtectionFromEverything
                | Self::ProtectionFromChosenPlayer
                | Self::ProtectionFromChosenColor
                | Self::ProtectionFromFilter(_)
                | Self::ProtectionFromCardType(_)
                | Self::ProtectionFromSubtype(_)
                | Self::Unblockable
                | Self::Devoid
                | Self::Annihilator(_)
                | Self::Marker(_)
                | Self::MarkerText(_)
        )
    }

    pub fn display_text(&self) -> String {
        fn single_color_name(colors: ColorSet) -> Option<&'static str> {
            if colors == ColorSet::WHITE {
                return Some("white");
            }
            if colors == ColorSet::BLUE {
                return Some("blue");
            }
            if colors == ColorSet::BLACK {
                return Some("black");
            }
            if colors == ColorSet::RED {
                return Some("red");
            }
            if colors == ColorSet::GREEN {
                return Some("green");
            }
            None
        }

        match self {
            Self::Flying => "Flying".to_string(),
            Self::Menace => "Menace".to_string(),
            Self::Banding => "Banding".to_string(),
            Self::Hexproof => "Hexproof".to_string(),
            Self::Haste => "Haste".to_string(),
            Self::Improvise => "Improvise".to_string(),
            Self::Convoke => "Convoke".to_string(),
            Self::AffinityForArtifacts => "Affinity for artifacts".to_string(),
            Self::Delve => "Delve".to_string(),
            Self::FirstStrike => "First strike".to_string(),
            Self::DoubleStrike => "Double strike".to_string(),
            Self::Deathtouch => "Deathtouch".to_string(),
            Self::Lifelink => "Lifelink".to_string(),
            Self::Vigilance => "Vigilance".to_string(),
            Self::Trample => "Trample".to_string(),
            Self::Reach => "Reach".to_string(),
            Self::Defender => "Defender".to_string(),
            Self::Decayed => "Decayed".to_string(),
            Self::Flash => "Flash".to_string(),
            Self::Phasing => "Phasing".to_string(),
            Self::Indestructible => "Indestructible".to_string(),
            Self::Shroud => "Shroud".to_string(),
            Self::Ward(amount) => format!("Ward {{{amount}}}"),
            Self::Wither => "Wither".to_string(),
            Self::Afflict(amount) => format!("Afflict {amount}"),
            Self::Afterlife(amount) => format!("Afterlife {amount}"),
            Self::Fabricate(amount) => format!("Fabricate {amount}"),
            Self::Infect => "Infect".to_string(),
            Self::Undying => "Undying".to_string(),
            Self::Persist => "Persist".to_string(),
            Self::Prowess => "Prowess".to_string(),
            Self::Exalted => "Exalted".to_string(),
            Self::Cascade => "Cascade".to_string(),
            Self::Storm => "Storm".to_string(),
            Self::Toxic(amount) => format!("Toxic {amount}"),
            Self::BattleCry => "Battle cry".to_string(),
            Self::Dethrone => "Dethrone".to_string(),
            Self::Evolve => "Evolve".to_string(),
            Self::Ingest => "Ingest".to_string(),
            Self::Mentor => "Mentor".to_string(),
            Self::Skulk => "Skulk".to_string(),
            Self::Training => "Training".to_string(),
            Self::Myriad => "Myriad".to_string(),
            Self::Riot => "Riot".to_string(),
            Self::Unleash => "Unleash".to_string(),
            Self::Renown(amount) => format!("Renown {amount}"),
            Self::Modular(amount) => format!("Modular {amount}"),
            Self::ModularSunburst => "Modular-Sunburst".to_string(),
            Self::Graft(amount) => format!("Graft {amount}"),
            Self::Soulbond => "Soulbond".to_string(),
            Self::Soulshift(amount) => format!("Soulshift {amount}"),
            Self::Outlast(cost) => format!("Outlast {}", cost.to_oracle()),
            Self::Scavenge(cost) => format!("Scavenge {}", cost.to_oracle()),
            Self::Unearth(cost) => format!("Unearth {}", cost.to_oracle()),
            Self::Ninjutsu(cost) => format!("Ninjutsu {}", cost.to_oracle()),
            Self::Backup(amount) => format!("Backup {amount}"),
            Self::Cipher => "Cipher".to_string(),
            Self::Dash(cost) => format!("Dash {}", cost.to_oracle()),
            Self::Blitz(cost) => format!("Blitz {}", cost.to_oracle()),
            Self::BlitzFromGraveyard => {
                "You may cast this card from your graveyard using its blitz ability.".to_string()
            }
            Self::Warp(cost) => format!("Warp {}", cost.to_oracle()),
            Self::Plot(cost) => format!("Plot {}", cost.to_oracle()),
            Self::Melee => "Melee".to_string(),
            Self::Mobilize(amount) => format!("Mobilize {amount}"),
            Self::Suspend { time, cost } => format!("Suspend {time}—{}", cost.to_oracle()),
            Self::Disturb(cost) => format!("Disturb {}", cost.to_oracle()),
            Self::Overload(cost) => format!("Overload {}", cost.to_oracle()),
            Self::Awaken { amount, cost } => format!("Awaken {amount}—{}", cost.to_oracle()),
            Self::Spectacle(cost) => format!("Spectacle {}", cost.to_oracle()),
            Self::Foretell(cost) => format!("Foretell {}", cost.to_oracle()),
            Self::Echo { text, .. } => text.clone(),
            Self::CumulativeUpkeep { text, .. } => text.clone(),
            Self::Casualty(amount) => format!("Casualty {amount}"),
            Self::Conspire => "Conspire".to_string(),
            Self::Amplify(amount) => format!("Amplify {amount}"),
            Self::AuraSwap(cost) => format!("Aura swap {}", cost.to_oracle()),
            Self::Devour(amount) => format!("Devour {amount}"),
            Self::Ravenous => "Ravenous".to_string(),
            Self::Ascend => "Ascend".to_string(),
            Self::Daybound => "Daybound".to_string(),
            Self::Nightbound => "Nightbound".to_string(),
            Self::Haunt => "Haunt".to_string(),
            Self::Provoke => "Provoke".to_string(),
            Self::Undaunted => "Undaunted".to_string(),
            Self::Enlist => "Enlist".to_string(),
            Self::Extort => "Extort".to_string(),
            Self::Partner => "Partner".to_string(),
            Self::StartYourEngines => "Start your engines!".to_string(),
            Self::Assist => "Assist".to_string(),
            Self::SplitSecond => "Split second".to_string(),
            Self::Rebound => "Rebound".to_string(),
            Self::Sunburst => "Sunburst".to_string(),
            Self::ReadAhead => "Read ahead".to_string(),
            Self::Fading(amount) => format!("Fading {amount}"),
            Self::Vanishing(amount) => format!("Vanishing {amount}"),
            Self::Fear => "Fear".to_string(),
            Self::Intimidate => "Intimidate".to_string(),
            Self::Shadow => "Shadow".to_string(),
            Self::Horsemanship => "Horsemanship".to_string(),
            Self::Flanking => "Flanking".to_string(),
            Self::UmbraArmor => "Umbra armor".to_string(),
            Self::Landwalk(kind) => kind.display(),
            Self::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
            Self::Rampage(amount) => format!("Rampage {amount}"),
            Self::Bushido(amount) => format!("Bushido {amount}"),
            Self::Changeling => "Changeling".to_string(),
            Self::HexproofFrom(filter) => {
                let description = filter.description();
                let fragment = description
                    .strip_suffix(" permanent")
                    .or_else(|| description.strip_suffix(" spell"))
                    .or_else(|| description.strip_suffix(" source"))
                    .unwrap_or(description.as_str());
                format!("Hexproof from {fragment}")
            }
            Self::ProtectionFrom(colors) => single_color_name(*colors)
                .map(|name| format!("Protection from {name}"))
                .unwrap_or_else(|| "Protection from colors".to_string()),
            Self::ProtectionFromAllColors => "Protection from all colors".to_string(),
            Self::ProtectionFromColorless => "Protection from colorless".to_string(),
            Self::ProtectionFromEverything => "Protection from everything".to_string(),
            Self::ProtectionFromChosenPlayer => "Protection from the chosen player".to_string(),
            Self::ProtectionFromChosenColor => "Protection from the chosen color".to_string(),
            Self::ProtectionFromFilter(filter) => {
                format!("Protection from {}", filter.description())
            }
            Self::ProtectionFromCardType(card_type) => {
                format!(
                    "Protection from {}",
                    card_type.to_string().to_ascii_lowercase()
                )
            }
            Self::ProtectionFromSubtype(subtype) => {
                format!(
                    "Protection from {}",
                    subtype.to_string().to_ascii_lowercase()
                )
            }
            Self::Unblockable => "This can't be blocked".to_string(),
            Self::Devoid => "Devoid".to_string(),
            Self::Annihilator(amount) => format!("Annihilator {amount}"),
            Self::ForMirrodin => "For Mirrodin!".to_string(),
            Self::LivingWeapon => "Living weapon".to_string(),
            Self::Crew { amount, .. } => format!("Crew {amount}"),
            Self::Saddle { amount, .. } => format!("Saddle {amount}"),
            Self::Marker(name) => (*name).to_string(),
            Self::MarkerText(text) => text.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfResultPredicate {
    Did,
    DidNot,
    DiesThisWay,
    WasDeclined,
    Value(ironsmith_core::Comparison),
}
