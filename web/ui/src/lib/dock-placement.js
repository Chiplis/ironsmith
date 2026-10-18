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

const PANEL = ".action-strip-decision-stack, .combat-decision-panel, .mana-payment-editor";
const MIN_GAP = 12;

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

  // Centre the visible buttons, not the dock's box, which can be wider.
  const buttonsLeft = Math.min(...buttons.map((rect) => rect.left));
  const buttonsRight = Math.max(...buttons.map((rect) => rect.right));
  const buttonsBottom = Math.max(...buttons.map((rect) => rect.bottom));
  const handRight = hand.getBoundingClientRect().right;
  const space = window.innerWidth - handRight;
  const width = Math.min(buttonsRight - buttonsLeft, space - MIN_GAP * 2);
  let left = handRight + Math.max(MIN_GAP, (space - width) / 2) - (buttonsLeft - dockRect.left);

  // Keep the current offset of the buttons' centre inside the dock.
  const rowRect = row.getBoundingClientRect();
  const rowOffset = rowRect.top + rowRect.height / 2 - dockRect.top;
  const navRect = nav?.getBoundingClientRect();
  let centre = navRect && navRect.height > 0
    ? navRect.top + navRect.height / 2
    : rowRect.top + rowRect.height / 2;

  // With », the row continues the zone chip column: » shares the chips'
  // right edge, and the row rests on the screen's bottom edge, the same
  // margin below it as the chips keep to the right, leaving a gap above.
  const chips = [...document.querySelectorAll("[data-my-zone] .zone-pile--icon")]
    .map((chip) => chip.getBoundingClientRect())
    .filter((rect) => rect.width > 0);
  const autoPass = dock.querySelector(".battlefield-auto-pass-toggle")?.getBoundingClientRect();
  if (chips.length && autoPass?.width) {
    const chipsRight = Math.max(...chips.map((rect) => rect.right));
    const shiftedLeft = chipsRight - (autoPass.right - dockRect.left);
    if (shiftedLeft + (buttonsLeft - dockRect.left) >= handRight + MIN_GAP) {
      const autoPassMid = autoPass.top + autoPass.height / 2;
      const margin = Math.max(0, window.innerWidth - chipsRight);
      const target = window.innerHeight - margin - (buttonsBottom - autoPassMid);
      left = shiftedLeft;
      centre = target - autoPassMid + rowRect.top + rowRect.height / 2;
    }
  }

  setVar(dock, "--fc-dock-left", `${Math.round(left)}px`);
  setVar(dock, "--fc-dock-top", `${Math.round(centre - rowOffset)}px`);
  setVar(dock, "--fc-dock-max-width", `${Math.round(space - MIN_GAP * 2)}px`);
  if (!dock.hasAttribute("data-fc-placed")) dock.setAttribute("data-fc-placed", "");
}

export function installDockPlacement() {
  if (typeof window === "undefined" || typeof ResizeObserver === "undefined") return () => {};
  let dock = null;
  let frame = 0;
  const schedule = () => { if (!frame) frame = requestAnimationFrame(run); };
  const resize = new ResizeObserver(schedule);
  const mutations = new MutationObserver(schedule);

  function run() {
    frame = 0;
    if (dock?.isConnected) place(dock);
  }

  function attach(next) {
    resize.disconnect();
    mutations.disconnect();
    dock = next;
    if (!dock) return;
    resize.observe(dock);
    const hand = document.querySelector(".hand-zone-cycle");
    if (hand) resize.observe(hand);
    mutations.observe(dock, { childList: true, subtree: true, attributes: true, attributeFilter: ["style", "class"] });
    schedule();
  }

  // The dock and the hand mount and unmount with games; re-find them cheaply.
  const finder = window.setInterval(() => {
    const current = document.querySelector(".battlefield-human-action-dock");
    if (current !== dock) attach(current);
    else schedule();
  }, 1000);
  window.addEventListener("resize", schedule);

  return () => {
    window.clearInterval(finder);
    window.removeEventListener("resize", schedule);
    if (frame) cancelAnimationFrame(frame);
    resize.disconnect();
    mutations.disconnect();
  };
}
