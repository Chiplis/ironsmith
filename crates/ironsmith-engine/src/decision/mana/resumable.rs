//! Resumable exact mana search. A suspended query is never cached as unpayable.
use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct Node {
    pip: usize,
    pool: crate::player::ManaPool,
    snow: crate::player::ManaPool,
    used: Vec<bool>,
    life: u32,
}

#[derive(Debug, Clone)]
struct Search {
    frontier: Vec<Node>,
    seen: HashSet<(ManaPaymentSearchKey, Vec<bool>)>,
    result: Option<bool>,
}

/// Owned by a single immutable priority snapshot. Completed queries are reused
/// across menu passes; unfinished queries retain their frontier without restart.
#[derive(Debug, Default)]
pub struct ManaAnalysisSession {
    searches: HashMap<String, Search>,
    remaining: usize,
    pending: bool,
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
        self.remaining = budget.max(1);
        self.pending = false;
        SESSION.with(|slot| {
            assert!(slot.borrow().is_none(), "nested mana analysis session");
            *slot.borrow_mut() = Some(std::mem::take(self));
        });
        let restore = Restore(self);
        let result = compute();
        let complete = SESSION.with(|slot| !slot.borrow().as_ref().unwrap().pending);
        drop(restore);
        (result, complete)
    }
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
    // spending permissions. No game-pointer identity or incomplete hash key.
    let key =
        format!("{pips:?}/{pool:?}/{snow:?}/{sources:?}/{max_life}/{policy:?}/{source_policies:?}");
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
    };
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(session) = slot.as_mut() {
            let search = session.searches.entry(key).or_insert_with(initial);
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
                    false
                }
            }
        } else {
            let mut search = initial();
            let mut remaining = usize::MAX;
            advance(
                &mut search,
                pips,
                sources,
                max_life,
                policy,
                source_policies,
                &mut remaining,
            );
            search.result.expect("unbounded search completes")
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
            for (index, source) in sources.iter().enumerate().rev() {
                if node.used[index] {
                    continue;
                }
                // Interchangeable sources need only one branch. Their object
                // identity remains in the query key; equivalence is local to
                // this payment's resolved policy and output choices.
                if (0..index).any(|earlier| {
                    !node.used[earlier]
                        && sources[earlier].outputs == source.outputs
                        && sources[earlier].from_snow_source == source.from_snow_source
                        && source_policies[earlier] == source_policies[index]
                }) {
                    continue;
                }
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
