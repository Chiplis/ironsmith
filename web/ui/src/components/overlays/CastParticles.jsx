import { useState, useEffect, useCallback } from "react";

const PARTICLE_COUNT = 14;

const GLOW_COLORS = {
  land: ["#58d6a6", "#3fbf8c"],
  spell: ["#64a9ff", "#4a8fee"],
  ability: ["#e0e8f0", "#c0d0e0"],
};

export default function CastParticles() {
  const [bursts, setBursts] = useState([]);

  const spawnBurst = useCallback((x, y, glowKind) => {
    const id = Date.now() + Math.random();
    const colors = GLOW_COLORS[glowKind] || GLOW_COLORS.spell;
    const particles = Array.from({ length: PARTICLE_COUNT }, (_, i) => {
      const angle = (i / PARTICLE_COUNT) * Math.PI * 2 + (Math.random() - 0.5) * 0.4;
      const dist = 40 + Math.random() * 50;
      const color = colors[Math.floor(Math.random() * colors.length)];
      const size = 3 + Math.random() * 4;
      return { angle, dist, color, size };
    });
    setBursts((prev) => [...prev, { id, x, y, particles }]);
    setTimeout(() => {
      setBursts((prev) => prev.filter((b) => b.id !== id));
    }, 700);
  }, []);

  // Expose spawnBurst globally so Workspace can call it
  useEffect(() => {
    window.__castParticles = spawnBurst;
    return () => { delete window.__castParticles; };
  }, [spawnBurst]);

  return (
    <>
      {bursts.map((burst) => (
        <div key={burst.id} className="fixed z-[101] pointer-events-none" style={{ left: burst.x, top: burst.y }}>
          {burst.particles.map((p, i) => (
            <div
              key={i}
              className="absolute rounded-full"
              style={{
                width: p.size,
                height: p.size,
                backgroundColor: p.color,
                boxShadow: `0 0 6px 2px ${p.color}`,
                animation: `cast-particle 600ms ease-out forwards`,
                "--cast-tx": `${Math.cos(p.angle) * p.dist}px`,
                "--cast-ty": `${Math.sin(p.angle) * p.dist}px`,
              }}
            />
          ))}
          {/* Central flash */}
          <div
            className="absolute rounded-full"
            style={{
              width: 16,
              height: 16,
              left: -8,
              top: -8,
              backgroundColor: "rgba(255,255,255,0.9)",
              boxShadow: "0 0 20px 8px rgba(255,255,255,0.4)",
              animation: "cast-flash 400ms ease-out forwards",
            }}
          />
        </div>
      ))}
    </>
  );
}
