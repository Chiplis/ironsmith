import { useLayoutEffect, useRef } from "react";
import "@/styles/card-frame-text-fit.css";

export default function CardFrameRulesBox({ children, label }) {
  const boxRef = useRef(null);
  const fitRef = useRef(null);

  useLayoutEffect(() => {
    const box = boxRef.current;
    let frame = 0;
    let active = true;
    const fit = () => {
      const line = box.querySelector(".interactive-card-frame__rule-line");
      if (!line || !box.clientWidth || !box.clientHeight) return;

      // Start from the printing's preferred size every time, allowing text to
      // grow back after a resize or a switch to a shorter card description.
      box.style.removeProperty("--card-fitted-rules-font-size");
      box.style.setProperty("--card-rules-fit-scale", "1");
      const preferred = parseFloat(getComputedStyle(line).fontSize);
      const apply = size => {
        box.style.setProperty("--card-fitted-rules-font-size", `${size}px`);
        box.style.setProperty("--card-rules-fit-scale", String(size / preferred));
      };
      const fits = () => box.scrollHeight <= box.clientHeight + 1 && box.scrollWidth <= box.clientWidth + 1;

      if (!fits()) {
        let low = Math.min(1, preferred);
        let high = preferred;
        // Layout-aware search accounts for line wrapping, mana symbols,
        // ability padding, and flavor text instead of estimating by length.
        for (let i = 0; i < 12; i++) {
          const size = (low + high) / 2;
          apply(size);
          if (fits()) low = size;
          else high = size;
        }
        apply(Math.floor(low * 100) / 100);
      } else {
        apply(preferred);
      }
      box.scrollTop = 0;
    };
    const scheduleFit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => { if (active) fit(); });
    };
    fitRef.current = fit;
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(box);
    if (box.firstElementChild) observer.observe(box.firstElementChild);
    document.fonts.ready.then(() => { if (active) scheduleFit(); });
    document.fonts.addEventListener("loadingdone", scheduleFit);
    fit();
    return () => {
      active = false;
      fitRef.current = null;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.fonts.removeEventListener("loadingdone", scheduleFit);
    };
  }, []);

  useLayoutEffect(() => { fitRef.current?.(); }, [children]);

  return <div ref={boxRef} className="interactive-card-frame__rules" data-fit-text="true" aria-label={label}>{children}</div>;
}
