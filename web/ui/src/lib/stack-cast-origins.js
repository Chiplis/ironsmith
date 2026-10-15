// Where a spell came from on screen, for the comet that carries it onto the
// card stack (CardStackPile). A hand drag reports its release point; a click
// or keyboard cast leaves the rect of the hand card it started from. While a
// drag is still held -- a targeted spell enters the stack as soon as it
// leaves the hand -- arrivals wait for its release.

const RELEASE_TTL_MS = 5000;
const HAND_PRESS_TTL_MS = 20000;

let activeDrag = null;
let lastRelease = null;
let lastHandPress = null;
const waiters = new Set();

function sameName(a, b) {
  return String(a || "").trim().toLowerCase() === String(b || "").trim().toLowerCase();
}

/** The live hand drag (from Workspace's drag session), or null when none. */
export function noteCastDrag(cardName) {
  const previous = activeDrag;
  activeDrag = cardName ? { cardName } : null;
  // A drag that ends without a release over the table settles its waiters
  // with whatever release (if any) it reported.
  if (previous && !activeDrag) {
    for (const waiter of [...waiters]) {
      if (!sameName(waiter.cardName, previous.cardName)) continue;
      waiters.delete(waiter);
      waiter.resolve(lastRelease && sameName(lastRelease.cardName, previous.cardName) ? lastRelease : null);
    }
  }
}

/** The point a dragged hand card was let go at. */
export function noteCastRelease(cardName, x, y) {
  if (!cardName || !Number.isFinite(x) || !Number.isFinite(y)) return;
  lastRelease = { cardName, point: { x, y }, at: performance.now(), consumed: false };
  for (const waiter of [...waiters]) {
    if (!sameName(waiter.cardName, cardName)) continue;
    waiters.delete(waiter);
    lastRelease.consumed = true;
    waiter.resolve(lastRelease);
  }
}

function rememberHandCard(target) {
  const card = target?.closest?.(".game-card.hand-card[data-card-name]");
  if (!card || card.closest("[data-card-stack-pile]")) return;
  const rect = card.getBoundingClientRect();
  if (rect.width <= 0) return;
  lastHandPress = { cardName: card.getAttribute("data-card-name"), rect, at: performance.now() };
}

if (typeof document !== "undefined") {
  document.addEventListener("pointerdown", (event) => rememberHandCard(event.target), true);
  document.addEventListener("focusin", (event) => rememberHandCard(event.target), true);
}

/**
 * Resolves with { point } or { rect } for the spell named `cardName`, or
 * null when nothing on screen launched it (an opponent's cast, a cascade).
 */
export function castOrigin(cardName, { maxWaitMs = 30000 } = {}) {
  const now = performance.now();
  if (activeDrag && sameName(activeDrag.cardName, cardName)) {
    return new Promise((resolve) => {
      const waiter = { cardName, resolve };
      waiters.add(waiter);
      window.setTimeout(() => {
        if (!waiters.delete(waiter)) return;
        resolve(null);
      }, maxWaitMs);
    });
  }
  if (lastRelease && !lastRelease.consumed && sameName(lastRelease.cardName, cardName)
    && now - lastRelease.at < RELEASE_TTL_MS) {
    lastRelease.consumed = true;
    return Promise.resolve(lastRelease);
  }
  if (lastHandPress && sameName(lastHandPress.cardName, cardName) && now - lastHandPress.at < HAND_PRESS_TTL_MS) {
    const press = lastHandPress;
    lastHandPress = null;
    return Promise.resolve(press);
  }
  return Promise.resolve(null);
}
