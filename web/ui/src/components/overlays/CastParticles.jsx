import { useState, useEffect, useCallback, useLayoutEffect, useRef } from "react";
import { animate, cancelMotion, snappySpring } from "@/lib/motion/anime";

const PARTICLE_COUNT = 14;

const GLOW_COLORS = {
  land: ["#58d6a6", "#3fbf8c"],
  spell: ["#64a9ff", "#4a8fee"],
  ability: ["#e0e8f0", "#c0d0e0"],
};

function CastBurst({ burst }) {
  const particleRefs = useRef([]);
  const flashRef = useRef(null);
  const motionRefs = useRef([]);

  useLayoutEffect(() => {
    motionRefs.current.forEach(cancelMotion);
    motionRefs.current = [];

    for (let index = 0; index < burst.particles.length; index += 1) {
      const particle = burst.particles[index];
      const node = particleRefs.current[index];
      if (!node) continue;

      motionRefs.current.push(
        animate(node, {
          x: Math.cos(particle.angle) * particle.dist,
          y: Math.sin(particle.angle) * particle.dist,
          scale: [1, 0.24],
          opacity: [1, 0],
          ease: snappySpring({ duration: 520, bounce: 0.12 }),
        })
      );
    }

    if (flashRef.current) {
      motionRefs.current.push(
        animate(flashRef.current, {
          scale: [1, 2.65],
          opacity: [0.92, 0],
          ease: "out(4)",
          duration: 360,
        })
      );
    }

    return () => {
      motionRefs.current.forEach(cancelMotion);
      motionRefs.current = [];
    };
  }, [burst]);

  return (
    <div className="fixed z-[101] pointer-events-none" style={{ left: burst.x, top: burst.y }}>
      {burst.particles.map((particle, index) => (
        <div
          key={index}
          ref={(node) => {
            particleRefs.current[index] = node;
          }}
          className="absolute rounded-full"
          style={{
            width: particle.size,
            height: particle.size,
            backgroundColor: particle.color,
            boxShadow: `0 0 6px 2px ${particle.color}`,
          }}
        />
      ))}
      <div
        ref={flashRef}
        className="absolute rounded-full"
        style={{
          width: 16,
          height: 16,
          left: -8,
          top: -8,
          backgroundColor: "rgba(255,255,255,0.9)",
          boxShadow: "0 0 20px 8px rgba(255,255,255,0.4)",
        }}
      />
    </div>
  );
}

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
      {bursts.map((burst) => <CastBurst key={burst.id} burst={burst} />)}
    </>
  );
}
