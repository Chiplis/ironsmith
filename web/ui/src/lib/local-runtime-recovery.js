// These are client-owned native handles, never network payloads.
export function createLocalRuntimeRecovery({ limit = 3 } = {}) {
  let points = [];
  const dispose = async point => {
    try { await point.snapshot.release(); } catch { /* An expired worker owns no recoverable handle. */ }
  };
  return {
    async remember(point) {
      const retired = points.filter(old => old.game !== point.game
        || old.matchId !== point.matchId || old.seat !== point.seat || old.seq === point.seq);
      points = points.filter(old => !retired.includes(old));
      points.push(point);
      points.sort((a, b) => b.seq - a.seq);
      retired.push(...points.splice(limit));
      await Promise.all(retired.map(dispose));
    },
    candidates({ game, matchId, seat, actions }) {
      return points.filter(point => point.game === game && point.matchId === matchId && point.seat === seat
        && point.seq > 0 && point.seq <= actions.length
        && actions[point.seq - 1]?.audit?.nextStateHash === point.snapshot.auditStateHash
        && actions[point.seq - 1]?.prefixHash === point.prefixHash);
    },
    async clear() {
      const retired = points;
      points = [];
      await Promise.all(retired.map(dispose));
    },
  };
}

// Every level must reproduce the signed head. A failed local level falls back
// to an older local point, then genesis; it never accepts a partial replay.
export async function recoverVerifiedRuntime({ current, saved, restore, genesis, replay, verify, onFailure }) {
  for (const candidate of [current, ...saved, { level: 'genesis', seq: 0 }].filter(Boolean)) {
    try {
      if (candidate.level === 'genesis') await genesis();
      else await restore(candidate);
      await replay(candidate.seq);
      await verify();
      return candidate;
    } catch (error) {
      onFailure?.(candidate, error);
      if (candidate.level === 'genesis') throw error;
    }
  }
}
