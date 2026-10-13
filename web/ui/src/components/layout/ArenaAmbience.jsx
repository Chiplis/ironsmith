import useBattlefieldAppearance from "@/hooks/useBattlefieldAppearance";

// Living arena over the full-card backdrop (gothic-floating-arena-v2.png):
// distant lightning in the sky, small fires with embers on the painted
// flames, both answered by the wet stone floor, and faint motes in the air. Purely decorative: the markup is computed once at module load and
// never re-renders; everything moves with CSS animations on transform and
// opacity. It renders nothing with the compact battlefield appearance;
// arena-ambience.css places each piece on the art and shows the layer only
// for the full-card desktop HUD.

// A fixed seed keeps the scene identical between renders and reloads.
function seeded(seed) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const random = seeded(1672941);
const between = ([min, max]) => min + random() * (max - min);

/* ---- lightning ---------------------------------------------------------- */

// Bolts are drawn in a 100 x 300 box. Midpoint displacement gives the
// channel its jagged, many-scaled path; each pass halves the segments and
// nudges every new midpoint sideways in proportion to its segment.
function jagged(from, to, passes, roughness) {
  let points = [from, to];
  for (let pass = 0; pass < passes; pass += 1) {
    const next = [points[0]];
    for (let i = 1; i < points.length; i += 1) {
      const [ax, ay] = points[i - 1];
      const [bx, by] = points[i];
      const length = Math.hypot(bx - ax, by - ay);
      next.push([
        (ax + bx) / 2 + (random() - 0.5) * length * roughness,
        (ay + by) / 2 + (random() - 0.5) * length * roughness * 0.18,
      ], points[i]);
    }
    points = next;
  }
  return points;
}

const toPath = points => points
  .map(([x, y], index) => `${index ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join("");

function makeBolt({ length, roughness, branches }) {
  const startX = between([44, 56]);
  const main = jagged([startX, 0], [startX + between([-22, 22]), 300 * length], 6, roughness);
  const forks = Array.from({ length: branches }, () => {
    // Branches leave the middle of the channel and fall away to one side.
    const index = Math.floor(between([0.18, 0.7]) * main.length);
    const [x, y] = main[index];
    const side = random() < 0.5 ? -1 : 1;
    const reach = between([45, 110]);
    return toPath(jagged([x, y], [x + side * reach * between([0.35, 0.7]), y + reach], 4, 0.7));
  });
  return { main: toPath(main), forks };
}

// Each event: its place in the sky (sx, sy: fractions of the art, top of
// the bolt), its height (sh, fraction of the art's height), how bright it
// is, how long its cycle is and where it starts. Periods share no factor.
// floor: the stretch of wet stone under the flash whose painted highlights
// light up with it (x0, y0 to x1, y1: fractions of the art).
const STRIKES = [
  { name: "near-right", sx: 0.72, sy: 0.015, sh: 0.36, intensity: 0.9, period: 29, delay: 6,
    flicker: 0.41, glow: [0.34, 0.22], bolt: { length: 0.92, roughness: 0.62, branches: 3 },
    floor: { x0: 0.34, y0: 0.6, x1: 0.92, y1: 0.95 } },
  { name: "far-left", sx: 0.27, sy: 0.07, sh: 0.22, intensity: 0.55, period: 43, delay: 19,
    flicker: 0.37, glow: [0.26, 0.17], bolt: { length: 0.8, roughness: 0.7, branches: 2 },
    floor: { x0: 0.12, y0: 0.62, x1: 0.44, y1: 0.86 } },
  { name: "far-right", sx: 0.84, sy: 0.1, sh: 0.17, intensity: 0.45, period: 61, delay: 41,
    flicker: 0.33, glow: [0.2, 0.14], bolt: { length: 0.75, roughness: 0.75, branches: 1 },
    floor: { x0: 0.7, y0: 0.62, x1: 0.94, y1: 0.8 } },
  // Sheet lightning: light inside the clouds with no visible channel.
  { name: "in-cloud", sx: 0.46, sy: 0.2, sh: 0.12, intensity: 0.5, period: 53, delay: 33,
    flicker: 0.29, glow: [0.42, 0.18], bolt: null,
    floor: { x0: 0.22, y0: 0.6, x1: 0.74, y1: 0.92 } },
];

const wetStyle = ({ x0, y0, x1, y1 }) => ({ "--x0": x0, "--y0": y0, "--x1": x1, "--y1": y1 });

const SKY = STRIKES.map(strike => ({
  ...strike,
  style: {
    "--sx": strike.sx,
    "--sy": strike.sy,
    "--sh": strike.sh,
    "--intensity": strike.intensity,
    "--period": `${strike.period}s`,
    "--delay": `${strike.delay}s`,
    "--flicker": `${strike.flicker}s`,
    "--glow-w": strike.glow[0],
    "--glow-h": strike.glow[1],
  },
  bolt: strike.bolt ? makeBolt(strike.bolt) : null,
  floorStyle: wetStyle(strike.floor),
}));

function Bolt({ id, bolt }) {
  return (
    <svg className="arena-strike-bolt" viewBox="0 0 100 300" preserveAspectRatio="xMidYMin meet" overflow="visible">
      <defs>
        {/* The channel fades out as it falls into the haze. */}
        <linearGradient id={`${id}-fade`} gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="300">
          <stop offset="0" stopColor="#f6f3ff" stopOpacity="1" />
          <stop offset="0.6" stopColor="#ebe5ff" stopOpacity="0.85" />
          <stop offset="1" stopColor="#d9d0ff" stopOpacity="0" />
        </linearGradient>
        <filter id={`${id}-halo`} x="-50%" y="-10%" width="200%" height="120%">
          <feGaussianBlur stdDeviation="1.6" />
        </filter>
      </defs>
      <g fill="none" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke">
        <path d={bolt.main} stroke="#b9a9ff" strokeOpacity="0.32" strokeWidth="3.4" filter={`url(#${id}-halo)`} vectorEffect="non-scaling-stroke" />
        {bolt.forks.map((fork, index) => (
          <path key={index} d={fork} stroke={`url(#${id}-fade)`} strokeOpacity="0.5" strokeWidth="0.6" vectorEffect="non-scaling-stroke" />
        ))}
        <path d={bolt.main} stroke={`url(#${id}-fade)`} strokeWidth="1.05" vectorEffect="non-scaling-stroke" />
      </g>
    </svg>
  );
}

