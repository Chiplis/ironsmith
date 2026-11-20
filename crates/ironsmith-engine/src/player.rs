use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use std::collections::HashMap;

/// Mana pool tracking by color/type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ManaPool {
    // CR 107.1: no maximum. u64 holds stacked multipliers exactly (3^30 green
    // from thirty Nyxbloom Ancients) and saturates beyond that.
    pub white: u64,
    pub blue: u64,
    pub black: u64,
    pub red: u64,
    pub green: u64,
    pub colorless: u64,
}

/// Provenance retained for a single mana unit in a player's pool.
///
/// Ordinary mana is tracked separately from usage restrictions so effects
/// such as "for each mana from an artifact source spent to cast it" can use
/// the source's last known information even after a Treasure is sacrificed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ManaSourceProvenance {
    pub(crate) symbol: ManaSymbol,
    pub(crate) source: ObjectId,
    pub(crate) snapshot: Option<ObjectSnapshot>,
    pub(crate) restricted: bool,
    pub(crate) retention: Option<ironsmith_core::ManaRetentionDuration>,
    /// Identical units sharing this production record. Ordinary productions
    /// record one unit per entry; only a counted production (a stacked
    /// multiplier too large to materialize) records more than one.
    pub(crate) count: u64,
}

impl ManaSourceProvenance {
    /// Same production metadata, ignoring how many units remain.
    pub(crate) fn same_production(&self, other: &Self) -> bool {
        self.symbol == other.symbol
            && self.source == other.source
            && self.snapshot == other.snapshot
            && self.restricted == other.restricted
            && self.retention == other.retention
    }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManaSpendPolicy {
    pub mode: ironsmith_core::value_model::ManaSpendMode,
    pub any_color_mana_symbols: Vec<ManaSymbol>,
    pub other_mana_only_as_colorless: bool,
    /// (spent, paid) symbol conversions: "spend white mana as though it were
    /// red mana" (Sunglasses of Urza).
    #[cfg_attr(feature = "serialization", serde(default))]
    pub symbol_as_symbol: Vec<(ManaSymbol, ManaSymbol)>,
}

impl ManaSpendPolicy {
    pub fn from_any_color(allow_any_color: bool) -> Self {
        Self {
            mode: allow_any_color.into(),
            ..Self::default()
        }
    }

    pub fn allow_mode(&mut self, mode: ironsmith_core::value_model::ManaSpendMode) {
        // A single-symbol conversion ("spend colorless mana as though it
        // were mana of any color", CR 609.4b) is a per-symbol permission,
        // never a blanket mode.
        if let Some(symbol) = mode.any_color_mana_symbol() {
            self.add_symbol_as_any_color(symbol);
            return;
        }
        self.mode = self.mode.combine(mode);
    }

    pub fn add_symbol_as_any_color(&mut self, symbol: ManaSymbol) {
        if !self.any_color_mana_symbols.contains(&symbol) {
            self.any_color_mana_symbols.push(symbol);
        }
    }

    pub fn add_symbol_as_symbol(&mut self, symbol: ManaSymbol, as_symbol: ManaSymbol) {
        if !self.symbol_as_symbol.contains(&(symbol, as_symbol)) {
            self.symbol_as_symbol.push((symbol, as_symbol));
        }
    }

    pub(crate) fn has_any_color_spending(&self) -> bool {
        self.mode.allows_any_color()
            || !self.any_color_mana_symbols.is_empty()
            || !self.symbol_as_symbol.is_empty()
    }

    fn symbol_spends_as_any_color(&self, symbol: ManaSymbol) -> bool {
        self.any_color_mana_symbols.contains(&symbol)
            || (self.mode.allows_any_color() && !self.other_mana_only_as_colorless)
    }

    fn symbol_spends_as_colorless(&self, symbol: ManaSymbol) -> bool {
        symbol == ManaSymbol::Colorless
            || self.mode.allows_any_type()
            || (self.other_mana_only_as_colorless && !self.symbol_spends_as_any_color(symbol))
    }

