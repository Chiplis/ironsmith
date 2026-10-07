//! Resumable exact mana search. A suspended query is never cached as unpayable.
use super::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Work chunk for synchronous finite-source queries, not a legality cutoff.
const SYNC_SEARCH_NODE_CHUNK: usize = 65_536;

#[derive(Debug, Clone)]
struct Node {
    pip: usize,
    pool: crate::player::ManaPool,
    snow: crate::player::ManaPool,
    used: Vec<bool>,
    life: u32,
}

/// Sources that are interchangeable for this query: same outputs, same snow
/// provenance, same spending policy. Only one member of a class needs to branch
/// at any node, so the classes are computed once per query instead of rescanned
/// for every source at every node expansion.
#[derive(Debug, Clone)]
struct SourceClasses {
    /// Member source indices, ascending, one entry per class.
    classes: Vec<Vec<usize>>,
}

impl SourceClasses {
    fn build(
        sources: &[AvailableManaSource],
        source_policies: &[crate::player::ManaSpendPolicy],
    ) -> Self {
        let mut classes: Vec<Vec<usize>> = Vec::new();
        for index in 0..sources.len() {
            let existing = classes.iter_mut().find(|members| {
                let first = members[0];
                sources[first].outputs == sources[index].outputs
                    && sources[first].from_snow_source == sources[index].from_snow_source
                    && source_policies[first] == source_policies[index]
            });
            match existing {
                Some(members) => members.push(index),
                None => classes.push(vec![index]),
            }
        }
        Self { classes }
    }

    /// The canonical branch candidates for a node: the lowest-indexed unused
    /// member of each class, in descending index order so the depth-first
    /// preference matches the original per-source scan.
    fn representatives(&self, used: &[bool], out: &mut Vec<usize>) {
        out.clear();
        for members in &self.classes {
            if let Some(index) = members.iter().copied().find(|index| !used[*index]) {
                out.push(index);
            }
        }
        out.sort_unstable_by(|a, b| b.cmp(a));
    }
}

/// Every input the search reads. Held owned so a lookup compares exactly, with
/// no formatted string key and no chance of a hash collision deciding
/// payability.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManaQuery {
    pips: Vec<Vec<ManaSymbol>>,
    pool: crate::player::ManaPool,
    snow: crate::player::ManaPool,
    sources: Vec<AvailableManaSource>,
    max_life: u32,
    policy: crate::player::ManaSpendPolicy,
    source_policies: Vec<crate::player::ManaSpendPolicy>,
}

/// A borrowed view of the same inputs, so the hot lookup path hashes and
/// compares without cloning anything.
#[derive(Clone, Copy)]
struct ManaQueryRef<'a> {
    pips: &'a [Vec<ManaSymbol>],
    pool: &'a crate::player::ManaPool,
    snow: &'a crate::player::ManaPool,
    sources: &'a [AvailableManaSource],
    max_life: u32,
    policy: &'a crate::player::ManaSpendPolicy,
    source_policies: &'a [crate::player::ManaSpendPolicy],
}

fn hash_pool<H: Hasher>(pool: &crate::player::ManaPool, state: &mut H) {
    pool.white.hash(state);
    pool.blue.hash(state);
    pool.black.hash(state);
    pool.red.hash(state);
    pool.green.hash(state);
    pool.colorless.hash(state);
}

fn hash_policy<H: Hasher>(policy: &crate::player::ManaSpendPolicy, state: &mut H) {
    policy.mode.hash(state);
    policy.any_color_mana_symbols.hash(state);
    policy.other_mana_only_as_colorless.hash(state);
}

fn hash_source<H: Hasher>(source: &AvailableManaSource, state: &mut H) {
    source.source_id.hash(state);
    source.outputs.hash(state);
    source.from_snow_source.hash(state);
}

