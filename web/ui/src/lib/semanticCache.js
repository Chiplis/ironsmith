const DEFAULT_CACHE_FILE = "ironsmith_semantic_scores.json";

function coerceClampedScore(raw) {
  const score = Number(raw);
  if (!Number.isFinite(score)) return null;
  return Math.min(1, Math.max(0, score));
}

function maybeInsertScore(scoresByName, rawName, rawScore) {
  if (typeof rawName !== "string") return;
  const name = rawName.trim().toLowerCase();
  const score = coerceClampedScore(rawScore);
  if (!name || score === null) return;
  const previous = scoresByName.get(name);
  if (previous === undefined || score > previous) {
    scoresByName.set(name, score);
  }
}

export function buildSemanticStats(payload) {
  if (
    payload
    && typeof payload === "object"
    && Array.isArray(payload.thresholdCounts)
    && Number.isFinite(Number(payload.scoredCount))
  ) {
    const thresholdCounts = payload.thresholdCounts
      .slice(0, 100)
      .map((value) => {
        const count = Number(value);
        return Number.isFinite(count) ? Math.max(0, Math.floor(count)) : 0;
      });
    while (thresholdCounts.length < 100) {
      thresholdCounts.push(0);
    }
    return {
      scoredCount: Math.max(0, Math.floor(Number(payload.scoredCount))),
      thresholdCounts,
    };
  }

  const scoresByName = new Map();

  if (Array.isArray(payload)) {
    for (const entry of payload) {
      if (!entry || typeof entry !== "object") continue;
      maybeInsertScore(scoresByName, entry.name, entry.similarity_score);
    }
  } else if (payload && typeof payload === "object") {
    if (Array.isArray(payload.entries)) {
      for (const entry of payload.entries) {
        if (!entry || typeof entry !== "object") continue;
        if (entry.parse_error !== null && entry.parse_error !== undefined) continue;
        if (Boolean(entry.has_unimplemented)) continue;
        maybeInsertScore(scoresByName, entry.name, entry.similarity_score);
      }
    } else {
      for (const [name, score] of Object.entries(payload)) {
        maybeInsertScore(scoresByName, name, score);
      }
    }
  }

  const thresholdCounts = Array(100).fill(0);
  for (const score of scoresByName.values()) {
    const thresholdsMet = Math.floor(score * 100);
    for (let idx = 0; idx < thresholdsMet; idx += 1) {
      thresholdCounts[idx] += 1;
    }
  }

  return {
    scoredCount: scoresByName.size,
    thresholdCounts,
  };
}

export function cardsMeetingThresholdFromStats(thresholdPercent, stats) {
  if (
    !stats
    || !Array.isArray(stats.thresholdCounts)
    || stats.thresholdCounts.length !== 100
    || !Number.isFinite(stats.scoredCount)
  ) {
    return null;
  }

  const threshold = Number(thresholdPercent);
  if (!Number.isFinite(threshold) || threshold <= 0) {
    return stats.scoredCount;
  }

  const thresholdIndex = Math.min(100, Math.max(1, Math.ceil(threshold))) - 1;
  return stats.thresholdCounts[thresholdIndex] ?? 0;
}

export async function loadSemanticStats(fileName = DEFAULT_CACHE_FILE) {
  const base = import.meta.env.BASE_URL || "/";
  const normalizedBase = base.endsWith("/") ? base : `${base}/`;
  const url = new URL(`${normalizedBase}${fileName}`, window.location.href);
  const response = await fetch(url.toString(), { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`semantic cache fetch failed: HTTP ${response.status}`);
  }
  const payload = await response.json();
  return buildSemanticStats(payload);
}
