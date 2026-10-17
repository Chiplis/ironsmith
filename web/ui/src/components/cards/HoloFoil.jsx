import { useEffect, useRef } from "react";
import "./holo-foil.css";

// Holographic foil over a card face. The pointer position over the card feeds
// CSS variables on the card (--holo-x/--holo-y in percent, --holo-tilt-*),
// which the rainbow and glare layers in holo-foil.css read. Only variables
// change, once per frame at most; with no pointer the foil stays still.
// Without `glow` it only tracks the pointer, so a plain hand card tilts too.
export default function HoloFoil({ glow = true }) {
  const layer = useRef(null);

  useEffect(() => {
    const card = layer.current?.closest(".game-card");
    if (!card) return undefined;
    let frame = 0;
    let pending = null;

    const paint = () => {
      frame = 0;
      if (!pending) return;
      const { x, y } = pending;
      card.style.setProperty("--holo-x", `${x.toFixed(2)}%`);
      card.style.setProperty("--holo-y", `${y.toFixed(2)}%`);
      card.style.setProperty("--holo-tilt-x", `${(-(y - 50) / 50 * 6).toFixed(2)}deg`);
      card.style.setProperty("--holo-tilt-y", `${((x - 50) / 50 * 6).toFixed(2)}deg`);
    };

    const onMove = (event) => {
      const rect = card.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      pending = {
        x: Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100)),
        y: Math.min(100, Math.max(0, ((event.clientY - rect.top) / rect.height) * 100)),
      };
      card.dataset.holoActive = "true";
      if (!frame) frame = requestAnimationFrame(paint);
    };

    const onLeave = () => {
      pending = null;
      if (frame) cancelAnimationFrame(frame);
      frame = 0;
      delete card.dataset.holoActive;
      for (const name of ["--holo-x", "--holo-y", "--holo-tilt-x", "--holo-tilt-y"]) {
        card.style.removeProperty(name);
      }
    };

    card.addEventListener("pointermove", onMove, { passive: true });
    card.addEventListener("pointerleave", onLeave, { passive: true });
    return () => {
      card.removeEventListener("pointermove", onMove);
      card.removeEventListener("pointerleave", onLeave);
      onLeave();
    };
  }, []);

  if (!glow) return <span ref={layer} hidden aria-hidden="true" />;
  return (
    <span ref={layer} className="holo-foil" aria-hidden="true">
      <span className="holo-foil__rainbow" />
      <span className="holo-foil__glare" />
    </span>
  );
}