impl ManaQueryRef<'_> {
    fn digest(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.pips.hash(&mut hasher);
        hash_pool(self.pool, &mut hasher);
        hash_pool(self.snow, &mut hasher);
        self.sources.len().hash(&mut hasher);
        for source in self.sources {
            hash_source(source, &mut hasher);
        }
        self.max_life.hash(&mut hasher);
        hash_policy(self.policy, &mut hasher);
        for policy in self.source_policies {
            hash_policy(policy, &mut hasher);
        }
        hasher.finish()
    }

    fn matches(&self, owned: &ManaQuery) -> bool {
        self.max_life == owned.max_life
            && self.pips == owned.pips.as_slice()
            && self.pool == &owned.pool
            && self.snow == &owned.snow
            && self.policy == &owned.policy
            && self.sources == owned.sources.as_slice()
            && self.source_policies == owned.source_policies.as_slice()
    }

    fn to_owned_query(self) -> ManaQuery {
        ManaQuery {
            pips: self.pips.to_vec(),
            pool: self.pool.clone(),
            snow: self.snow.clone(),
            sources: self.sources.to_vec(),
            max_life: self.max_life,
            policy: self.policy.clone(),
            source_policies: self.source_policies.to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
struct Search {
    frontier: Vec<Node>,
    seen: HashSet<(ManaPaymentSearchKey, Vec<bool>)>,
    result: Option<bool>,
    classes: SourceClasses,
}

/// A fact about the analysis snapshot that does not depend on the mana search.
/// The session is installed only while a sliced analysis runs against an owned,
/// immutable `GameState`, so an entry stays valid for the life of the job and
/// is dropped with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SnapshotFactKey {
    pub(crate) kind: SnapshotFactKind,
    pub(crate) object: ObjectId,
    pub(crate) player: PlayerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SnapshotFactKind {
    CastTargetLegality,
}

/// The rest of the memo key, compared exactly rather than hashed, so two
/// casting methods for the same card can never share an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotFactContext {
    pub(crate) casting_method: crate::alternative_cast::CastingMethod,
    pub(crate) mana_cost: Option<crate::mana::ManaCost>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum PaymentQueryContext {
    #[default]
    Root,
    ContinuousCheckedRoot,
    ProposedSpellRoot(ObjectId),
    ProposedSpellCheckedRoot(ObjectId),
    DeclaredCastRoot(ObjectId, crate::alternative_cast::CastingMethod, crate::cost::OptionalCostsPaid),
    DeclaredCastCheckedRoot(ObjectId, crate::alternative_cast::CastingMethod, crate::cost::OptionalCostsPaid),
}

/// Owned by a single immutable priority snapshot. Completed queries are reused
/// across menu passes; unfinished queries retain their frontier without restart.
#[derive(Debug, Default)]
pub struct ManaAnalysisSession {
    searches: HashMap<u64, Vec<(ManaQuery, Search)>>,
    facts: HashMap<SnapshotFactKey, Vec<(SnapshotFactContext, bool)>>,
    payment_searches: Vec<(PaymentQueryContext, crate::mana_payment::ManaPaymentRequest, crate::mana_payment::ManaPaymentAnalysis)>,
    /// Identity checks only: these addresses are never dereferenced. The owner
    /// must retain its immutable snapshot for the session's lifetime.
    bound_root: Option<usize>,
    active_root: Option<usize>,
    active_context: PaymentQueryContext,
    remaining: usize,
    pending: bool,
    /// An incomplete calculation is not evidence that a payment is illegal.
    failure: Option<crate::effects::ExecutionError>,
    /// Incremented whenever a query suspends, so a caller can tell whether the
    /// work it just ran was complete or provisional.
    suspensions: u64,
    /// Node pops consumed by the most recent slice.
    last_slice_nodes: usize,
}

thread_local! {
    static ASSUME_MANA_FOR_PRESENTATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn mana_payment_is_assumed() -> bool {
    ASSUME_MANA_FOR_PRESENTATION.with(|value| value.get())
}

/// Recompute current timing, targets and non-mana costs without an affordability
/// search. These candidates may start an announcement; payment remains an
/// independently validated step before the action can complete.
pub(crate) fn with_assumed_mana_for_presentation<T>(compute: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { ASSUME_MANA_FOR_PRESENTATION.with(|value| value.set(self.0)); }
    }
    let _restore = Restore(ASSUME_MANA_FOR_PRESENTATION.with(|value| value.replace(true)));
    compute()
}

thread_local! {
    static SESSION: RefCell<Option<ManaAnalysisSession>> = const { RefCell::new(None) };
}

impl ManaAnalysisSession {
    pub fn run<T>(&mut self, budget: usize, compute: impl FnOnce() -> T) -> (T, bool) {
        self.run_with_root(None, budget, compute)
    }

    /// Bind full payment queries to this exact immutable game. Hypothetical
    /// clones cannot reuse its frontier, even if their payment requests match.
    pub fn run_for_game<T>(&mut self, game: &GameState, budget: usize, compute: impl FnOnce() -> T) -> (T, bool) {
        self.run_with_root(Some(game as *const GameState as usize), budget, compute)
    }

    fn run_with_root<T>(&mut self, root: Option<usize>, budget: usize, compute: impl FnOnce() -> T) -> (T, bool) {
        if root.is_some() && self.bound_root != root {
            *self = Self::default();
            self.bound_root = root;
        }
        self.active_root = root;
        self.active_context = PaymentQueryContext::Root;
        struct Restore<'a>(&'a mut ManaAnalysisSession);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                *self.0 = SESSION.with(|slot| slot.borrow_mut().take().unwrap());
            }
        }
        let budget = budget.max(1);
        self.remaining = budget;
        self.pending = false;
        SESSION.with(|slot| {
            assert!(slot.borrow().is_none(), "nested mana analysis session");
            *slot.borrow_mut() = Some(std::mem::take(self));
        });
        let restore = Restore(self);
        let result = compute();
        let complete = SESSION.with(|slot| {
            let mut slot = slot.borrow_mut();
            let session = slot.as_mut().unwrap();
            session.last_slice_nodes = budget.saturating_sub(session.remaining);
            !session.pending && session.failure.is_none()
        });
        drop(restore);
        self.active_root = None;
        (result, complete)
    }

    /// A terminal calculation failure for this immutable analysis job. Callers
    /// must surface it instead of publishing a complete or negative menu.
    pub fn failure(&self) -> Option<&crate::effects::ExecutionError> { self.failure.as_ref() }

    /// Node pops the last slice actually consumed. A slice that returns fewer
    /// nodes than its budget was limited by the fixed cost of re-enumerating
    /// the menu, not by the search, which is what the scheduler needs to know
    /// to size the next slice.
    pub fn last_slice_nodes(&self) -> usize {
        self.last_slice_nodes
    }
}

