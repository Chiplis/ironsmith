import { cn } from "@/lib/utils";

function glowPhaseFromSeed(seed) {
  let hash = 0;
  const text = String(seed || "");
  for (let i = 0; i < text.length; i++) {
    hash = ((hash * 31) + text.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

export default function AnimatedCircuitFrame({
  seed = "",
  path = "",
  viewBox = "0 0 100 140",
  className = "",
  overlayClassName = "",
  svgClassName = "",
}) {
  const glowPhase = glowPhaseFromSeed(seed);
  const circuitStyle = {
    "--circuit-delay": `-${((glowPhase % 3200) / 1000).toFixed(3)}s`,
    "--circuit-duration": `${(3.1 + ((glowPhase % 900) / 1000)).toFixed(3)}s`,
    "--circuit-accent-delay": `-${(((glowPhase * 17) % 4100) / 1000).toFixed(3)}s`,
  };
  const edges = ["top", "right", "bottom", "left"];

  if (!path) return null;

  return (
    <div
      className={cn("card-circuit-overlay", overlayClassName)}
      style={circuitStyle}
      aria-hidden="true"
      data-circuit-path={path || undefined}
      data-circuit-viewbox={viewBox}
    >
      <div className={cn("card-circuit-strip-set", svgClassName, className)}>
        {edges.map((edge) => (
          <span key={edge} className={`card-circuit-edge card-circuit-edge-${edge}`}>
            <span className="card-circuit-stream card-circuit-stream-a" />
            <span className="card-circuit-stream card-circuit-stream-b" />
          </span>
        ))}
      </div>
    </div>
  );
}