    pub(crate) fn can_pay_symbol(&self, symbol: ManaSymbol, required: ManaSymbol) -> bool {
        match required {
            ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green => {
                // CR 609.4b: "spend other mana only as though it were
                // colorless" (Celestial Dawn) stops that mana paying even its
                // own color.
                (symbol == required
                    && (!self.other_mana_only_as_colorless
                        || self.any_color_mana_symbols.contains(&symbol)))
                    || self.symbol_spends_as_any_color(symbol)
                    || self.symbol_as_symbol.contains(&(symbol, required))
            }
            ManaSymbol::Colorless => self.symbol_spends_as_colorless(symbol),
            ManaSymbol::Generic(_) => matches!(
                symbol,
                ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless
            ),
            ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X => false,
        }
    }
}

#[derive(Clone, Copy)]
enum PoolSpending<'a> {
    Exact,
    AnyColor,
    Policy(&'a ManaSpendPolicy),
}

impl From<bool> for PoolSpending<'_> {
    fn from(any_color: bool) -> Self {
        if any_color {
            Self::AnyColor
        } else {
            Self::Exact
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PoolPaymentPass {
    ManaFirst,
    LifeFirst,
    // The affordability probe can use implicit black life after trying the
    // printed alternatives, without accumulating a life charge.
    CheckOnly,
}

impl PoolPaymentPass {
    const TRACKED: &'static [Self] = &[Self::ManaFirst, Self::LifeFirst];
    const AFFORDABILITY: &'static [Self] = &[Self::ManaFirst, Self::LifeFirst, Self::CheckOnly];
}

impl ManaPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds mana of the specified type.
    pub fn add(&mut self, symbol: ManaSymbol, amount: u32) {
        self.add_wide(symbol, u64::from(amount));
    }

    /// Adds a (possibly very large) amount of mana of the specified type.
    pub fn add_wide(&mut self, symbol: ManaSymbol, amount: u64) {
        match symbol {
            ManaSymbol::White => self.white = self.white.saturating_add(amount),
            ManaSymbol::Blue => self.blue = self.blue.saturating_add(amount),
            ManaSymbol::Black => self.black = self.black.saturating_add(amount),
            ManaSymbol::Red => self.red = self.red.saturating_add(amount),
            ManaSymbol::Green => self.green = self.green.saturating_add(amount),
            ManaSymbol::Colorless => self.colorless = self.colorless.saturating_add(amount),
            _ => {} // Generic, Snow, Life, X don't add to pool directly
        }
    }

    /// Removes mana of the specified type. Returns true if successful.
    pub fn remove(&mut self, symbol: ManaSymbol, amount: u32) -> bool {
        self.remove_wide(symbol, u64::from(amount))
    }

    /// Removes a (possibly very large) amount of mana of the specified type.
    pub fn remove_wide(&mut self, symbol: ManaSymbol, amount: u64) -> bool {
        let pool = match symbol {
            ManaSymbol::White => &mut self.white,
            ManaSymbol::Blue => &mut self.blue,
            ManaSymbol::Black => &mut self.black,
            ManaSymbol::Red => &mut self.red,
            ManaSymbol::Green => &mut self.green,
            ManaSymbol::Colorless => &mut self.colorless,
            _ => return false,
        };

        if *pool >= amount {
            *pool -= amount;
            true
        } else {
            false
        }
    }

    /// Exact (saturating u64) aggregate for checked scalar/payment boundaries.
    pub fn total_wide(&self) -> u64 {
        [self.white, self.blue, self.black, self.red, self.green, self.colorless]
            .into_iter()
            .fold(0u64, u64::saturating_add)
    }

    /// Returns the total amount of mana in the pool, saturating at u32::MAX.
    pub fn total(&self) -> u32 {
        u32::try_from(self.total_wide()).unwrap_or(u32::MAX)
    }

    /// Empties the mana pool.
    pub fn empty(&mut self) {
        self.white = 0;
        self.blue = 0;
        self.black = 0;
        self.red = 0;
        self.green = 0;
        self.colorless = 0;
    }

    /// Returns the amount of mana of a specific type, saturating at u32::MAX.
    pub fn amount(&self, symbol: ManaSymbol) -> u32 {
        u32::try_from(self.amount_wide(symbol)).unwrap_or(u32::MAX)
    }

    /// Returns the exact amount of mana of a specific type.
    pub fn amount_wide(&self, symbol: ManaSymbol) -> u64 {
        match symbol {
            ManaSymbol::White => self.white,
            ManaSymbol::Blue => self.blue,
            ManaSymbol::Black => self.black,
            ManaSymbol::Red => self.red,
            ManaSymbol::Green => self.green,
            ManaSymbol::Colorless => self.colorless,
            _ => 0,
        }
    }

    /// Check if this pool can pay a mana cost with X=x_value.
    ///
    /// Returns true if the cost can be paid with some valid combination.
    /// For Phyrexian mana (pips with life alternatives), this considers
    /// both mana payment and life payment options.
    pub fn can_pay(&self, cost: &crate::mana::ManaCost, x_value: u32) -> bool {
        self.can_pay_with_any_color(cost, x_value, false)
    }

    /// Check if this pool can pay a mana cost with X=x_value, optionally
    /// allowing mana to be spent as though it were mana of any color.
    pub fn can_pay_with_any_color(
        &self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
    ) -> bool {
        self.can_pay_with_any_color_and_black_life(cost, x_value, allow_any_color, false)
    }

    /// Check if this pool can pay a mana cost with X=x_value, optionally
    /// allowing mana to be spent as though it were any color and singleton
    /// black pips to be paid with 2 life (for effects like K'rrik).
    pub fn can_pay_with_any_color_and_black_life(
        &self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
        allow_black_life: bool,
    ) -> bool {
        self.clone()
            .pay_from_pool(
                &Self::ordered_payment_pips(cost),
                x_value,
                allow_any_color.into(),
                allow_black_life,
                PoolPaymentPass::AFFORDABILITY,
            )
            .0
    }

    /// Try to pay a mana cost with X=x_value.
    ///
    /// Returns true if successful (and the pool is modified).
    /// Returns false if payment fails (pool is left in undefined state).
    pub fn try_pay(&mut self, cost: &crate::mana::ManaCost, x_value: u32) -> bool {
        self.try_pay_with_any_color(cost, x_value, false)
    }

    /// Try to pay a mana cost with X=x_value, tracking life payment for Phyrexian mana.
    ///
    /// Returns (success, life_to_pay) where:
    /// - success: true if payment succeeded (pool is modified), false otherwise
    /// - life_to_pay: amount of life that should be deducted for Phyrexian mana pips
    ///   that couldn't be paid with mana
    ///
    /// For Phyrexian mana (e.g., {B/P}), this tries mana-first strategy. If that fails
    /// (e.g., using mana for {B/P} leaves insufficient mana for X), it falls back to
    /// life-first strategy where Phyrexian pips prefer life payment to preserve mana.
    pub fn try_pay_tracking_life(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> (bool, u32) {
        self.try_pay_tracking_life_with_any_color(cost, x_value, false)
    }

    /// Try to pay a mana cost with X=x_value, tracking life payment and optionally
    /// allowing mana to be spent as though it were any color.
    pub fn try_pay_tracking_life_with_any_color(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
    ) -> (bool, u32) {
        self.try_pay_tracking_life_with_any_color_and_black_life(
            cost,
            x_value,
            allow_any_color,
            false,
        )
    }

    /// Try to pay a mana cost with X=x_value, tracking life payment and optionally
    /// allowing mana to be spent as though it were any color and singleton black
    /// pips to be paid with 2 life.
    pub fn try_pay_tracking_life_with_any_color_and_black_life(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
        allow_black_life: bool,
    ) -> (bool, u32) {
        self.pay_from_pool(
            &Self::ordered_payment_pips(cost),
            x_value,
            allow_any_color.into(),
            allow_black_life,
            PoolPaymentPass::TRACKED,
        )
    }

    /// Try to pay a mana cost with X=x_value, optionally allowing any color spending.
    pub fn try_pay_with_any_color(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
    ) -> bool {
        self.try_pay_with_any_color_and_black_life(cost, x_value, allow_any_color, false)
    }

    /// Try to pay a mana cost with X=x_value, optionally allowing any color spending
    /// and singleton black pips to be paid with 2 life.
    pub fn try_pay_with_any_color_and_black_life(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
        allow_black_life: bool,
    ) -> bool {
        self.try_pay_tracking_life_with_any_color_and_black_life(
            cost,
            x_value,
            allow_any_color,
            allow_black_life,
        )
        .0
    }

    pub fn try_pay_tracking_life_with_mana_spend_policy_and_black_life(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        policy: &ManaSpendPolicy,
        allow_black_life: bool,
    ) -> (bool, u32) {
        self.pay_from_pool(
            &Self::ordered_payment_pips(cost),
            x_value,
            PoolSpending::Policy(policy),
            allow_black_life,
            PoolPaymentPass::TRACKED,
        )
    }

    pub fn max_x_for_cost_with_mana_spend_policy_and_black_life(
        &self,
        cost: &crate::mana::ManaCost,
        policy: &ManaSpendPolicy,
        allow_black_life: bool,
    ) -> u32 {
        let x_pip_count = cost.x_pip_count();
        let search_divisor = x_pip_count as u32;
        if search_divisor == 0 {
            return 0;
        }

        let max_candidate = self.total() / search_divisor;
        let pips = Self::ordered_payment_pips(cost);
        let pay = |pool: &mut Self, x, passes: &[PoolPaymentPass]| {
            pool.pay_from_pool(
                &pips,
                x,
                PoolSpending::Policy(policy),
                allow_black_life,
                passes,
            )
            .0
        };

        // With only standalone generic/X requirements, every other choice is
        // made before spending on X. What remains at X=0 is the entire X budget.
        if self.total_wide() <= u64::from(u32::MAX)
            && pips
                .iter()
                .all(|pip| pip.len() == 1 || !Self::has_generic_or_x(pip))
        {
            let mut best = 0;
            for pass in PoolPaymentPass::TRACKED {
                let mut remaining = self.clone();
                if pay(&mut remaining, 0, std::slice::from_ref(pass)) {
                    let x = remaining.total() as usize / x_pip_count;
                    best = best.max(x as u32);
                    if best == max_candidate {
                        break;
                    }
                }
            }
            return best;
        }

        // Mixed pips can switch alternatives as X increases, leaving gaps in
        // payability. Oversized pools also retain the caller's u32 search bound.
        (0..=max_candidate)
            .rev()
            .find(|&x| pay(&mut self.clone(), x, PoolPaymentPass::TRACKED))
            .unwrap_or(0)
    }

    fn ordered_payment_pips(cost: &crate::mana::ManaCost) -> Vec<&[ManaSymbol]> {
        // Keep printed order within each group. Generic and X pips can spend
        // mana needed by a more constrained pip and are considered last.
        let mut pips: Vec<_> = cost.pips().iter().map(Vec::as_slice).collect();
        pips.sort_by_key(|pip| Self::has_generic_or_x(pip));
        pips
    }

    fn has_generic_or_x(pip: &[ManaSymbol]) -> bool {
        pip.iter()
            .any(|s| matches!(s, ManaSymbol::Generic(_) | ManaSymbol::X))
    }

    fn pay_from_pool(
        &mut self,
        pips: &[&[ManaSymbol]],
        x_value: u32,
        spending: PoolSpending<'_>,
        allow_black_life: bool,
        passes: &[PoolPaymentPass],
    ) -> (bool, u32) {
        let original = self.clone();
        for (index, &pass) in passes.iter().enumerate() {
            if index != 0 {
                *self = original.clone();
            }
            let life = pips.iter().try_fold(0u32, |total, pip| {
                let life = self.pay_pip(pip, x_value, spending, allow_black_life, pass)?;
                Some(if pass == PoolPaymentPass::CheckOnly {
                    0
                } else {
                    total.saturating_add(life)
                })
            });
            if let Some(life) = life {
                return (true, life);
            }
        }
        (false, 0)
    }

    /// Pay one pip and return the life charge for the selected alternative.
    fn pay_pip(
        &mut self,
        pip: &[ManaSymbol],
        x_value: u32,
        spending: PoolSpending<'_>,
        allow_black_life: bool,
        pass: PoolPaymentPass,
    ) -> Option<u32> {
        let black_life = allow_black_life && pip == [ManaSymbol::Black];
        if pass == PoolPaymentPass::LifeFirst {
            if black_life {
                return Some(2);
            }
            if let Some(life) = pip.iter().find_map(|s| match s {
                ManaSymbol::Life(n) => Some(u32::from(*n)),
                _ => None,
            }) {
                return Some(life);
            }
        }

        for &symbol in pip {
            let paid = match symbol {
                ManaSymbol::Life(n) => return Some(u32::from(n)),
                ManaSymbol::Generic(n) => self.pay_generic(u32::from(n)),
                ManaSymbol::X => self.pay_generic(x_value),
                // Only GameState can verify a mana unit's snow provenance.
                ManaSymbol::Snow => false,
                _ => self.pay_typed_mana(symbol, spending),
            };
            if paid {
                return Some(0);
            }
        }
        // The ordinary any-color payer retries from the original pool before
        // substituting life for a singleton black pip.
        (black_life
            && !(matches!(spending, PoolSpending::AnyColor) && pass == PoolPaymentPass::ManaFirst))
            .then_some(2)
    }

    fn pay_typed_mana(&mut self, required: ManaSymbol, spending: PoolSpending<'_>) -> bool {
        match spending {
            PoolSpending::AnyColor if required != ManaSymbol::Colorless => self.pay_generic(1),
            PoolSpending::Exact | PoolSpending::AnyColor => self.remove(required, 1),
            PoolSpending::Policy(policy) => [
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
                ManaSymbol::Colorless,
            ]
            .into_iter()
            .any(|symbol| {
                self.amount(symbol) > 0
                    && policy.can_pay_symbol(symbol, required)
                    && self.remove(symbol, 1)
            }),
        }
    }

    /// Pay generic mana using colorless first, then colored mana in WUBRG order.
    fn pay_generic(&mut self, amount: u32) -> bool {
        let mut amount = u64::from(amount);
        if self.total_wide() < amount {
            return false;
        }
        for pool in [
            &mut self.colorless,
            &mut self.white,
            &mut self.blue,
            &mut self.black,
            &mut self.red,
            &mut self.green,
        ] {
            let from_this = amount.min(*pool);
            *pool -= from_this;
            amount -= from_this;
            if amount == 0 {
                break;
            }
        }
        true
    }

    /// Calculate the maximum X value that can be paid given a mana cost.
    ///
    /// This finds the highest X where can_pay(cost, X) returns true.
    pub fn max_x_for_cost(&self, cost: &crate::mana::ManaCost) -> u32 {
        self.max_x_for_cost_with_any_color(cost, false)
    }

    /// Calculate the maximum X value that can be paid given a mana cost,
    /// optionally allowing mana to be spent as though it were any color.
    pub fn max_x_for_cost_with_any_color(
        &self,
        cost: &crate::mana::ManaCost,
        allow_any_color: bool,
    ) -> u32 {
        self.max_x_for_cost_with_any_color_and_black_life(cost, allow_any_color, false)
    }

    /// Calculate the maximum X value that can be paid given a mana cost,
    /// optionally allowing any-color spending and singleton black pips to be
    /// paid with 2 life.
    pub fn max_x_for_cost_with_any_color_and_black_life(
        &self,
        cost: &crate::mana::ManaCost,
        allow_any_color: bool,
        allow_black_life: bool,
    ) -> u32 {
        let policy = ManaSpendPolicy::from_any_color(allow_any_color);
        self.max_x_for_cost_with_mana_spend_policy_and_black_life(cost, &policy, allow_black_life)
    }
}

/// Complete player state.
#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,

    // Life and resources
    pub starting_life: i32,
    pub life: i32,
    pub mana_pool: ManaPool,
    pub restricted_mana: Vec<crate::ability::RestrictedManaUnit>,
    pub(crate) mana_source_provenance: Vec<ManaSourceProvenance>,
    pub poison_counters: u32,
    pub energy_counters: u32,
    pub experience_counters: u32,
    /// Player counters without dedicated rules fields (for example, rad counters).
    pub other_counters: std::collections::BTreeMap<crate::object::CounterType, u32>,
    pub speed: Option<u8>,
    pub ring_temptations: u32,
    pub ring_bearer: Option<ObjectId>,
    pub ring_legendary_added: Option<ObjectId>,

    // Per-turn tracking
    pub lands_played_this_turn: u32,
    /// Land allowances consumed by casts, without an actual land-play event.
    pub nonland_land_play_usage_this_turn: u32,
    pub land_plays_per_turn: u32,

    // Game-scoped tracking
    pub spells_cast_this_game: u32,
    /// Name at casting time, keyed by the exact cast spell incarnation.
    pub spell_cast_names_this_game: std::collections::BTreeMap<ObjectId, String>,
    /// Creature spells this player has cast this game ("until a player
    /// casts a creature spell").
    pub creature_spells_cast_this_game: u32,
    /// Times this player has cycled a card with each name this game
    /// ("if you've cycled a card named ... four or more times this game").
    pub cycled_card_names_this_game: std::collections::BTreeMap<String, u32>,

    // Hand size
    pub max_hand_size: i32,

    // Game status
    pub has_lost: bool,
    pub has_won: bool,
    pub has_left_game: bool,
    /// This player attempted an unreplaced draw from an empty library since
    /// the last state-based-action pass.
    pub attempted_draw_from_empty_library: bool,

    // Zones (stored as object IDs)
    pub library: crate::zone_sequence::ZoneSequence,
    pub hand: crate::zone_sequence::ZoneSequence,
    pub graveyard: crate::zone_sequence::ZoneSequence,
    pub sideboard: crate::zone_sequence::ZoneSequence,
    pub attachments: Vec<ObjectId>,

    // Companion tracking (CR 103.2b, 116.2g, 702.139)
    /// The one outside-game card this player revealed as their companion.
    pub companion: Option<ObjectId>,
    /// Whether this player has taken the companion special action this game.
    pub companion_special_action_used: bool,

    // Commander tracking
    /// The card IDs of this player's commanders.
    /// Supports Partner mechanic (multiple commanders).
    /// Commanders can be legendary creatures, planeswalkers with "can be your commander",
    /// or certain artifacts (per recent rule changes).
    pub commanders: Vec<ObjectId>,
    /// Each commander's color identity, fixed when it was designated
    /// (CR 903.4a). Keyed like `commanders`; ordered for determinism.
    pub commander_color_identities: std::collections::BTreeMap<ObjectId, crate::color::ColorSet>,

    // Commander damage tracking (commander identity -> damage)
    pub commander_damage: HashMap<ObjectId, u32>,
    /// Cards in this player's starting deck (CR 103.2; commanders included,
    /// CR 903.5a), recorded when the match starts. `None` for states built
    /// without a deck setup.
    pub starting_deck_size: Option<u32>,
}

impl Player {
    pub fn new(id: PlayerId, name: impl Into<String>, starting_life: i32) -> Self {
        Self {
            id,
            name: name.into(),
            starting_life,
            life: starting_life,
            mana_pool: ManaPool::new(),
            restricted_mana: Vec::new(),
            mana_source_provenance: Vec::new(),
            poison_counters: 0,
            energy_counters: 0,
            experience_counters: 0,
            other_counters: std::collections::BTreeMap::new(),
            speed: None,
            ring_temptations: 0,
            ring_bearer: None,
            ring_legendary_added: None,
            lands_played_this_turn: 0,
            nonland_land_play_usage_this_turn: 0,
            land_plays_per_turn: 1,
            spells_cast_this_game: 0,
            spell_cast_names_this_game: std::collections::BTreeMap::new(),
            creature_spells_cast_this_game: 0,
            cycled_card_names_this_game: std::collections::BTreeMap::new(),
            max_hand_size: 7,
            has_lost: false,
            has_won: false,
            has_left_game: false,
            attempted_draw_from_empty_library: false,
            library: crate::zone_sequence::ZoneSequence::new(),
            hand: crate::zone_sequence::ZoneSequence::new(),
            graveyard: crate::zone_sequence::ZoneSequence::new(),
            sideboard: crate::zone_sequence::ZoneSequence::new(),
            attachments: Vec::new(),
            companion: None,
            companion_special_action_used: false,
            commanders: Vec::new(),
            commander_color_identities: std::collections::BTreeMap::new(),
            commander_damage: HashMap::new(),
            starting_deck_size: None,
        }
    }

    /// Sets the commanders for this player.
    /// Supports Partner mechanic with multiple commanders.
    pub fn set_commanders(&mut self, commander_ids: Vec<ObjectId>) {
        self.commanders = commander_ids;
    }

    /// Adds a commander to this player's commander list.
    pub fn add_commander(&mut self, commander_id: ObjectId) {
        if !self.commanders.contains(&commander_id) {
            self.commanders.push(commander_id);
        }
    }

    /// Returns true if the given object ID is one of this player's commanders.
    pub fn is_commander(&self, object_id: ObjectId) -> bool {
        self.commanders.contains(&object_id)
    }

    /// Returns the list of commander IDs.
    pub fn get_commanders(&self) -> &[ObjectId] {
        &self.commanders
    }

    /// Records combat damage dealt to this player by a commander.
    pub fn record_commander_damage(&mut self, commander: ObjectId, amount: u32) {
        let total = self.commander_damage.entry(commander).or_insert(0);
        *total = total.saturating_add(amount);
    }

    pub(crate) fn add_unrestricted_mana(
        &mut self,
        symbol: ManaSymbol,
        source: ObjectId,
        snapshot: Option<ObjectSnapshot>,
    ) {
        self.add_unrestricted_mana_with_retention(symbol, source, snapshot, None);
    }

    pub(crate) fn add_unrestricted_mana_with_retention(
        &mut self,
        symbol: ManaSymbol,
        source: ObjectId,
        snapshot: Option<ObjectSnapshot>,
        retention: Option<ironsmith_core::ManaRetentionDuration>,
    ) {
        self.add_unrestricted_mana_counted(symbol, 1, source, snapshot, retention);
    }

    /// Credit `count` identical unrestricted units as one production record,
    /// so a geometric multiplier never materializes one record per unit.
    pub(crate) fn add_unrestricted_mana_counted(
        &mut self,
        symbol: ManaSymbol,
        count: u64,
        source: ObjectId,
        snapshot: Option<ObjectSnapshot>,
        retention: Option<ironsmith_core::ManaRetentionDuration>,
    ) {
        if count == 0 {
            return;
        }
        let before = self.mana_pool.amount_wide(symbol);
        self.mana_pool.add_wide(symbol, count);
        // A saturated pool records only the units it actually holds.
        let added = self.mana_pool.amount_wide(symbol).saturating_sub(before);
        if added == 0 {
            return;
        }
        self.mana_source_provenance.push(ManaSourceProvenance {
            symbol,
            source,
            snapshot,
            restricted: false,
            retention,
            count: added,
        });
    }

    pub fn add_restricted_mana(&mut self, unit: crate::ability::RestrictedManaUnit) {
        self.add_restricted_mana_with_snapshot(unit, None);
    }

    pub(crate) fn add_restricted_mana_with_snapshot(
        &mut self,
        unit: crate::ability::RestrictedManaUnit,
        snapshot: Option<ObjectSnapshot>,
    ) {
        self.add_restricted_mana_with_snapshot_and_retention(unit, snapshot, None);
    }

    pub(crate) fn add_restricted_mana_with_snapshot_and_retention(
        &mut self,
        unit: crate::ability::RestrictedManaUnit,
        snapshot: Option<ObjectSnapshot>,
        retention: Option<ironsmith_core::ManaRetentionDuration>,
    ) {
        self.mana_pool.add(unit.symbol, 1);
        self.mana_source_provenance.push(ManaSourceProvenance {
            symbol: unit.symbol,
            source: unit.source,
            snapshot,
            restricted: true,
            retention,
            count: 1,
        });
        self.restricted_mana.push(unit);
    }

    pub(crate) fn take_mana_source_provenance(
        &mut self,
        symbol: ManaSymbol,
        restricted: bool,
        source: Option<ObjectId>,
    ) -> Option<ManaSourceProvenance> {
        let index = self
            .mana_source_provenance
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.symbol == symbol
                    && unit.restricted == restricted
                    && source.is_none_or(|source| unit.source == source)
            })
            // Spend ordinary mana before longer-lived mana when both can pay.
            .min_by_key(|(_, unit)| unit.retention.is_some())
            .map(|(index, _)| index)?;
        let record = &mut self.mana_source_provenance[index];
        if record.count > 1 {
            // Split one unit off a counted record.
            record.count -= 1;
            let mut unit = record.clone();
            unit.count = 1;
            return Some(unit);
        }
        Some(self.mana_source_provenance.remove(index))
    }