/// The legal-action boundary has just successfully computed this exact
/// continuous-query snapshot from the immutable root. Give that deterministic
/// view its own cache namespace; no arbitrary hypothetical clone is admitted.
pub(crate) fn with_checked_query<T>(root: &GameState, checked: &GameState, compute: impl FnOnce() -> T) -> T {
    let previous = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let session = slot.as_mut()?;
        if session.active_root != Some(root as *const GameState as usize)
            || session.active_context != PaymentQueryContext::Root { return None; }
        let previous = (session.active_root, session.active_context.clone());
        session.active_root = Some(checked as *const GameState as usize);
        session.active_context = PaymentQueryContext::ContinuousCheckedRoot;
        Some(previous)
    });
    struct Restore(Option<(Option<usize>, PaymentQueryContext)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some((root, context)) = self.0.take() {
                SESSION.with(|slot| {
                    if let Some(session) = slot.borrow_mut().as_mut() {
                        session.active_root = root;
                        session.active_context = context;
                    }
                });
            }
        }
    }
    let _restore = Restore(previous);
    compute()
}

/// The casting boundary changes only this proposed spell's zone to Stack.
/// Bind that deterministic view separately from root and continuous-check
/// queries. Other hypothetical games remain outside the resumable session.
pub(super) fn with_proposed_spell<T>(
    root: &GameState, proposed: &GameState, spell: ObjectId, compute: impl FnOnce() -> T,
) -> T {
    let previous = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let session = slot.as_mut()?;
        if session.active_root != Some(root as *const GameState as usize) { return None; }
        let context = match session.active_context.clone() {
            PaymentQueryContext::Root => PaymentQueryContext::ProposedSpellRoot(spell),
            PaymentQueryContext::ContinuousCheckedRoot => PaymentQueryContext::ProposedSpellCheckedRoot(spell),
            _ => return None,
        };
        let previous = (session.active_root, session.active_context.clone());
        session.active_root = Some(proposed as *const GameState as usize);
        session.active_context = context;
        Some(previous)
    });
    struct Restore(Option<(Option<usize>, PaymentQueryContext)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some((root, context)) = self.0.take() {
                SESSION.with(|slot| {
                    if let Some(session) = slot.borrow_mut().as_mut() {
                        session.active_root = root;
                        session.active_context = context;
                    }
                });
            }
        }
    }
    let _restore = Restore(previous);
    compute()
}

