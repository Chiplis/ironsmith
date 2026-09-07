import { useLayoutEffect, useRef } from "react";

// Keep the printing's preferred typography unless the complete line won't fit.
export default function CardFrameSingleLine({ as = "span", className, children }) {
  const textRef = useRef(null);
  const fitRef = useRef(null);

  useLayoutEffect(() => {
    const text = textRef.current;
    const range = document.createRange();
    let frame = 0;
    let active = true;
    const fit = () => {
      const available = text.getBoundingClientRect().width;
      if (!available) return;

      // Reset before measuring so shorter text, new fonts, and wider cards can
      // recover their original size. Flex layout reserves mana/count space.
      text.style.removeProperty("font-size");
      const preferred = parseFloat(getComputedStyle(text).fontSize);
      range.selectNodeContents(text);
      if (range.getBoundingClientRect().width <= available) return;

      let low = 0;
      let high = preferred;
      for (let i = 0; i < 12; i++) {
        const size = (low + high) / 2;
        text.style.fontSize = `${size}px`;
        if (range.getBoundingClientRect().width <= available) low = size;
        else high = size;
      }
      text.style.fontSize = `${Math.floor(low * 100) / 100}px`;
    };
    const scheduleFit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => { if (active) fit(); });
    };
    fitRef.current = fit;
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(text);
    document.fonts.ready.then(() => { if (active) scheduleFit(); });
    document.fonts.addEventListener("loadingdone", scheduleFit);
    return () => {
      active = false;
      fitRef.current = null;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.fonts.removeEventListener("loadingdone", scheduleFit);
    };
  }, []);

  // The parent can update sampled font metrics without changing the text.
  useLayoutEffect(() => { fitRef.current?.(); });

  return as === "h2"
    ? <h2 ref={textRef} className={className}>{children}</h2>
    : <span ref={textRef} className={className}>{children}</span>;
}