    /// Remove `count` units from the provenance record at `index`, dropping the
    /// record once it is exhausted. Returns false for an invalid index.
    pub(crate) fn consume_mana_source_provenance(&mut self, index: usize, count: u64) -> bool {
        let Some(record) = self.mana_source_provenance.get_mut(index) else {
            return false;
        };
        if record.count > count {
            record.count -= count;
        } else {
            self.mana_source_provenance.remove(index);
        }
        true
    }

    /// Count-only wire pools do not preserve production snapshots, exact
    /// restriction pairing or per-unit retention. Local clones retain these.
    pub fn has_runtime_mana_provenance(&self) -> bool {
        !self.mana_source_provenance.is_empty() || !self.restricted_mana.is_empty()
    }

    pub(crate) fn clear_mana_source_provenance(&mut self) {
        self.mana_source_provenance.clear();
    }

    /// Discard provenance entries that no longer have a corresponding unit in
    /// the aggregate mana pool. This is used by legacy bulk-payment paths
    /// which update `ManaPool` directly rather than selecting units one by one.
    pub(crate) fn trim_mana_source_provenance_to_pool(&mut self) {
        let mut remaining = self.mana_pool.clone();
        self.mana_source_provenance.retain_mut(|unit| {
            let available = remaining.amount_wide(unit.symbol);
            if available == 0 || unit.count == 0 {
                return false;
            }
            unit.count = unit.count.min(available);
            remaining.remove_wide(unit.symbol, unit.count)
        });
    }