/// Capture the first failed calculation, including while a nested planner has
/// temporarily removed the thread-local session to avoid recursive caching.
fn payment_result<T>(
    game: &GameState,
    result: Result<T, crate::mana_payment::ManaPaymentFailure>,
    session: Option<&mut ManaAnalysisSession>,
) -> bool {
    match result {
        Ok(_) => true,
        Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => {
            game.record_token_resource_failure(&error);
            if let Some(session) = session {
                if session.failure.is_none() { session.failure = Some(error); }
                session.pending = true;
                session.suspensions = session.suspensions.saturating_add(1);
            }
            false
        }
        Err(_) => false,
    }
}


pub(super) fn failed_calculation(game: &GameState, error: crate::effects::ExecutionError) -> bool {
    SESSION.with(|slot| payment_result::<()>(game,
        Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)),
        slot.borrow_mut().as_mut()))
}

/// A selected cast face and price are part of the payment query identity.
/// Only a deterministic declaration from the bound immutable root is admitted.
pub(super) fn with_declared_cast<T>(
    root: &GameState, proposed: &GameState, spell: ObjectId,
    method: &crate::alternative_cast::CastingMethod, optional_costs: &crate::cost::OptionalCostsPaid, compute: impl FnOnce() -> T,
) -> T {
    let previous = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let session = slot.as_mut()?;
        if session.active_root != Some(root as *const GameState as usize) { return None; }
        let context = match session.active_context.clone() {
            PaymentQueryContext::Root => PaymentQueryContext::DeclaredCastRoot(spell, method.clone(), optional_costs.clone()),
            PaymentQueryContext::ContinuousCheckedRoot => PaymentQueryContext::DeclaredCastCheckedRoot(spell, method.clone(), optional_costs.clone()),
            _ => return None,
        };
        let previous = (session.active_root, session.active_context.clone());
        session.active_root = Some(proposed as *const GameState as usize);
        session.active_context = context;
        Some(previous)
    });
    struct Restore(Option<(Option<usize>, PaymentQueryContext)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some((root, context)) = self.0.take() {
                SESSION.with(|slot| {
                    if let Some(session) = slot.borrow_mut().as_mut() {
                        session.active_root = root;
                        session.active_context = context;
                    }
                });
            }
        }
    }
    let _restore = Restore(previous);
    compute()
}

pub(crate) fn analysis_failure() -> Option<crate::effects::ExecutionError> {
    SESSION.with(|slot| slot.borrow().as_ref().and_then(|session| session.failure.clone()))
}

/// Use the authoritative planner's resumable search for a query against the
/// bound root. Pending remains provisional and prevents publishing a complete
/// menu. Other games retain the synchronous oracle, never a cached root answer.
pub(super) fn check_payment(game: &GameState, request: &crate::mana_payment::ManaPaymentRequest) -> bool {
    if mana_payment_is_assumed() { return true; }
    let bound = SESSION.with(|slot| slot.borrow().as_ref().is_some_and(|session|
        session.active_root == Some(game as *const GameState as usize)));
    // Nested affordability checks inside one planner work unit must remain
    // exact; they must not turn a suspended inner solver into a negative veto.
    struct Restore(Option<ManaAnalysisSession>);
    impl Drop for Restore {
        fn drop(&mut self) { SESSION.with(|slot| *slot.borrow_mut() = self.0.take()); }
    }
    let mut restore = Restore(SESSION.with(|slot| slot.borrow_mut().take()));
    if !bound {
        return payment_result(game, crate::mana_payment::check_mana_payment(game, request), restore.0.as_mut());
    }
    let session = restore.0.as_mut().unwrap();
    let context = session.active_context.clone();
    let index = session.payment_searches.iter().position(|(kind, key, _)| *kind == context && key == request)
        .unwrap_or_else(|| {
            session.payment_searches.push((context.clone(), request.clone(), crate::mana_payment::ManaPaymentAnalysis::check(game, request.clone())));
            session.payment_searches.len() - 1
        });
    if session.remaining == 0 {
        session.pending = true;
        session.suspensions = session.suspensions.saturating_add(1);
        return false;
    }
    let search = &mut session.payment_searches[index].2;
    let result = search.step(session.remaining);
    session.remaining = session.remaining.saturating_sub(search.last_slice_units());
    match result {
        Some(result) => payment_result(game, result, Some(session)),
        None => {
            session.pending = true;
            session.suspensions = session.suspensions.saturating_add(1);
            false
        }
    }
}

