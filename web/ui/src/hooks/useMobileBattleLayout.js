import { useLayoutEffect, useState } from "react";
import { solveMobileBattleLayout } from "@/lib/mobile-battle-layout";

// Measure the scene, not the window. Its parent owns Safari's safe areas, and
// ResizeObserver also catches browser chrome, split views and orientation changes.
export default function useMobileBattleLayout(config, sceneRef) {
  const [viewport, setViewport] = useState({ width: 852, height: 320 });

  useLayoutEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return undefined;
    const update = () => {
      const { width, height } = scene.getBoundingClientRect();
      const next = { width: Math.floor(width), height: Math.floor(height) };
      setViewport((current) => current.width === next.width && current.height === next.height
        ? current : next);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(scene);
    return () => observer.disconnect();
  }, [sceneRef]);

  return solveMobileBattleLayout({
    ...config,
    viewportWidth: viewport.width,
    viewportHeight: viewport.height,
  });
}