    /// Returns the combat damage this player has taken from a commander.
    pub fn commander_damage_from(&self, commander: ObjectId) -> u32 {
        self.commander_damage.get(&commander).copied().unwrap_or(0)
    }

    /// Returns true if this player can play a land this turn.
    pub fn can_play_land(&self) -> bool {
        self.lands_played_this_turn.saturating_add(self.nonland_land_play_usage_this_turn) < self.land_plays_per_turn
    }

    /// Deals damage to this player. Returns the actual damage dealt.
    pub fn deal_damage(&mut self, amount: u32) -> u32 {
        self.life = ironsmith_core::quantity::clamp_i32(i64::from(self.life) - i64::from(amount));
        amount
    }

    /// Gains life.
    pub fn gain_life(&mut self, amount: u32) {
        self.life = ironsmith_core::quantity::clamp_i32(i64::from(self.life) + i64::from(amount));
    }

    /// Loses life (different from damage - can't be prevented/redirected the same way).
    pub fn lose_life(&mut self, amount: u32) {
        self.life = ironsmith_core::quantity::clamp_i32(i64::from(self.life) - i64::from(amount));
    }

    pub fn start_engines(&mut self) -> bool {
        if self.speed.is_some() {
            return false;
        }
        self.speed = Some(1);
        true
    }