/// Memoizes a fact that depends only on the analysis snapshot, not on the mana
/// search. Without a session installed (every synchronous caller) this is a
/// plain call through.
///
/// A value computed while the enclosing query suspended is provisional, so it
/// is recomputed on a later slice rather than cached.
pub(crate) fn memo_snapshot_fact(
    key: SnapshotFactKey,
    context: &SnapshotFactContext,
    compute: impl FnOnce() -> bool,
) -> bool {
    let cached = SESSION.with(|slot| {
        slot.borrow().as_ref().and_then(|session| {
            session.facts.get(&key).and_then(|entries| {
                entries
                    .iter()
                    .find(|(candidate, _)| candidate == context)
                    .map(|(_, value)| *value)
            })
        })
    });
    if let Some(cached) = cached {
        return cached;
    }
    let suspensions_before =
        SESSION.with(|slot| slot.borrow().as_ref().map(|session| session.suspensions));
    let value = compute();
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return;
        };
        if session.failure.is_none() && suspensions_before == Some(session.suspensions) {
            session
                .facts
                .entry(key)
                .or_default()
                .push((context.clone(), value));
        }
    });
    value
}

pub(super) fn solve(
    pips: &[Vec<ManaSymbol>],
    pool: crate::player::ManaPool,
    snow: crate::player::ManaPool,
    sources: &[AvailableManaSource],
    max_life: u32,
    policy: &crate::player::ManaSpendPolicy,
    source_policies: &[crate::player::ManaSpendPolicy],
) -> bool {
    // A concrete pool-only payment is a sound success shortcut. Failure of
    // this greedy attempt is not a proof: hybrid choices may need backtracking.
    let mut direct_pool = pool.clone();
    let mut direct_snow = snow.clone();
    let mut direct_life = 0u32;
    let direct = pips.iter().all(|pip| {
        pip.iter().any(|symbol| {
            if let ManaSymbol::Life(amount) = *symbol {
                if direct_life.saturating_add(amount as u32) <= max_life {
                    direct_life += amount as u32;
                    return true;
                }
                return false;
            }
            remove_mana_for_pip(&mut direct_pool, &mut direct_snow, *symbol, policy)
        })
    });
    if direct {
        return true;
    }
    if !pips
        .iter()
        .flatten()
        .any(|symbol| matches!(symbol, ManaSymbol::Life(_)))
    {
        let capacity = sources.iter().fold(pool.total() as usize, |sum, source| {
            sum.saturating_add(source.outputs.iter().map(Vec::len).max().unwrap_or(0))
        });
        if capacity < pips.len() {
            return false;
        }
    }
    // Includes every input read by the search, including contextual source
    // spending permissions. Structural hash plus exact comparison; no formatted
    // key and no game-pointer identity.
    let query = ManaQueryRef {
        pips,
        pool: &pool,
        snow: &snow,
        sources,
        max_life,
        policy,
        source_policies,
    };
    let digest = query.digest();
    let initial = || Search {
        frontier: vec![Node {
            pip: 0,
            pool: pool.clone(),
            snow: snow.clone(),
            used: vec![false; sources.len()],
            life: 0,
        }],
        seen: HashSet::new(),
        result: None,
        classes: SourceClasses::build(sources, source_policies),
    };
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(session) = slot.as_mut() {
            let bucket = session.searches.entry(digest).or_default();
            let position = bucket
                .iter()
                .position(|(candidate, _)| query.matches(candidate));
            let index = match position {
                Some(index) => index,
                None => {
                    bucket.push((query.to_owned_query(), initial()));
                    bucket.len() - 1
                }
            };
            let search = &mut bucket[index].1;
            advance(
                search,
                pips,
                sources,
                max_life,
                policy,
                source_policies,
                &mut session.remaining,
            );
            match search.result {
                Some(result) => result,
                None => {
                    session.pending = true;
                    session.suspensions = session.suspensions.saturating_add(1);
                    false
                }
            }
        } else {
            let mut search = initial();
            // Every source is used at most once in this solver, so its state
            // space is finite. Exhaustion of a work chunk is not unpayability.
            while search.result.is_none() {
                let mut remaining = SYNC_SEARCH_NODE_CHUNK;
                advance(&mut search, pips, sources, max_life, policy,
                    source_policies, &mut remaining);
            }
            search.result.unwrap()
        }
    })
}

