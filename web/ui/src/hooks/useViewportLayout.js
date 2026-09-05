import { useEffect, useState } from "react";
import { classifyViewport } from "@/lib/viewport-layout";

function readViewportLayout() {
  if (typeof window === "undefined") return classifyViewport(1440, 900);
  return classifyViewport(window.innerWidth, window.innerHeight);
}

export default function useViewportLayout() {
  const [viewportLayout, setViewportLayout] = useState(readViewportLayout);

  useEffect(() => {
    const updateViewportLayout = () => {
      const next = readViewportLayout();
      setViewportLayout((current) => Object.keys(next).every((key) => next[key] === current[key]) ? current : next);
    };
    updateViewportLayout();
    window.addEventListener("resize", updateViewportLayout);
    return () => window.removeEventListener("resize", updateViewportLayout);
  }, []);

  return viewportLayout;
}