    pub fn increase_speed(&mut self, amount: u32) -> u32 {
        if amount == 0 {
            return 0;
        }
        let old_speed = self.speed.unwrap_or(0);
        let amount = amount.min(4) as u8;
        let new_speed = old_speed.saturating_add(amount).clamp(1, 4);
        self.speed = Some(new_speed);
        u32::from(new_speed.saturating_sub(old_speed))
    }

    pub fn reduce_speed(&mut self, amount: u32, minimum: u8) -> u32 {
        let Some(old_speed) = self.speed else {
            return 0;
        };
        if amount == 0 || old_speed <= minimum {
            return 0;
        }
        let amount = amount.min(4) as u8;
        let new_speed = old_speed.saturating_sub(amount).max(minimum);
        self.speed = Some(new_speed);
        u32::from(old_speed.saturating_sub(new_speed))
    }

    /// Adds poison counters. Returns the new total.
    pub fn add_poison(&mut self, amount: u32) -> u32 {
        self.poison_counters = self.poison_counters.saturating_add(amount);
        self.poison_counters
    }

    pub fn counter_count(&self, counter_type: crate::object::CounterType) -> u32 {
        match counter_type {
            crate::object::CounterType::Poison => self.poison_counters,
            crate::object::CounterType::Energy => self.energy_counters,
            crate::object::CounterType::Experience => self.experience_counters,
            _ => self.other_counters.get(&counter_type).copied().unwrap_or(0),
        }
    }