fn advance(
    search: &mut Search,
    pips: &[Vec<ManaSymbol>],
    sources: &[AvailableManaSource],
    max_life: u32,
    policy: &crate::player::ManaSpendPolicy,
    source_policies: &[crate::player::ManaSpendPolicy],
    remaining: &mut usize,
) {
    if search.result.is_some() {
        return;
    }
    let mut representatives: Vec<usize> = Vec::with_capacity(search.classes.classes.len());
    while *remaining > 0 {
        let Some(node) = search.frontier.pop() else {
            search.result = Some(false);
            return;
        };
        *remaining -= 1;
        if node.pip == pips.len() {
            search.result = Some(true);
            search.frontier.clear();
            search.seen.clear();
            return;
        }
        let key = (
            ManaPaymentSearchKey::new(node.pip, &node.pool, &node.snow, node.life, 0),
            node.used.clone(),
        );
        if !search.seen.insert(key) {
            continue;
        }
        // Interchangeable sources need only one branch. Their object identity
        // remains in the query key; equivalence is local to this payment's
        // resolved policy and output choices, and the partition is computed
        // once per query rather than rescanned per source.
        search
            .classes
            .representatives(&node.used, &mut representatives);
        // Reverse insertion preserves the old depth-first preference: life,
        // floating pool, then the first available source/output.
        for &symbol in pips[node.pip].iter().rev() {
            if let ManaSymbol::Life(amount) = symbol {
                let life = node.life.saturating_add(amount as u32);
                if life <= max_life {
                    search.frontier.push(Node {
                        pip: node.pip + 1,
                        life,
                        ..node.clone()
                    });
                }
                continue;
            }
            for &index in representatives.iter() {
                let source = &sources[index];
                for output in source.outputs.iter().rev() {
                    if let Some((extra, extra_snow)) = consume_output_for_pip(
                        output,
                        symbol,
                        &source_policies[index],
                        source.from_snow_source,
                    ) {
                        let mut next = node.clone();
                        next.pip += 1;
                        next.used[index] = true;
                        add_pool(&mut next.pool, &extra);
                        add_pool(&mut next.snow, &extra_snow);
                        search.frontier.push(next);
                    }
                }
            }
            let mut next = node.clone();
            if remove_mana_for_pip(&mut next.pool, &mut next.snow, symbol, policy) {
                next.pip += 1;
                search.frontier.push(next);
            }
        }
    }
    if search.frontier.is_empty() {
        search.result = Some(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource_fixture() -> (GameState, PlayerId, ObjectId, crate::mana_payment::ManaPaymentRequest) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        game.turn.active_player = player; game.turn.priority_player = Some(player);
        game.turn.phase = crate::game_state::Phase::FirstMain; game.turn.step = None;
        let card = crate::CardBuilder::new(crate::CardId::new(), "Query source")
            .card_types(vec![crate::CardType::Land]).build();
        let source = game.create_object_from_card(&card, player, crate::Zone::Battlefield);
        game.object_mut(source).unwrap().abilities_mut().push(crate::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()), vec![ManaSymbol::Green]));
        game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
            source, player, crate::events::mana::matchers::ManaProducedBySourceMatcher::new(crate::target::ObjectFilter::specific(source)),
            crate::replacement::ReplacementAction::Additionally(vec![crate::effect::Effect::new(crate::effects::CreateTokenEffect::you(crate::cards::tokens::treasure_token_definition(), 2))]),
        ));
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
        let request = crate::mana_payment::ManaPaymentRequest::new(player, source, crate::costs::PaymentReason::Effect,
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Green]));
        (game, player, source, request)
    }

    #[test]
    fn resource_failed_payment_is_neither_complete_nor_a_cached_negative_fact() {
        let (game, player, source, request) = resource_fixture();
        for bind_root in [false, true] {
            let mut session = ManaAnalysisSession::default();
            let key = SnapshotFactKey { kind: SnapshotFactKind::CastTargetLegality, object: source, player };
            let context = SnapshotFactContext { casting_method: crate::alternative_cast::CastingMethod::Normal, mana_cost: Some(request.cost.clone()) };
            let mut failure_seen = false;
            for _ in 0..256 {
                let compute = || memo_snapshot_fact(key, &context, || check_payment(&game, &request));
                let (payable, complete) = if bind_root { session.run_for_game(&game, 1, compute) }
                    else { session.run(1, compute) };
                if session.failure().is_some() {
                    assert!(!payable); assert!(!complete);
                    assert!(matches!(session.failure(), Some(crate::effects::ExecutionError::ResourceLimitExceeded { .. })));
                    assert!(!session.facts.contains_key(&key));
                    failure_seen = true; break;
                }
                assert!(!complete, "resource-limited payment cannot finish as unavailable");
            }
            assert!(failure_seen);
        }
        assert!(!game.is_tapped(source)); assert_eq!(game.battlefield.len(), 1);
    }

    #[test]
    fn legal_action_root_surfaces_resource_unknown_and_new_job_can_recover() {
        let (mut game, player, source, _) = resource_fixture();
        let spell = crate::CardBuilder::new(crate::CardId::new(), "Green spell")
            .card_types(vec![crate::CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Green])).build();
        let card = game.create_object_from_card(&spell, player, crate::Zone::Hand);
        assert!(matches!(crate::decision::compute_legal_actions(&game, player),
            Err(crate::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert!(!game.is_tapped(source)); assert_eq!(game.battlefield.len(), 1);
        assert!(game.player(player).unwrap().hand.contains(&card));
        game.set_token_creation_limits(Default::default());
        let actions = crate::decision::compute_legal_actions(&game, player).unwrap();
        assert!(actions.iter().any(|action| matches!(action, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == card)));
        assert!(!game.is_tapped(source)); assert_eq!(game.battlefield.len(), 1);
    }

    #[test]
    fn full_payment_queries_resume_without_publishing_pending_as_unpayable() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Life-cost mana source")
            .card_types(vec![crate::types::CardType::Land])
            .with_ability(crate::Ability::mana_with_effects(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
                vec![crate::effect::Effect::add_mana_of_any_color_restricted(1,
                    vec![crate::color::Color::White, crate::color::Color::Blue])])).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        for amount in [1, 2, 3] {
            let request = crate::mana_payment::ManaPaymentRequest::new(alice, source,
                crate::costs::PaymentReason::Effect,
                crate::mana::ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; amount]));
            let expected = crate::mana_payment::check_mana_payment(&game, &request).is_ok();
            let mut session = ManaAnalysisSession::default();
            let mut completed = false;
            let mut suspended = false;
            for _ in 0..1000 {
                let (actual, complete) = session.run_for_game(&game, 1, || check_payment(&game, &request));
                assert!(session.last_slice_nodes() <= 1);
                if complete {
                    assert_eq!(actual, expected, "cost {amount}");
                    completed = true;
                    break;
                }
                suspended = true;
            }
            assert!(completed, "full search must finish its frontier for cost {amount}");
            if amount <= 2 {
                assert!(expected && suspended, "payable fallback must retain and resume its frontier for cost {amount}");
            } else {
                // Three pips exceed these two fixed, single-use sources. A
                // sound finite-production proof may finish without suspension.
                assert!(!expected);
            }
            let (again, complete) = session.run_for_game(&game, 1, || check_payment(&game, &request));
            assert!(complete);
            assert_eq!(again, expected);
            assert_eq!(session.last_slice_nodes(), 0, "completed exact query is reusable");
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(!game.is_tapped(source));
        }
    }

    #[test]
    fn full_payment_root_cache_does_not_alias_hypothetical_game_or_request() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let request = crate::mana_payment::ManaPaymentRequest::new(alice, ObjectId::from_raw(999),
            crate::costs::PaymentReason::Effect,
            crate::mana::ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]));
        let mut hypothetical = game.clone();
        hypothetical.player_mut(alice).unwrap().mana_pool.blue = 1;
        let mut session = ManaAnalysisSession::default();
        let (actual, complete) = session.run_for_game(&game, 100, || {
            let root = check_payment(&game, &request);
            let alternate = check_payment(&hypothetical, &request);
            let mut free = request.clone();
            free.cost = crate::mana::ManaCost::new();
            let changed_request = check_payment(&game, &free);
            (root, alternate, changed_request)
        });
        assert!(complete);
        assert_eq!(actual, (false, true, true));
        let (actual, complete) = session.run_for_game(&hypothetical, 100, || check_payment(&hypothetical, &request));
        assert!(complete && actual, "binding a different root must discard old root facts");
    }

    #[test]
    fn resumable_search_matches_recursive_oracle() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        let outputs = [
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::White, ManaSymbol::Blue],
        ];
        let costs = [
            vec![vec![ManaSymbol::White]],
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]],
            vec![
                vec![ManaSymbol::White, ManaSymbol::Blue],
                vec![ManaSymbol::Generic(1)],
            ],
            vec![vec![ManaSymbol::Snow], vec![ManaSymbol::Colorless]],
            vec![
                vec![ManaSymbol::Life(2), ManaSymbol::Blue],
                vec![ManaSymbol::White],
            ],
            vec![vec![ManaSymbol::Generic(1)]; 4],
        ];
        for mask in 0..81usize {
            let mut code = mask;
            let sources = (0..4)
                .map(|index| {
                    let choice = code % 3;
                    code /= 3;
                    AvailableManaSource {
                        source_id: ObjectId::from_raw(index + 1),
                        outputs: vec![outputs[choice].clone()],
                        from_snow_source: index % 2 == 0,
                    }
                })
                .collect::<Vec<_>>();
            for policy in [
                crate::player::ManaSpendPolicy::default(),
                crate::player::ManaSpendPolicy::from_any_color(true),
            ] {
                for pips in &costs {
                    let pool = crate::player::ManaPool {
                        colorless: 1,
                        ..Default::default()
                    };
                    let snow = crate::player::ManaPool::default();
                    let expected = can_pay_expanded_pips(
                        &game,
                        player,
                        pips,
                        0,
                        pool.clone(),
                        snow.clone(),
                        &sources,
                        0,
                        0,
                        4,
                        &policy,
                        None,
                        &mut HashSet::new(),
                    );
                    let mut session = ManaAnalysisSession::default();
                    let mut finished = false;
                    for _ in 0..1000 {
                        let (actual, complete) = session.run(1, || {
                            solve(
                                pips,
                                pool.clone(),
                                snow.clone(),
                                &sources,
                                4,
                                &policy,
                                &vec![policy.clone(); sources.len()],
                            )
                        });
                        if complete {
                            assert_eq!(actual, expected, "mask={mask}, pips={pips:?}");
                            finished = true;
                            break;
                        }
                    }
                    assert!(finished, "search failed to resume");
                }
            }
        }
    }

    #[test]
    fn pending_is_not_cached_as_unpayable_and_complete_queries_are_reused() {
        let sources = vec![AvailableManaSource {
            source_id: ObjectId::from_raw(1),
            outputs: vec![vec![ManaSymbol::Blue]],
            from_snow_source: false,
        }];
        let policy = crate::player::ManaSpendPolicy::default();
        let mut session = ManaAnalysisSession::default();
        let query = || {
            solve(
                &[vec![ManaSymbol::Blue]],
                Default::default(),
                Default::default(),
                &sources,
                0,
                &policy,
                &[policy.clone()],
            )
        };
        assert_eq!(session.run(1, query), (false, false));
        assert_eq!(session.run(1, query), (true, true));
        assert_eq!(session.run(1, query), (true, true));
    }

    #[test]
    fn more_than_128_sources_keep_exact_snow_and_color_semantics() {
        let sources = (0..130)
            .map(|i| AvailableManaSource {
                source_id: ObjectId::from_raw(i + 1),
                outputs: vec![vec![ManaSymbol::Blue]],
                from_snow_source: i == 129,
            })
            .collect::<Vec<_>>();
        let policy = crate::player::ManaSpendPolicy::default();
        assert!(solve(
            &[vec![ManaSymbol::Snow], vec![ManaSymbol::Blue]],
            Default::default(),
            Default::default(),
            &sources,
            0,
            &policy,
            &vec![policy.clone(); sources.len()]
        ));
        assert!(!solve(
            &[vec![ManaSymbol::Snow], vec![ManaSymbol::Snow]],
            Default::default(),
            Default::default(),
            &sources,
            0,
            &policy,
            &vec![policy.clone(); sources.len()]
        ));
    }
}
