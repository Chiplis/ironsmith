import { useLayoutEffect, useMemo, useRef } from 'react';
import { rasterizeFrameBevel } from '@/lib/card-border-analysis';

export default function CardFrameBorder({profile}) {
  const canvasRef = useRef(null);
  const profiles = useMemo(() => JSON.parse(profile), [profile]);
  useLayoutEffect(() => {
    const canvas = canvasRef.current;
    let frame = 0;
    const draw = () => {
      const {width, height} = canvas.getBoundingClientRect();
      if (!width || !height) return;
      const measured = canvas.closest('.interactive-card-frame-stage').dataset.boxSizing === 'measured';
      const radius = parseFloat(getComputedStyle(measured ? canvas : canvas.parentElement).borderTopLeftRadius);
      const stageWidth = canvas.closest('.interactive-card-frame').getBoundingClientRect().width;
      const pixels = rasterizeFrameBevel({width, height, radius, step: stageWidth / 488, profiles, pixelRatio: Math.min(window.devicePixelRatio || 1, 2)});
      canvas.width = pixels.width; canvas.height = pixels.height;
      canvas.getContext('2d').putImageData(new ImageData(pixels.data, pixels.width, pixels.height), 0, 0);
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(draw);
    });
    observer.observe(canvas);
    draw();
    return () => {observer.disconnect(); cancelAnimationFrame(frame);};
  }, [profiles]);
  return <canvas ref={canvasRef} className="interactive-card-frame__bevel" aria-hidden="true" />;
}