    pub fn counter_types_with_counters(&self) -> Vec<crate::object::CounterType> {
        let mut counter_types = Vec::new();
        for counter_type in [
            crate::object::CounterType::Poison,
            crate::object::CounterType::Energy,
            crate::object::CounterType::Experience,
        ] {
            if self.counter_count(counter_type) > 0 {
                counter_types.push(counter_type);
            }
        }
        let mut other = self
            .other_counters
            .iter()
            .filter_map(|(counter_type, count)| (*count > 0).then_some(*counter_type))
            .collect::<Vec<_>>();
        other.sort_by_key(|counter_type| counter_type.description().into_owned());
        counter_types.extend(other);
        counter_types
    }

    pub(crate) fn add_counters(&mut self, counter_type: crate::object::CounterType, amount: u32) {
        match counter_type {
            crate::object::CounterType::Poison => {
                self.poison_counters = self.poison_counters.saturating_add(amount);
            }
            crate::object::CounterType::Energy => {
                self.energy_counters = self.energy_counters.saturating_add(amount);
            }
            crate::object::CounterType::Experience => {
                self.experience_counters = self.experience_counters.saturating_add(amount);
            }
            _ => {
                let count = self.other_counters.entry(counter_type).or_default();
                *count = count.saturating_add(amount);
            }
        }
    }

    pub(crate) fn remove_counters(
        &mut self,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) -> u32 {
        let available = self.counter_count(counter_type);
        let removed = available.min(amount);
        match counter_type {
            crate::object::CounterType::Poison => {
                self.poison_counters = self.poison_counters.saturating_sub(removed);
            }
            crate::object::CounterType::Energy => {
                self.energy_counters = self.energy_counters.saturating_sub(removed);
            }
            crate::object::CounterType::Experience => {
                self.experience_counters = self.experience_counters.saturating_sub(removed);
            }
            _ => {
                let remove_entry = if let Some(count) = self.other_counters.get_mut(&counter_type) {
                    *count = count.saturating_sub(removed);
                    *count == 0
                } else {
                    false
                };
                if remove_entry {
                    self.other_counters.remove(&counter_type);
                }
            }
        }
        removed
    }