/* ---- fires -------------------------------------------------------------- */

// Every fire: its flame base in the artwork (bx, by: fractions of the
// image), the flame box (w, h: fractions of the image's height), how many
// tongues it has, its own dance and glow clocks, and the character of its
// embers. No two fires share timings, so they never move together.
// reflection: the stretch of wet floor where the art already paints this
// fire's reflection (x0, y0 to x1, y1: fractions of the art). Only the two
// cliff fires have one; the braziers stand on ledges above the floor.
const FIRES = [
  { name: "left-brazier", bx: 0.06, by: 0.418, w: 0.034, h: 0.05, tongues: 1,
    dance: ["a", 0.53, -0.21, "normal"], glow: ["a", 2.9, -0.7],
    embers: { count: 7, size: [0.8, 1.7], rise: [0.09, 0.17], drift: 22, duration: [5.2, 9.4], spread: 6 } },
  { name: "right-brazier", bx: 0.965, by: 0.47, w: 0.034, h: 0.055, tongues: 1,
    dance: ["b", 0.71, -0.44, "normal"], glow: ["b", 3.7, -1.9],
    embers: { count: 6, size: [0.8, 1.8], rise: [0.08, 0.16], drift: 26, duration: [5.8, 10.6], spread: 6 } },
  { name: "left-cliff", bx: 0.048, by: 0.62, w: 0.055, h: 0.055, tongues: 2,
    dance: ["a", 0.47, -0.13, "reverse"], glow: ["b", 4.3, -2.6],
    reflection: { x0: 0.045, y0: 0.628, x1: 0.112, y1: 0.705 },
    embers: { count: 8, size: [0.7, 1.6], rise: [0.06, 0.13], drift: 34, duration: [4.8, 9], spread: 16 } },
  { name: "right-cliff", bx: 0.96, by: 0.655, w: 0.055, h: 0.055, tongues: 2,
    dance: ["b", 0.79, -0.37, "alternate"], glow: ["a", 5.3, -3.1],
    reflection: { x0: 0.918, y0: 0.66, x1: 0.985, y1: 0.728 },
    embers: { count: 6, size: [0.7, 1.5], rise: [0.05, 0.12], drift: 30, duration: [6.1, 11.2], spread: 14 } },
  { name: "right-torch", bx: 0.867, by: 0.232, w: 0.018, h: 0.03, tongues: 1,
    dance: ["a", 0.41, -0.09, "alternate"], glow: ["b", 3.1, -0.5],
    embers: { count: 4, size: [0.6, 1.2], rise: [0.04, 0.09], drift: 14, duration: [5.5, 9.8], spread: 3 } },
  { name: "left-torch", bx: 0.142, by: 0.205, w: 0.018, h: 0.03, tongues: 1,
    dance: ["b", 0.61, -0.27, "reverse"], glow: ["a", 4.7, -1.2],
    embers: { count: 3, size: [0.6, 1.2], rise: [0.04, 0.08], drift: 12, duration: [6.4, 10.2], spread: 3 } },
];

// Warm oranges with the odd pale-gold spark; no deep reds.
const COLORS = ["#ffb35a", "#ffa347", "#ff9a3d", "#ffbe6e", "#ffc980", "#ff9030"];

