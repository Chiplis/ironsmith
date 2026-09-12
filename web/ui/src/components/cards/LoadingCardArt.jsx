import { useState } from "react";
import CardArtLoader from "./CardArtLoader";

function ArtImage({ src, pending, variant, colors, svg = false, onLoad, ...imageProps }) {
  const [status, setStatus] = useState("loading");
  const loaded = Boolean(src) && (status === "loaded" || status === "revealing");
  const failed = status === "failed" || (!pending && !src);
  const events = {
    onLoad: event => {
      setStatus(current => current === "loading"
        ? (window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "loaded" : "revealing")
        : current);
      onLoad?.(event);
    },
    onAnimationEnd: event => {
      if (event.target === event.currentTarget) setStatus(current => current === "revealing" ? "loaded" : current);
    },
    onError: () => setStatus("failed"),
  };
  const revealProps = {
    "data-card-art-state": status,
    "data-card-art-svg": svg ? "true" : undefined,
  };
  const sheen = status === "revealing" && <span className="card-art-reveal-sheen" aria-hidden="true" />;
  const placeholder = !loaded && <CardArtLoader variant={variant} failed={failed} colors={colors} />;
  if (svg) return <>
    {src && <image {...imageProps} href={src} {...events} {...revealProps} opacity={loaded ? 1 : 0} />}
    {sheen && <foreignObject x={imageProps.x} y={imageProps.y} width={imageProps.width} height={imageProps.height} clipPath={imageProps.clipPath} style={{ pointerEvents: "none", overflow: "hidden" }}>{sheen}</foreignObject>}
    {placeholder && <foreignObject x={imageProps.x} y={imageProps.y} width={imageProps.width} height={imageProps.height} clipPath={imageProps.clipPath}>{placeholder}</foreignObject>}
  </>;
  return <>
    {src && <img {...imageProps} src={src} {...events} {...revealProps} style={{ ...imageProps.style, opacity: loaded ? 1 : 0 }} />}
    {sheen}
    {placeholder}
  </>;
}

// Source changes reset the state. Derived hand-corner repairs retain loaded state.
export default function LoadingCardArt({ sourceKey, ...props }) {
  return <ArtImage key={sourceKey || props.src || "pending"} {...props} />;
}
