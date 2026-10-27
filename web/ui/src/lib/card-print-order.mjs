// Printed-card conventions, not game rules. Sources and examples:
// https://magic.wizards.com/en/news/making-magic/split-odds-ends-2006-04-17
// Modern wedges (Jodah URW, Muldrotha BGU):
// https://magic.wizards.com/en/news/feature/dominaria-release-notes
// Type lines: Bident of Thassa (Enchantment Artifact), Dryad Arbor (Land
// Creature), and Lorwyn Eclipsed's Firdoch Core (Kindred Artifact).
// https://magic.wizards.com/en/news/feature/release-notes-2013-09-16
// https://magic.wizards.com/en/news/feature/lorwyn-eclipsed-release-notes
// https://media.wizards.com/2021/downloads/TSR_Release_Notes/EN_MTGTSR_FAQ_20210118.pdf
const wheel = 'WUBRG';
const wedges = {BGW:'WBG', RUW:'URW', BGU:'BGU', GRU:'GUR', BRW:'RWB'};

export function printedColorOrder(colors) {
  const present = [...wheel].filter(color => colors.includes(color));
  if (present.length === 3) {
    const wedge = wedges[[...present].sort().join('')];
    if (wedge) return [...wedge];
  }
  if (present.length < 2 || present.length === 5) return present;
  // Break the circle at the largest absent arc: GW, RW, GWU, RGWU, etc.
  let start = 0, largest = 0;
  present.forEach((color, i) => {
    const gap = (wheel.indexOf(color) - wheel.indexOf(present[(i + present.length - 1) % present.length]) + 5) % 5;
    if (gap > largest) {largest = gap; start = i;}
  });
  return [...present.slice(start), ...present.slice(0, start)];
}

function canonicalSymbol(symbol) {
  const parts = symbol.toUpperCase().split('/');
  const colors = parts.filter(part => wheel.includes(part) && part.length === 1);
  if (colors.length === 2) {
    const other = parts.filter(part => !colors.includes(part));
    return [...other.filter(part => part !== 'P'), ...printedColorOrder(colors), ...other.filter(part => part === 'P')].join('/');
  }
  if (colors.length === 1 && parts.length > 1) {
    return [...parts.filter(part => part !== colors[0] && part !== 'P'), colors[0], ...parts.filter(part => part === 'P')].join('/');
  }
  return symbol.toUpperCase();
}

export function orderedManaSymbols(symbols) {
  const normalized = symbols.map(canonicalSymbol);
  const order = printedColorOrder(normalized.flatMap(symbol => symbol.split('/').filter(part => /^[WUBRG]$/.test(part))));
  const rank = symbol => {
    if (/^[XYZ]$/.test(symbol)) return [-4, 'XYZ'.indexOf(symbol), 0];
    if (/^\d+$/.test(symbol)) return [-3, Number(symbol), 0];
    if (symbol === 'C') return [-2, 0, 0];
    if (symbol === 'S') return [-1, 0, 0];
    const colors = symbol.split('/').filter(part => /^[WUBRG]$/.test(part));
    if (!colors.length) return [2, 0, 0];
    // Hybrid symbols straddle their two colors (Thopter Foundry: W/B, U;
    // Ognis: B/R, R, R/G). A Phyrexian symbol follows its ordinary color.
    const position = colors.reduce((sum, color) => sum + order.indexOf(color), 0) / colors.length;
    return [0, position, colors.length === 2 ? -1 : symbol.includes('/') ? 1 : 0];
  };
  return normalized.sort((a, b) => {
    const x = rank(a), y = rank(b);
    return x[0] - y[0] || x[1] - y[1] || x[2] - y[2] || a.localeCompare(b);
  });
}

export function printedManaCost(cost) {
  if (cost == null) return cost;
  const text = String(cost);
  // Only a complete cost: never reorder symbols across prose or split faces.
  if (!/^\s*(?:\{[^{}]+\}\s*)+$/.test(text)) return text;
  return orderedManaSymbols([...text.matchAll(/\{([^{}]+)\}/g)].map(match => match[1])).map(symbol => `{${symbol}}`).join('');
}

const typeOrder = ['Token','Basic','Legendary','Snow','World','Ongoing','Kindred','Tribal',
  'Enchantment','Artifact','Land','Creature','Battle','Planeswalker','Instant','Sorcery',
  'Conspiracy','Dungeon','Phenomenon','Plane','Scheme','Vanguard'];
const typeRank = new Map(typeOrder.map((type, index) => [type.toLowerCase(), index]));
export function printedTypeLine(value) {
  if (value == null) return value;
  return String(value).split(/\s*\/\/\s*/).map(line => {
    const match = /^(.*?)(\s+[—–-]\s+.*)?$/.exec(line.trim());
    const words = match[1].split(/\s+/);
    // Unknown/custom/localized text stays intact; subtype order is significant.
    if (words.some(word => !typeRank.has(word.toLowerCase()))) return line;
    return words.sort((a,b) => typeRank.get(a.toLowerCase()) - typeRank.get(b.toLowerCase())).join(' ') + (match[2] ?? '');
  }).join(' // ');
}

export function printableCardOrder(card) {
  const result = {...card};
  for (const key of ['mana_cost','display_mana_cost']) if (result[key] != null) result[key] = printedManaCost(result[key]);
  for (const key of ['type_line','type_line_display']) if (result[key] != null) result[key] = printedTypeLine(result[key]);
  if (Array.isArray(result.card_faces)) result.card_faces = result.card_faces.map(printableCardOrder);
  return result;
}