    /// Draws cards from library to hand. Returns the IDs of cards drawn.
    pub fn draw(&mut self, count: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(card_id) = self.library.pop() {
                self.hand.push(card_id);
                drawn.push(card_id);
            } else {
                self.attempted_draw_from_empty_library = true;
            }
        }
        drawn
    }

    /// Called at the beginning of this player's turn.
    pub fn begin_turn(&mut self) {
        self.lands_played_this_turn = 0;
        self.nonland_land_play_usage_this_turn = 0;
    }

    /// Consume an allowance without claiming that a land was played.
    pub fn consume_land_play_for_cast(&mut self) {
        self.nonland_land_play_usage_this_turn = self.nonland_land_play_usage_this_turn.saturating_add(1);
    }

    /// Records a land play.
    pub fn record_land_play(&mut self) {
        self.lands_played_this_turn = self.lands_played_this_turn.saturating_add(1);
    }

    /// Returns true if this player is still in the game.
    pub fn is_in_game(&self) -> bool {
        !self.has_lost && !self.has_won && !self.has_left_game
    }

    /// Checks if this player should lose due to poison counters.
    pub fn has_lethal_poison(&self) -> bool {
        self.poison_counters >= 10
    }

    /// Checks if this player should lose due to life total.
    pub fn has_lethal_life(&self) -> bool {
        self.life <= 0
    }

    /// Returns the number of cards in hand.
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }

    /// Returns the number of cards in library.
    pub fn library_size(&self) -> usize {
        self.library.len()
    }

    /// Shuffles the library.
    ///
    /// This derives a deterministic seed from the current library contents so
    /// direct engine consumers don't depend on ambient process RNG state.
    pub fn shuffle_library(&mut self) {
        use rand::seq::SliceRandom;

        let mut seed = 0xcbf2_9ce4_8422_2325u64;
        for object_id in &self.library {
            seed ^= object_id.0;
            seed = seed.wrapping_mul(0x1000_0000_01b3);
        }
        if seed == 0 {
            seed = 0x9e37_79b9_7f4a_7c15;
        }

        let mut rng = ChaCha12Rng::seed_from_u64(seed);
        self.library.with_vec_mut(|ids| ids.shuffle(&mut rng));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mana_pool() {
        let mut pool = ManaPool::new();
        assert_eq!(pool.total(), 0);

        pool.add(ManaSymbol::White, 2);
        pool.add(ManaSymbol::Blue, 1);
        assert_eq!(pool.total(), 3);
        assert_eq!(pool.amount(ManaSymbol::White), 2);
        assert_eq!(pool.amount(ManaSymbol::Blue), 1);

        assert!(pool.remove(ManaSymbol::White, 1));
        assert_eq!(pool.amount(ManaSymbol::White), 1);

        assert!(!pool.remove(ManaSymbol::White, 5)); // Can't remove more than available
        assert_eq!(pool.amount(ManaSymbol::White), 1); // Unchanged

        pool.empty();
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn any_type_but_not_any_color_can_pay_colorless_requirements() {
        let mut any_color = ManaSpendPolicy::default();
        any_color.allow_mode(ironsmith_core::value_model::ManaSpendMode::AnyColor);
        assert!(any_color.can_pay_symbol(ManaSymbol::Red, ManaSymbol::Blue));
        assert!(!any_color.can_pay_symbol(ManaSymbol::Red, ManaSymbol::Colorless));

        let mut any_type = ManaSpendPolicy::default();
        any_type.allow_mode(ironsmith_core::value_model::ManaSpendMode::AnyType);
        assert!(any_type.can_pay_symbol(ManaSymbol::Red, ManaSymbol::Blue));
        assert!(any_type.can_pay_symbol(ManaSymbol::Red, ManaSymbol::Colorless));
    }

    #[test]
    fn test_player_creation() {
        let player = Player::new(PlayerId::from_index(0), "Alice", 20);
        assert_eq!(player.life, 20);
        assert_eq!(player.name, "Alice");
        assert!(player.can_play_land());
        assert!(player.is_in_game());
    }

    #[test]
    fn test_damage_and_life() {
        let mut player = Player::new(PlayerId::from_index(0), "Bob", 20);

        player.deal_damage(5);
        assert_eq!(player.life, 15);

        player.gain_life(3);
        assert_eq!(player.life, 18);

        player.lose_life(2);
        assert_eq!(player.life, 16);

        // Deal lethal damage
        player.deal_damage(20);
        assert!(player.has_lethal_life());
    }

    #[test]
    fn test_poison() {
        let mut player = Player::new(PlayerId::from_index(0), "Charlie", 20);

        player.add_poison(5);
        assert!(!player.has_lethal_poison());

        player.add_poison(5);
        assert!(player.has_lethal_poison());
    }

    #[test]
    fn test_land_plays() {
        let mut player = Player::new(PlayerId::from_index(0), "Diana", 20);

        assert!(player.can_play_land());
        player.record_land_play();
        assert!(!player.can_play_land());

        player.begin_turn();
        assert!(player.can_play_land());
    }

    #[test]
    fn test_draw_cards() {
        let mut player = Player::new(PlayerId::from_index(0), "Eve", 20);

        // Add some cards to library
        player.library.push(ObjectId::from_raw(1));
        player.library.push(ObjectId::from_raw(2));
        player.library.push(ObjectId::from_raw(3));

        assert_eq!(player.library_size(), 3);
        assert_eq!(player.hand_size(), 0);

        let drawn = player.draw(2);
        assert_eq!(drawn.len(), 2);
        assert_eq!(player.library_size(), 1);
        assert_eq!(player.hand_size(), 2);

        // Drawing from top means we got the last cards added
        assert_eq!(drawn[0], ObjectId::from_raw(3));
        assert_eq!(drawn[1], ObjectId::from_raw(2));
    }

    #[test]
    fn test_draw_from_empty_library() {
        let mut player = Player::new(PlayerId::from_index(0), "Frank", 20);

        player.library.push(ObjectId::from_raw(1));

        let drawn = player.draw(5); // Try to draw more than available
        assert_eq!(drawn.len(), 1);
        assert_eq!(player.library_size(), 0);
        assert!(player.attempted_draw_from_empty_library);
    }

    #[test]
    fn test_commander_damage() {
        let mut player = Player::new(PlayerId::from_index(0), "Grace", 40);

        player.commander_damage.insert(ObjectId::from_raw(101), 10);
        player.commander_damage.insert(ObjectId::from_raw(202), 5);

        assert_eq!(
            player.commander_damage.get(&ObjectId::from_raw(101)),
            Some(&10)
        );
        assert_eq!(
            player.commander_damage.get(&ObjectId::from_raw(202)),
            Some(&5)
        );
    }

    #[test]
    fn test_phyrexian_mana_prefers_mana_over_life() {
        use crate::mana::ManaCost;

        // Phyrexian mana cost: {{B/P}} - can be paid with Black OR 2 life
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]]);

        // Case 1: Have Black mana - should use mana, no life cost
        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Black, 1);
        let (success, life_to_pay) = pool.try_pay_tracking_life(&cost, 0);
        assert!(success, "Should be able to pay {{B/P}} with Black mana");
        assert_eq!(
            life_to_pay, 0,
            "Should not pay life when Black mana is available"
        );
        assert_eq!(pool.black, 0, "Black mana should be consumed");

        // Case 2: No mana - should fall back to life
        let mut pool = ManaPool::new();
        let (success, life_to_pay) = pool.try_pay_tracking_life(&cost, 0);
        assert!(success, "Should be able to pay {{B/P}} with life");
        assert_eq!(life_to_pay, 2, "Should pay 2 life when no mana available");
    }

    #[test]
    fn test_single_black_pip_can_use_krrik_life_without_rewriting_cost() {
        use crate::mana::ManaCost;

        let cost = ManaCost::from_symbols(vec![ManaSymbol::Black]);

        let mut pool = ManaPool::new();
        let (success, life_to_pay) =
            pool.try_pay_tracking_life_with_any_color_and_black_life(&cost, 0, false, true);
        assert!(
            success,
            "Krrik-style life payment should cover a plain {{B}} pip"
        );
        assert_eq!(life_to_pay, 2, "the plain black pip should cost 2 life");
    }

    #[test]
    fn any_color_payment_preserves_each_callers_pool_order() {
        let cost = crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Blue]);
        let mut ordinary = ManaPool {
            white: 1,
            colorless: 1,
            ..ManaPool::default()
        };
        let mut policy = ordinary.clone();
        assert!(ordinary.try_pay_with_any_color(&cost, 0, true));
        assert_eq!((ordinary.white, ordinary.colorless), (1, 0));
        assert_eq!(
            policy.try_pay_tracking_life_with_mana_spend_policy_and_black_life(
                &cost,
                0,
                &ManaSpendPolicy::from_any_color(true),
                false,
            ),
            (true, 0),
        );
        assert_eq!((policy.white, policy.colorless), (0, 1));
    }

    #[test]
    fn implicit_black_life_can_trigger_a_full_pool_retry() {
        use ManaSymbol::{Black, Life, Red};
        let cost = crate::mana::ManaCost::from_pips(vec![vec![Red, Life(2)], vec![Black]]);
        let mut ordinary = ManaPool {
            red: 1,
            ..ManaPool::default()
        };
        let mut policy = ordinary.clone();
        assert_eq!(
            ordinary.try_pay_tracking_life_with_any_color_and_black_life(&cost, 0, true, true),
            (true, 4),
        );
        assert_eq!(ordinary.red, 1);
        assert_eq!(
            policy.try_pay_tracking_life_with_mana_spend_policy_and_black_life(
                &cost,
                0,
                &ManaSpendPolicy::from_any_color(true),
                true,
            ),
            (true, 2),
        );
        assert_eq!(policy.red, 0);
    }

    #[test]
    fn test_phyrexian_mana_with_x_cost() {
        use crate::mana::ManaCost;

        // Hex Parasite's cost: {{X}}, {{B/P}}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Black, ManaSymbol::Life(2)],
        ]);

        // With Black + Red mana: X=1, pay Black for {{B/P}}
        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Black, 1);
        pool.add(ManaSymbol::Red, 1);

        let (success, life_to_pay) = pool.try_pay_tracking_life(&cost, 1);
        assert!(success, "Should be able to pay {{X}}{{B/P}} with X=1");
        assert_eq!(life_to_pay, 0, "Should use Black mana, not life");
        assert_eq!(pool.black, 0, "Black should be consumed for {{B/P}}");
        assert_eq!(pool.red, 0, "Red should be consumed for X=1");

        // With only Red mana: X=1, pay life for {{B/P}}
        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Red, 1);

        let (success, life_to_pay) = pool.try_pay_tracking_life(&cost, 1);
        assert!(
            success,
            "Should be able to pay {{X}}{{B/P}} with X=1 using life for {{B/P}}"
        );
        assert_eq!(life_to_pay, 2, "Should pay 2 life for {{B/P}}");
        assert_eq!(pool.red, 0, "Red should be consumed for X=1");
    }

    #[test]
    fn max_x_shares_remaining_mana_between_standalone_x_pips() {
        use ManaSymbol::{Generic, X};
        let cost = crate::mana::ManaCost::from_symbols(vec![X, Generic(5), X]);
        let pool = ManaPool {
            white: u64::from(u32::MAX),
            ..ManaPool::default()
        };
        assert_eq!(pool.max_x_for_cost(&cost), (u32::MAX - 5) / 2);
    }

    #[test]
    fn mixed_x_pips_can_be_payable_above_an_unpayable_value() {
        use ManaSymbol::{Blue, Generic, Red, X};
        let cost = crate::mana::ManaCost::from_pips(vec![
            vec![X, Generic(0)],
            vec![Generic(3), Blue],
            vec![X, Red],
        ]);
        let pool = ManaPool {
            blue: 1,
            red: 1,
            colorless: 2,
            ..ManaPool::default()
        };
        assert!(!pool.can_pay(&cost, 1));
        assert!(pool.can_pay(&cost, 2));
        assert_eq!(pool.max_x_for_cost(&cost), 2);
    }

    #[test]
    fn test_max_x_prefers_life_for_phyrexian() {
        use crate::mana::ManaCost;

        // Hex Parasite's cost: {{X}}, {{B/P}}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Black, ManaSymbol::Life(2)],
        ]);

        // With Black + Red: max_x should be 2 (using life for {{B/P}} to preserve mana for X)
        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Black, 1);
        pool.add(ManaSymbol::Red, 1);

        let max_x = pool.max_x_for_cost(&cost);
        assert_eq!(
            max_x, 2,
            "max_x should be 2 when we can use life for {{B/P}} and all mana for X"
        );
    }

    #[test]
    fn test_max_x_prefers_krrik_life_for_single_black_pip() {
        use crate::mana::ManaCost;

        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Black]]);

        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Black, 1);
        pool.add(ManaSymbol::Red, 1);

        let max_x = pool.max_x_for_cost_with_any_color_and_black_life(&cost, false, true);
        assert_eq!(
            max_x, 2,
            "Krrik-style payment should preserve both mana for X"
        );
    }

    #[test]
    fn test_phyrexian_falls_back_to_life_when_needed_for_x() {
        use crate::mana::ManaCost;

        // This tests the edge case where mana-first strategy fails
        // but life-first strategy succeeds.
        //
        // Cost: {{X}}{{B/P}} with X=2
        // Pool: 1B + 1R (2 total)
        //
        // Mana-first: Pay B for {{B/P}}, need 2 for X but only 1R left -> FAILS
        // Life-first: Pay life for {{B/P}}, pay 1B+1R for X=2 -> SUCCEEDS (2 life)

        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Black, ManaSymbol::Life(2)],
        ]);

        let mut pool = ManaPool::new();
        pool.add(ManaSymbol::Black, 1);
        pool.add(ManaSymbol::Red, 1);

        let (success, life_to_pay) = pool.try_pay_tracking_life(&cost, 2);
        assert!(
            success,
            "Should succeed by falling back to life-first strategy"
        );
        assert_eq!(
            life_to_pay, 2,
            "Should pay 2 life for {{B/P}} to have enough mana for X=2"
        );
        assert_eq!(pool.black, 0, "Black should be consumed for X");
        assert_eq!(pool.red, 0, "Red should be consumed for X");
    }
}

