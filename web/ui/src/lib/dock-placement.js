// Places the plain action row (main action, End turn, ») of the desktop HUD,
// in both battlefield appearances: its buttons share the vertical centre of the bottom-left
// navigation (Lobby / Build), and the row sits centred between the hand's
// right edge and the screen's, the same gap on either side. Decision panels
// (targets, payments, combat) keep the dock's own placement, since they need
// the height above.
//
// The dock positions itself with an inline inset that it rewrites on its own
// schedule, so this only publishes --fc-dock-left / --fc-dock-top and a
// data-fc-placed flag; desktop-full-cards.css applies them over the inset.
//
// The row keeps one place: it is measured before the browser paints, so it
// never shows at the dock's own inset first, and its right edge stays fixed
// as buttons come and go, with no glide: a phase change never shows the
// row drifting or dropping into place.

const DOCK = ".battlefield-human-action-dock";
const PANEL = ".action-strip-decision-stack, .combat-decision-panel, .mana-payment-editor";
const MIN_GAP = 12;
// Main action (190px) + End turn + » with their gaps: the row's usual width,
// so the first phases (before End turn shows) already use the full slot.
const MIN_SLOT = 336;

// The slot's width is remembered for the tab, so later games start with it.
const SLOT_KEY = "ironsmith.dockSlotWidth.v1";
let slotWidth = Math.max(MIN_SLOT, readSlot());
let anchor = null;

function readSlot() {
  try { return Number(sessionStorage.getItem(SLOT_KEY)) || 0; } catch { return 0; }
}

function rememberSlot() {
  try { sessionStorage.setItem(SLOT_KEY, String(Math.round(slotWidth))); } catch { /* not stored */ }
}

function setVar(element, name, value) {
  if (element.style.getPropertyValue(name) !== value) element.style.setProperty(name, value);
}

function place(dock) {
  const hand = document.querySelector(".hand-zone-cycle");
  const nav = document.querySelector(".home-navigation");
  if (!hand || dock.querySelector(PANEL)) {
    if (dock.hasAttribute("data-fc-placed")) dock.removeAttribute("data-fc-placed");
    return;
  }
  const row = dock.querySelector(".decision-main-button, .battlefield-auto-pass-toggle");
  const dockRect = dock.getBoundingClientRect();
  const buttons = [...dock.querySelectorAll("button")]
    .map((button) => button.getBoundingClientRect())
    .filter((rect) => rect.width > 0);
  if (!row || dockRect.width === 0 || !buttons.length) return;

  // Centre a slot as wide as the widest row seen (main action, End turn, »),
  // not the current buttons. » sits at its right edge and the main action
  // stretches from its left edge, so both keep their place whether End turn
  // shows or not.
  const buttonsLeft = Math.min(...buttons.map((rect) => rect.left));
  const buttonsRight = Math.max(...buttons.map((rect) => rect.right));
  const handRight = hand.getBoundingClientRect().right;
  const space = window.innerWidth - handRight;
  const room = space - MIN_GAP * 2;
  slotWidth = Math.max(slotWidth, buttonsRight - buttonsLeft);
  rememberSlot();
  const slot = Math.min(slotWidth, room);
  // The hand widens and narrows as cards come and go; the row keeps the spot
  // it took until the window resizes, the slot grows or the hand reaches it.
  let slotRight = anchor?.viewport === window.innerWidth && anchor.slot === slot ? anchor.right : 0;
  if (!slotRight || slotRight - slot < handRight + MIN_GAP) {
    slotRight = handRight + Math.max(MIN_GAP, (space - slot) / 2) + slot;
    anchor = { viewport: window.innerWidth, slot, right: slotRight };
  }
  const left = slotRight - (buttonsRight - dockRect.left);

  const mains = [...dock.querySelectorAll(".decision-main-button")]
    .filter((button) => button.offsetWidth > 0 && !button.closest(".decision-quick-controls"));
  if (mains.length) {
    const current = mains.reduce((sum, button) => sum + button.getBoundingClientRect().width, 0);
    const each = (current + slot - (buttonsRight - buttonsLeft)) / mains.length;
    setVar(dock, "--fc-main-width", `${Math.round(Math.min(300, Math.max(100, each)))}px`);
  }

  // Keep the current offset of the buttons' centre inside the dock.
  const rowRect = row.getBoundingClientRect();
  const rowOffset = rowRect.top + rowRect.height / 2 - dockRect.top;
  const navRect = nav?.getBoundingClientRect();
  const centre = navRect && navRect.height > 0
    ? navRect.top + navRect.height / 2
    : rowRect.top + rowRect.height / 2;

  setVar(dock, "--fc-dock-left", `${Math.round(left)}px`);
  setVar(dock, "--fc-dock-top", `${Math.round(centre - rowOffset)}px`);
  setVar(dock, "--fc-dock-max-width", `${Math.round(space - MIN_GAP * 2)}px`);
  if (!dock.hasAttribute("data-fc-placed")) dock.setAttribute("data-fc-placed", "");
}

export function installDockPlacement() {
  if (typeof window === "undefined" || typeof ResizeObserver === "undefined") return () => {};
  let dock = null;
  let frame = 0;
  let placing = false;

  // Observer callbacks run before the next paint, so placing from them means
  // the row is never drawn in the wrong spot first.
  function run() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    if (placing || !dock?.isConnected) return;
    placing = true;
    try { place(dock); } finally { placing = false; }
  }
  const schedule = () => { if (!frame) frame = requestAnimationFrame(run); };
  const resize = new ResizeObserver(run);
  const mutations = new MutationObserver(run);

  function attach(next) {
    resize.disconnect();
    mutations.disconnect();
    dock = next;
    if (!dock) return;
    resize.observe(dock);
    const hand = document.querySelector(".hand-zone-cycle");
    if (hand) resize.observe(hand);
    mutations.observe(dock, { childList: true, subtree: true, attributes: true, attributeFilter: ["style", "class"] });
    run();
  }

  // The dock and the hand mount and unmount with games: notice a new dock as
  // soon as it is inserted, with a slow poll as a fallback.
  const find = () => {
    if (dock?.isConnected) return;
    const current = document.querySelector(DOCK);
    if (current !== dock) attach(current);
  };
  const mounts = new MutationObserver(find);
  mounts.observe(document.body, { childList: true, subtree: true });
  const finder = window.setInterval(() => {
    const current = document.querySelector(DOCK);
    if (current !== dock) attach(current);
    else schedule();
  }, 1000);
  window.addEventListener("resize", schedule);
  find();

  return () => {
    window.clearInterval(finder);
    window.removeEventListener("resize", schedule);
    if (frame) cancelAnimationFrame(frame);
    resize.disconnect();
    mutations.disconnect();
    mounts.disconnect();
  };
}
