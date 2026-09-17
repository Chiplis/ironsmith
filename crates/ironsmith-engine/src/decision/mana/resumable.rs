//! Resumable exact mana search. A suspended query is never cached as unpayable.
use super::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// A synchronous caller has no slice to yield to, so the search still needs an
/// upper bound. Realistic costs settle well under this; reaching it means the
/// query is pathological and the action is withheld rather than hanging the
/// engine. Mirrors the general planner's `MAX_SEARCH_NODES` in spirit.
const SYNC_SEARCH_NODE_LIMIT: usize = 500_000;

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

/// Owned by a single immutable priority snapshot. Completed queries are reused
/// across menu passes; unfinished queries retain their frontier without restart.
#[derive(Debug, Default)]
pub struct ManaAnalysisSession {
    searches: HashMap<u64, Vec<(ManaQuery, Search)>>,
    facts: HashMap<SnapshotFactKey, Vec<(SnapshotFactContext, bool)>>,
    remaining: usize,
    pending: bool,
    /// Incremented whenever a query suspends, so a caller can tell whether the
    /// work it just ran was complete or provisional.
    suspensions: u64,
    /// Node pops consumed by the most recent slice.
    last_slice_nodes: usize,
}

thread_local! {
    static SESSION: RefCell<Option<ManaAnalysisSession>> = const { RefCell::new(None) };
}

impl ManaAnalysisSession {
    pub fn run<T>(&mut self, budget: usize, compute: impl FnOnce() -> T) -> (T, bool) {
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
            !session.pending
        });
        drop(restore);
        (result, complete)
    }

    /// Node pops the last slice actually consumed. A slice that returns fewer
    /// nodes than its budget was limited by the fixed cost of re-enumerating
    /// the menu, not by the search, which is what the scheduler needs to know
    /// to size the next slice.
    pub fn last_slice_nodes(&self) -> usize {
        self.last_slice_nodes
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
        if suspensions_before == Some(session.suspensions) {
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
            let mut remaining = SYNC_SEARCH_NODE_LIMIT;
            advance(
                &mut search,
                pips,
                sources,
                max_life,
                policy,
                source_policies,
                &mut remaining,
            );
            // Withhold rather than hang: an unsettled search at this depth is
            // pathological, and execution revalidates payment regardless.
            search.result.unwrap_or(false)
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
        search.classes.representatives(&node.used, &mut representatives);
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