/// Retain the physical units consumed by one confirmed activation payment.
/// Restrictions and spending conversions do not change a unit's actual type.
pub(crate) fn activation_mana_budget_spent(before: &Player, after: &Player) -> ironsmith_core::mana::ManaPaymentBudget {
    let mut budget = ironsmith_core::mana::ManaPaymentBudget::default();
    let symbols = [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless];
    for (index, symbol) in symbols.iter().enumerate() {
        budget.units[index] = u32::try_from(
            before.mana_pool.amount_wide(*symbol).saturating_sub(after.mana_pool.amount_wide(*symbol)),
        )
        .unwrap_or(u32::MAX);
    }
    let mut remaining = after.restricted_mana.clone();
    for unit in &before.restricted_mana {
        if let Some(index) = remaining.iter().position(|other| other == unit) { remaining.remove(index); }
        else if let Some(index) = symbols.iter().position(|symbol| *symbol == unit.symbol) { budget.units[index] = budget.units[index].saturating_add(1); }
    }
    let mut consumed_provenance = [0u32; 6];
    let mut remaining = after.mana_source_provenance.clone();
    for unit in &before.mana_source_provenance {
        // A counted record may survive with fewer units; only the difference
        // was consumed.
        let consumed = if let Some(index) = remaining.iter().position(|other| other == unit) {
            remaining.remove(index);
            0
        } else if let Some(index) = remaining
            .iter()
            .position(|other| other.same_production(unit) && other.count < unit.count)
        {
            let kept = remaining.remove(index).count;
            unit.count - kept
        } else {
            unit.count
        };
        if consumed == 0 {
            continue;
        }
        let consumed = u32::try_from(consumed).unwrap_or(u32::MAX);
        if let Some(index) = symbols.iter().position(|symbol| *symbol == unit.symbol) {
            consumed_provenance[index] = consumed_provenance[index].saturating_add(consumed);
            if unit.snapshot.as_ref().is_some_and(|snapshot| snapshot.supertypes.contains(&crate::types::Supertype::Snow)) { budget.snow[index] = budget.snow[index].saturating_add(consumed); }
        }
    }
    for index in 0..6 { budget.units[index] = budget.units[index].max(consumed_provenance[index]); }
    budget
}

#[cfg(test)]
mod sol_78_activation_budget_tests {
    use super::*;
    #[test]
    fn exact_receipt_includes_restricted_mana_and_snow_provenance() {
        let a = PlayerId::from_index(0);
        let source = ObjectId::from_raw(110);
        let game = crate::game_state::GameState::new(vec!["A".into(), "B".into()], 20);
        let object = crate::object::Object::new_hidden_card(source, a, crate::zone::Zone::Hand);
        let mut snapshot = ObjectSnapshot::from_object(&object, &game);
        snapshot.supertypes.push(crate::types::Supertype::Snow);
        let mut before = Player::new(a, "A", 20);
        before.mana_pool.add(ManaSymbol::White, 2);
        before.restricted_mana.push(crate::ability::RestrictedManaUnit {
            symbol: ManaSymbol::Green, source, source_chosen_creature_type: None,
            restrictions: Vec::new(), source_controller: Some(a),
        });
        before.mana_source_provenance.push(ManaSourceProvenance {
            symbol: ManaSymbol::Green, source, snapshot: Some(snapshot), restricted: true, retention: None,
            count: 1,
        });
        let mut after = before.clone();
        after.mana_pool.remove(ManaSymbol::White, 1);
        after.restricted_mana.clear();
        after.mana_source_provenance.clear();
        let budget = activation_mana_budget_spent(&before, &after);
        assert_eq!(budget.units, [1, 0, 0, 0, 1, 0]);
        assert_eq!(budget.snow, [0, 0, 0, 0, 1, 0]);
    }
}
