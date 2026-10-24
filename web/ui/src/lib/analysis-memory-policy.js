// iOS can kill the page instead of throwing when a second WASM heap is
// allocated. iPadOS may identify as a Mac; touch points are only available on
// the window navigator, so resolve this before starting the engine worker.
export function shouldShareAnalysisRuntime(navigator = globalThis.navigator) {
  return /iPad|iPhone|iPod/.test(navigator?.userAgent || '')
    || (/Mac/.test(navigator?.platform || '') && navigator?.maxTouchPoints > 1);
}

export function preferredVerifierWorkerCount(navigator = globalThis.navigator, configured) {
  // Core counts say nothing about iOS's per-page memory budget. Even an
  // explicit desktop pool setting must not multiply verifier heaps there.
  if (shouldShareAnalysisRuntime(navigator)) return 1;
  const requested = Number(configured);
  if (Number.isFinite(requested) && requested > 0) return Math.max(1, Math.min(8, Math.floor(requested)));
  const cores = Number(navigator?.hardwareConcurrency || 2);
  if (!Number.isFinite(cores) || cores <= 2) return 1;
  if (cores <= 4) return 2;
  if (cores <= 6) return 3;
  return 4;
}
