// Fits the labels of the bottom-right action dock to their buttons. A
// button's width depends on its neighbours (End turn, ») and the dock's
// capped width, and a label's length on the phase and the language, so a
// fixed font size either cuts long words ("ATACANTES") or wastes room. Each
// label steps its font down from the theme's size until it fits on at most
// two lines; past a readable floor it takes a third line instead (the button
// grows), and only a word wider than its button shrinks further.

const LABELS = [
  ".action-strip-main-title-text",
  ".decision-main-button:not(.priority-hold-window)",
  ".decision-phase-pass",
  ".decision-cancel-button",
  ".mana-payment-pay-button",
  ".action-strip-submit-button",
].join(", ");
const MAX_SIZE = 13;
// Below this a label stops shrinking to save a line and wraps to a third.
const TWO_LINE_FLOOR = 10.5;
// Only a single word wider than its button shrinks down to this.
const WORD_FLOOR = 9;
const STEP = 0.5;

const widthFits = (label) => label.scrollWidth <= label.clientWidth + 1;

// Buttons carry their own label (End turn, Pay); their height is the
// button's, so only the main title, a wrapping block, counts its lines.
function lineCountFits(label) {
  if (label.tagName === "BUTTON") return true;
  const style = getComputedStyle(label);
  const lineHeight = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.2;
  return label.scrollHeight <= lineHeight * 2 + 2;
}

function fitLabel(label) {
  if (!label.isConnected || label.clientWidth === 0) return;
  let size = MAX_SIZE;
  const apply = () => label.style.setProperty("font-size", `${size}px`, "important");
  apply();
  while (!(widthFits(label) && lineCountFits(label)) && size > TWO_LINE_FLOOR) {
    size -= STEP;
    apply();
  }
  while (!widthFits(label) && size > WORD_FLOOR) {
    size -= STEP;
    apply();
  }
}

export function installDockLabelFit() {
  if (typeof window === "undefined" || typeof ResizeObserver === "undefined") return () => {};
  let dock = null;
  let frame = 0;
  const resize = new ResizeObserver(() => schedule());
  const mutations = new MutationObserver(() => schedule());

  function fitAll() {
    frame = 0;
    if (!dock?.isConnected) return;
    for (const label of dock.querySelectorAll(LABELS)) fitLabel(label);
  }

  function schedule() {
    if (!frame) frame = requestAnimationFrame(fitAll);
  }

  function attach(next) {
    resize.disconnect();
    mutations.disconnect();
    dock = next;
    if (!dock) return;
    resize.observe(dock);
    mutations.observe(dock, { childList: true, subtree: true, characterData: true });
    schedule();
  }

  // The dock mounts and unmounts with games; a cheap check finds it again.
  const finder = window.setInterval(() => {
    const current = document.querySelector(".battlefield-human-action-dock");
    if (current !== dock) attach(current);
  }, 1000);

  return () => {
    window.clearInterval(finder);
    if (frame) cancelAnimationFrame(frame);
    resize.disconnect();
    mutations.disconnect();
  };
}
