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
      const flavor = box.querySelector(".inspector-flavor-text");
      box.style.removeProperty("--card-fitted-rules-font-size");
      box.style.removeProperty("--card-fitted-flavor-font-size");
      box.style.setProperty("--card-rules-fit-scale", "1");
      box.style.setProperty("--card-rules-spacing-scale", "1");
      box.dataset.textOverflow = "false";
      const preferred = parseFloat(getComputedStyle(box).fontSize);
      const flavorPreferred = flavor ? parseFloat(getComputedStyle(flavor).fontSize) : preferred;
      const apply = scale => {
        box.style.setProperty("--card-fitted-rules-font-size", `${preferred * scale}px`);
        box.style.setProperty("--card-fitted-flavor-font-size", `${flavorPreferred * scale}px`);
        box.style.setProperty("--card-rules-fit-scale", String(scale));
      };
      // Scroll extents include highlights and flex layout, which can report
      // overflow even with ample room for the actual text. Fit glyph contents.
      // Registered fields have no padding: text may run to their edges, while
      // padded boxes keep a small margin so decoration never clips.
      const fits = () => {
        const bounds = box.getBoundingClientRect();
        const style = getComputedStyle(box);
        const inset = side => Math.min(3, parseFloat(style.getPropertyValue(`padding-${side}`)) || 0);
        return [...box.querySelectorAll('.interactive-card-frame__rule-line')].every(node => {
          const range = document.createRange();
          range.selectNodeContents(node);
          const text = range.getBoundingClientRect();
          return !text.height || (text.bottom <= bounds.bottom - inset('bottom') + .5
            && text.right <= bounds.right - inset('right') + .5 && text.left >= bounds.left + inset('left') - .5);
        });
      };
      // Remove UI-only padding/gaps before changing the printing's typography.
      if (!fits()) {
        box.style.setProperty("--card-rules-spacing-scale", "0");
        if (fits()) {
          let low = 0, high = 1;
          for (let i = 0; i < 8; i++) {
            const spacing = (low + high) / 2;
            box.style.setProperty("--card-rules-spacing-scale", String(spacing));
            if (fits()) low = spacing; else high = spacing;
          }
          box.style.setProperty("--card-rules-spacing-scale", String(low));
        } else {
          // Longer translations/live text may need smaller type, but never
          // collapse an entire card to one-pixel lettering to hide overflow.
          let low = .75, high = 1;
          apply(low);
          if (fits()) {
            for (let i = 0; i < 10; i++) {
              const scale = (low + high) / 2;
              apply(scale);
              if (fits()) low = scale; else high = scale;
            }
            apply(low);
          }
          box.dataset.textOverflow = String(!fits());
        }
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
