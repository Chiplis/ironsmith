use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use rand::rng;
use std::collections::HashMap;

/// Mana pool tracking by color/type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManaPool {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
}

impl ManaPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds mana of the specified type.
    pub fn add(&mut self, symbol: ManaSymbol, amount: u32) {
        match symbol {
            ManaSymbol::White => self.white += amount,
            ManaSymbol::Blue => self.blue += amount,
            ManaSymbol::Black => self.black += amount,
            ManaSymbol::Red => self.red += amount,
            ManaSymbol::Green => self.green += amount,
            ManaSymbol::Colorless => self.colorless += amount,
            _ => {} // Generic, Snow, Life, X don't add to pool directly
        }
    }

    /// Removes mana of the specified type. Returns true if successful.
    pub fn remove(&mut self, symbol: ManaSymbol, amount: u32) -> bool {
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

    /// Returns the total amount of mana in the pool.
    pub fn total(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
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

    /// Returns the amount of mana of a specific type.
    pub fn amount(&self, symbol: ManaSymbol) -> u32 {
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
        // First try normal payment (uses mana when available)
        let mut pool = self.clone();
        if pool.try_pay_with_any_color(cost, x_value, allow_any_color) {
            return true;
        }

        // If that fails, try with life payment for Phyrexian pips
        let mut pool = self.clone();
        pool.try_pay_with_life_preference(cost, x_value, allow_any_color)
    }

    /// Try to pay preferring life for Phyrexian pips (to preserve mana for X).
    /// Used by can_pay to check if payment is possible with life option.
    fn try_pay_with_life_preference(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
    ) -> bool {
        use crate::mana::ManaSymbol;

        let mut pips: Vec<_> = cost.pips().iter().collect();
        pips.sort_by_key(|pip| {
            if pip
                .iter()
                .any(|s| matches!(s, ManaSymbol::Generic(_) | ManaSymbol::X))
            {
                1
            } else {
                0
            }
        });

        for pip in pips {
            let mut paid = false;

            for alternative in pip.iter() {
                match alternative {
                    ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green => {
                        if allow_any_color {
                            if self.total() > 0 {
                                self.pay_generic(1);
                                paid = true;
                                break;
                            }
                        } else if self.amount(*alternative) > 0 {
                            self.remove(*alternative, 1);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Colorless => {
                        if self.colorless > 0 {
                            self.colorless -= 1;
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Generic(n) => {
                        let needed = *n as u32;
                        if self.total() >= needed {
                            self.pay_generic(needed);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::X => {
                        if self.total() >= x_value {
                            self.pay_generic(x_value);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Snow => {
                        if self.total() > 0 {
                            self.pay_generic(1);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Life(_) => {
                        paid = true;
                        break;
                    }
                }
            }

            if !paid {
                return false;
            }
        }

        true
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
        // First, try mana-first strategy (prefer mana over life for Phyrexian pips)
        let original_pool = self.clone();
        let result = self.try_pay_internal(cost, x_value, false, allow_any_color);
        if result.0 {
            return result;
        }

        // Mana-first failed, restore pool and try life-first strategy
        *self = original_pool;
        self.try_pay_internal(cost, x_value, true, allow_any_color)
    }

    /// Try to pay a mana cost with X=x_value, optionally allowing any color spending.
    pub fn try_pay_with_any_color(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        allow_any_color: bool,
    ) -> bool {
        self.try_pay_tracking_life_with_any_color(cost, x_value, allow_any_color)
            .0
    }

    /// Internal payment implementation with configurable Phyrexian strategy.
    ///
    /// If `prefer_life_for_phyrexian` is true, Phyrexian pips are paid with life first.
    /// If false, tries mana first and only falls back to life if mana isn't available.
    fn try_pay_internal(
        &mut self,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        prefer_life_for_phyrexian: bool,
        allow_any_color: bool,
    ) -> (bool, u32) {
        // Sort pips so colored costs are paid first (more constrained),
        // then generic/X costs last (more flexible). This prevents the bug
        // where generic costs consume colored mana needed for later pips.
        let mut pips: Vec<_> = cost.pips().iter().collect();
        pips.sort_by_key(|pip| {
            // Colored pips have priority 0 (first), generic/X have priority 1 (last)
            if pip
                .iter()
                .any(|s| matches!(s, ManaSymbol::Generic(_) | ManaSymbol::X))
            {
                1
            } else {
                0
            }
        });

        let mut life_to_pay = 0u32;

        // Each pip in the cost must be paid
        for pip in pips {
            // For each pip, we have alternatives (e.g., [White, Blue] for hybrid)
            // Try each alternative until one works
            let mut paid = false;

            // Check if this is a Phyrexian pip (has Life alternative)
            let is_phyrexian = pip.iter().any(|s| matches!(s, ManaSymbol::Life(_)));

            // If preferring life for Phyrexian and this is a Phyrexian pip, pay with life first
            if prefer_life_for_phyrexian && is_phyrexian {
                for alternative in pip.iter() {
                    if let ManaSymbol::Life(amount) = alternative {
                        life_to_pay += *amount as u32;
                        paid = true;
                        break;
                    }
                }
                if paid {
                    continue;
                }
            }

            for alternative in pip.iter() {
                match alternative {
                    ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green => {
                        if allow_any_color {
                            if self.total() > 0 {
                                self.pay_generic(1);
                                paid = true;
                                break;
                            }
                        } else if self.amount(*alternative) > 0 {
                            self.remove(*alternative, 1);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Colorless => {
                        if self.colorless > 0 {
                            self.colorless -= 1;
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Generic(n) => {
                        // Generic can be paid with any mana
                        let needed = *n as u32;
                        if self.total() >= needed {
                            self.pay_generic(needed);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::X => {
                        // X is paid with generic mana equal to x_value
                        if self.total() >= x_value {
                            self.pay_generic(x_value);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Snow => {
                        // Snow mana - for simplicity, treat as generic
                        if self.total() > 0 {
                            self.pay_generic(1);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Life(amount) => {
                        // Life payment - track the amount to be deducted
                        life_to_pay += *amount as u32;
                        paid = true;
                        break;
                    }
                }
            }

            if !paid {
                return (false, 0);
            }
        }

        (true, life_to_pay)
    }

    /// Pay generic mana by draining from the pool.
    /// Prefers colorless, then other colors.
    fn pay_generic(&mut self, mut amount: u32) {
        // First use colorless
        let from_colorless = amount.min(self.colorless);
        self.colorless -= from_colorless;
        amount -= from_colorless;

        // Then use colored mana (arbitrary order)
        for pool in [
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
        // First check if the non-X part of the cost can be paid
        let mut test_pool = self.clone();

        // Count how many X pips there are
        let x_pip_count = cost
            .pips()
            .iter()
            .filter(|pip| pip.iter().any(|s| matches!(s, ManaSymbol::X)))
            .count() as u32;

        if x_pip_count == 0 {
            // No X in cost, X is 0
            return 0;
        }

        // Pay non-X costs first to see what's left
        // For max_x calculation, prefer life payment over mana to preserve mana for X
        for pip in cost.pips() {
            let is_x_pip = pip.iter().any(|s| matches!(s, ManaSymbol::X));
            if is_x_pip {
                continue; // Skip X pips for now
            }

            // First check if there's a life payment option - prefer it to save mana for X
            let has_life_option = pip.iter().any(|s| matches!(s, ManaSymbol::Life(_)));
            if has_life_option {
                continue; // Can pay with life, preserving mana for X
            }

            let mut paid = false;
            for alternative in pip {
                match alternative {
                    ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green => {
                        if allow_any_color {
                            if test_pool.total() > 0 {
                                test_pool.pay_generic(1);
                                paid = true;
                                break;
                            }
                        } else if test_pool.amount(*alternative) > 0 {
                            test_pool.remove(*alternative, 1);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Colorless => {
                        if test_pool.colorless > 0 {
                            test_pool.colorless -= 1;
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Generic(n) => {
                        let needed = *n as u32;
                        if test_pool.total() >= needed {
                            test_pool.pay_generic(needed);
                            paid = true;
                            break;
                        }
                    }
                    ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X => {
                        paid = true;
                        break;
                    }
                }
            }
            if !paid {
                return 0; // Can't even pay the base cost
            }
        }

        // Remaining mana can all go to X (divided by number of X pips, but usually 1)
        test_pool.total() / x_pip_count
    }
}

/// Complete player state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,

    // Life and resources
    pub life: i32,
    pub mana_pool: ManaPool,
    pub poison_counters: u32,
    pub energy_counters: u32,
    pub experience_counters: u32,

    // Per-turn tracking
    pub lands_played_this_turn: u32,
    pub land_plays_per_turn: u32,

    // Hand size
    pub max_hand_size: i32,

    // Game status
    pub has_lost: bool,
    pub has_won: bool,
    pub has_left_game: bool,

    // Zones (stored as object IDs)
    pub library: Vec<ObjectId>,
    pub hand: Vec<ObjectId>,
    pub graveyard: Vec<ObjectId>,

    // Commander tracking
    /// The card IDs of this player's commanders.
    /// Supports Partner mechanic (multiple commanders).
    /// Commanders can be legendary creatures, planeswalkers with "can be your commander",
    /// or certain artifacts (per recent rule changes).
    pub commanders: Vec<ObjectId>,

    // Commander damage tracking (source player -> damage)
    pub commander_damage: HashMap<PlayerId, u32>,
}

impl Player {
    pub fn new(id: PlayerId, name: impl Into<String>, starting_life: i32) -> Self {
        Self {
            id,
            name: name.into(),
            life: starting_life,
            mana_pool: ManaPool::new(),
            poison_counters: 0,
            energy_counters: 0,
            experience_counters: 0,
            lands_played_this_turn: 0,
            land_plays_per_turn: 1,
            max_hand_size: 7,
            has_lost: false,
            has_won: false,
            has_left_game: false,
            library: Vec::new(),
            hand: Vec::new(),
            graveyard: Vec::new(),
            commanders: Vec::new(),
            commander_damage: HashMap::new(),
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

    /// Returns true if this player can play a land this turn.
    pub fn can_play_land(&self) -> bool {
        self.lands_played_this_turn < self.land_plays_per_turn
    }

    /// Deals damage to this player. Returns the actual damage dealt.
    pub fn deal_damage(&mut self, amount: u32) -> u32 {
        self.life -= amount as i32;
        amount
    }

    /// Gains life.
    pub fn gain_life(&mut self, amount: u32) {
        self.life += amount as i32;
    }

    /// Loses life (different from damage - can't be prevented/redirected the same way).
    pub fn lose_life(&mut self, amount: u32) {
        self.life -= amount as i32;
    }

    /// Adds poison counters. Returns the new total.
    pub fn add_poison(&mut self, amount: u32) -> u32 {
        self.poison_counters += amount;
        self.poison_counters
    }

    /// Draws cards from library to hand. Returns the IDs of cards drawn.
    pub fn draw(&mut self, count: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(card_id) = self.library.pop() {
                self.hand.push(card_id);
                drawn.push(card_id);
            } else {
                // Can't draw from empty library - will trigger loss via state-based actions
                break;
            }
        }
        drawn
    }

    /// Called at the beginning of this player's turn.
    pub fn begin_turn(&mut self) {
        self.lands_played_this_turn = 0;
    }

    /// Records a land play.
    pub fn record_land_play(&mut self) {
        self.lands_played_this_turn += 1;
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
    /// This uses a simple randomization. In a real implementation, you might
    /// want to use a seeded RNG for reproducibility in tests.
    pub fn shuffle_library(&mut self) {
        use rand::seq::SliceRandom;
        self.library.shuffle(&mut rng());
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
    }

    #[test]
    fn test_commander_damage() {
        let mut player = Player::new(PlayerId::from_index(0), "Grace", 40);

        player.commander_damage.insert(PlayerId::from_index(1), 10);
        player.commander_damage.insert(PlayerId::from_index(2), 5);

        assert_eq!(
            player.commander_damage.get(&PlayerId::from_index(1)),
            Some(&10)
        );
        assert_eq!(
            player.commander_damage.get(&PlayerId::from_index(2)),
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