const FIRE_SCENE = FIRES.map(fire => ({
  ...fire,
  style: {
    "--bx": fire.bx,
    "--by": fire.by,
    "--fw": fire.w,
    "--fh": fire.h,
    "--dance-name": `arena-dance-${fire.dance[0]}`,
    "--dance-duration": `${fire.dance[1]}s`,
    "--dance-delay": `${fire.dance[2]}s`,
    "--dance-direction": fire.dance[3],
    "--glow-name": `arena-flame-${fire.glow[0]}`,
    "--glow-duration": `${fire.glow[1]}s`,
    "--glow-delay": `${fire.glow[2]}s`,
  },
  reflectionStyle: fire.reflection ? wetStyle(fire.reflection) : null,
  embers: Array.from({ length: fire.embers.count }, () => ({
    "--x": `${between([-fire.embers.spread, fire.embers.spread]).toFixed(1)}px`,
    "--size": `${between(fire.embers.size).toFixed(2)}px`,
    "--color": COLORS[Math.floor(random() * COLORS.length)],
    "--peak": between([0.4, 0.78]).toFixed(2),
    "--rise": between(fire.embers.rise).toFixed(3),
    "--drift": `${between([-fire.embers.drift, fire.embers.drift]).toFixed(1)}px`,
    "--wobble": `${between([2, 7]).toFixed(1)}px`,
    "--duration": `${between(fire.embers.duration).toFixed(2)}s`,
    "--delay": `${(-between([0, 10])).toFixed(2)}s`,
  })),
}));

/* ---- ambient motes ------------------------------------------------------ */

// Tiny lights in the air: pale lavender, gold and cool blue, very faint.
// Each rises at its own speed (6-26 s per climb), fading as it goes, and
// twinkles on its own short beat. Positions are in % of the viewport and
// favour the edges: most motes that land on the board's middle are moved
// out, so the cards stay clear.
const MOTE_COLORS = ["#e9e1ff", "#efe6ff", "#ffe9c2", "#fff0d6", "#d7ebff"];
const MOTE_COUNT = 30;

function motePosition() {
  for (;;) {
    const x = between([1, 99]);
    const y = between([6, 96]);
    // Normalised distance from the board's centre (an ellipse over the cards).
    const inner = ((x - 50) / 24) ** 2 + ((y - 50) / 32) ** 2;
    if (inner > 1 || random() < 0.18) return [x, y];
  }
}

// Durations spread on a log scale: a few quick climbs, many medium, some slow.
const climb = () => Math.exp(between([Math.log(6), Math.log(26)]));

const MOTES = Array.from({ length: MOTE_COUNT }, () => {
  const [x, y] = motePosition();
  const duration = climb();
  return {
    "--mx": `${x.toFixed(1)}%`,
    "--my": `${y.toFixed(1)}%`,
    "--ms": `${between([1.4, 3]).toFixed(2)}px`,
    "--mc": MOTE_COLORS[Math.floor(random() * MOTE_COLORS.length)],
    "--mp": between([0.18, 0.55]).toFixed(2),
    "--mdx1": `${between([-28, 28]).toFixed(0)}px`,
    "--mdx2": `${between([-55, 55]).toFixed(0)}px`,
    "--mdy": `${between([-260, -70]).toFixed(0)}px`,
    "--md": `${duration.toFixed(1)}s`,
    "--mdelay": `${(-between([0, duration])).toFixed(1)}s`,
    "--mt": `${between([0.5, 2.2]).toFixed(2)}s`,
    "--mtdelay": `${(-between([0, 2])).toFixed(2)}s`,
  };
});

export default function ArenaAmbience() {
  const [appearance] = useBattlefieldAppearance();
  if (appearance.compactCards) return null;
  return (
    <div className="arena-ambience" aria-hidden="true">
      {MOTES.map((style, index) => (
        <span key={`mote-${index}`} className="arena-mote" style={style} />
      ))}
      {SKY.map(strike => (
        <div key={strike.name} className="arena-strike" style={strike.style}>
          <span className="arena-strike-glow" />
          {strike.bolt ? <Bolt id={`arena-${strike.name}`} bolt={strike.bolt} /> : null}
          <span className="arena-wet arena-wet--strike" style={strike.floorStyle} />
        </div>
      ))}
      {FIRE_SCENE.map(fire => (
        <div key={fire.name} className="arena-fire" data-tongues={fire.tongues} style={fire.style}>
          <span className="arena-flame" />
          {fire.reflectionStyle ? <span className="arena-wet arena-wet--fire" style={fire.reflectionStyle} /> : null}
          {fire.embers.map((style, index) => (
            <span key={index} className="arena-ember" style={style} />
          ))}
        </div>
      ))}
    </div>
  );
}
