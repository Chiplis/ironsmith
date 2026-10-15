// Temporary in-browser driver for verifying CardStackPile motion by hand.
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const H = { sleep, log: [], fxFrames: 0 };

H.drag = async (fromEl, toX, toY) => {
  const r = fromEl.getBoundingClientRect();
  const x = r.x + r.width / 2;
  const y = r.y + 24;
  const opts = (px, py, up) => ({ bubbles: true, cancelable: true, composed: true, clientX: px, clientY: py, pointerId: 1, pointerType: "mouse", isPrimary: true, button: 0, buttons: up ? 0 : 1 });
  fromEl.dispatchEvent(new PointerEvent("pointerdown", opts(x, y)));
  fromEl.dispatchEvent(new MouseEvent("mousedown", opts(x, y)));
  for (let i = 1; i <= 24; i += 1) {
    const cx = x + (toX - x) * i / 24;
    const cy = y + (toY - y) * i / 24;
    const t = document.elementFromPoint(cx, cy) || document.body;
    t.dispatchEvent(new PointerEvent("pointermove", opts(cx, cy)));
    window.dispatchEvent(new PointerEvent("pointermove", opts(cx, cy)));
    await sleep(16);
  }
  const t = document.elementFromPoint(toX, toY) || document.body;
  t.dispatchEvent(new PointerEvent("pointerup", opts(toX, toY, true)));
  window.dispatchEvent(new PointerEvent("pointerup", opts(toX, toY, true)));
  t.dispatchEvent(new MouseEvent("mouseup", opts(toX, toY, true)));
};
H.tap = (el, dy = 12) => {
  const r = el.getBoundingClientRect();
  const x = r.x + r.width / 2;
  const y = r.y + dy;
  const o = { bubbles: true, cancelable: true, clientX: x, clientY: y, pointerId: 1, pointerType: "mouse", isPrimary: true, button: 0, buttons: 1 };
  const t = document.elementFromPoint(x, y);
  t.dispatchEvent(new PointerEvent("pointerdown", o));
  t.dispatchEvent(new PointerEvent("pointerup", { ...o, buttons: 0 }));
  t.dispatchEvent(new MouseEvent("click", { ...o, buttons: 0 }));
};
H.label = () => document.querySelector(".decision-main-button")?.textContent || "";
H.finish = async (targetFn) => {
  await sleep(1200);
  if (/submit targets \(0\)/i.test(H.label()) && targetFn) { H.tap(targetFn()); await sleep(1200); }
  if (/submit targets \(1\)/i.test(H.label())) { document.querySelector(".decision-main-button").click(); await sleep(1500); }
  if (/pay/i.test(H.label())) { document.querySelector(".decision-main-button").click(); await sleep(2500); }
  return H.label();
};
H.card = (n) => [...document.querySelectorAll(".battlefield-row .game-card[data-object-id]")].find((e) => new RegExp(n, "i").test(e.getAttribute("data-card-name") || ""));
H.hand = (n) => [...document.querySelectorAll(".hand-card")].find((e) => e.getAttribute("data-card-name") === n);
H.center = (e) => { const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; };
H.pile = (n) => [...document.querySelectorAll(".card-stack-slot .stack-card--frame")].find((e) => e.dataset.cardName === n);
H.slots = () => [...document.querySelectorAll(".card-stack-slot")].map((s) => {
  const r = s.getBoundingClientRect();
  return [s.querySelector("[data-card-name]")?.dataset.cardName, Object.keys(s.dataset).filter((k) => k !== "stackKey").join("+"), Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height), s.querySelector(".card-stack-motion")?.style.opacity];
});
H.walk = async () => {
  for (let i = 0; i < 20; i += 1) {
    const b = document.querySelector(".decision-main-button");
    if (!b) { await sleep(1500); continue; }
    const l = b.textContent.trim();
    if (/^go to combat/i.test(l)) return l;
    if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(l)) return `stop:${l}`;
    b.click();
    await sleep(1500);
  }
  return "timeout";
};
const t0 = performance.now();
new MutationObserver((records) => {
  for (const m of records) {
    if (!(m.target instanceof Element)) continue;
    if (!/^data-(materialize|burning|promoted|casting|pulled|arriving|viewed|leaving)$/.test(m.attributeName)) continue;
    const card = m.target.closest(".card-stack-slot")?.querySelector("[data-card-name]")?.dataset.cardName;
    H.log.push([Math.round(performance.now() - t0), card, m.attributeName, m.target.getAttribute(m.attributeName)]);
  }
}).observe(document.body, { subtree: true, attributes: true });
(function poll() {
  const canvas = document.querySelector(".stack-fx-canvas");
  if (canvas && canvas.style.display === "block") H.fxFrames += 1;
  requestAnimationFrame(poll);
}());
window.H = H;
export default H;
