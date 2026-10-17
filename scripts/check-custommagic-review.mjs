// Compile the current image transcriptions without editing printed wording.
// This is an incremental audit; publishing is handled separately.
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { initWasmGame } from "./wasm-test-harness.mjs";

export function faceFromTranscription([name, manaCost, typeLine, oracleText, pt, extras = {}]) {
  const [types, subtypes = ""] = typeLine.split(/\s+[—–]\s+/);
  const supertypes = new Set(["Basic", "Legendary", "Snow", "World", "Ongoing"]);
  const words = types.split(/\s+/);
  const [power = null, toughness = null] = pt?.split("/") || [];
  return {name, manaCost: manaCost || null, oracleText, power, toughness,
    supertypes: words.filter(word => supertypes.has(word)),
    cardTypes: words.filter(word => !supertypes.has(word)),
    subtypes: subtypes ? subtypes.split(/\s+/) : [],
    colorIndicator: [], ...extras,
    loyalty: extras.loyalty == null ? null : Number(extras.loyalty),
    defense: extras.defense == null ? null : Number(extras.defense)};
}

const input = new URL("../fixtures/custommagic/visual-review.json.fixture", import.meta.url);
const rows = JSON.parse(await readFile(input, "utf8"));
const latest = new Map(rows.map(row => [row.image, row]));
const {game} = await initWasmGame({pkg: "demo"});
const results = [];
try {
  for (const row of latest.values()) {
    if (row.needsDetail) continue;
    const groups = row.cardGroups || (row.layout ? [{cards: row.cards, layout: row.layout}] : (row.cards || []).map(card => ({cards: [card]})));
    for (const {cards, layout: groupLayout} of groups) {
      const layout = ({prepared: "prepare", modal_dfc: "transform_like"})[groupLayout] || groupLayout || "single";
      const draft = {layout, hasFuse: row.hasFuse || false,
        faces: cards.map(faceFromTranscription)};
      try {
        const preview = game.previewCustomCard(draft);
        if (!preview.canCreate || /\b(?:unsupported|unimplemented)\b/i.test(JSON.stringify(preview))) {
          throw new Error("Incomplete compilation");
        }
        results.push({image: row.image, status: "compiled", draft, preview});
      } catch (error) {
        results.push({image: row.image, status: "rejected", draft, error: String(error?.message || error)});
      }
    }
  }
} finally { game.free(); }
await mkdir(new URL("../reports/custommagic/", import.meta.url), {recursive: true});
await writeFile(new URL("../reports/custommagic/compilation.json", import.meta.url), JSON.stringify(results, null, 2) + "\n");
console.log(`${latest.size} image reviews; ${results.filter(r=>r.status==='compiled').length}/${results.length} candidate cards compile`);
console.log(results.filter(r=>r.status==='compiled').map(r=>`${r.image} ${r.draft.faces[0].name}`).join("\n"));
