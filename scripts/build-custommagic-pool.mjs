// Run check-custommagic-review.mjs first. Only manually reviewed, unchanged
// drafts that pass the shipped WASM compiler are admitted.
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { initWasmGame } from "./wasm-test-harness.mjs";

const read = async path => JSON.parse(await readFile(new URL(path, import.meta.url), "utf8"));
const manifest = await read("../fixtures/custommagic/posts.json.fixture");
const reviews = await read("../fixtures/custommagic/visual-review.json.fixture");
const results = await read("../reports/custommagic/compilation.json");
const byImage = new Map(reviews.map(row => [row.image, row]));
const {game} = await initWasmGame({pkg: "demo"});
const cards = [], audit = [], names = new Set();
let reviewedImages = 0, pendingImages = 0;
try {
  for (const post of manifest.posts) {
    const entry = {post: post.permalink, rank: post.rank, images: []};
    for (let index = 0; index < post.images.length; index++) {
      const image = String(post.rank).padStart(4, "0") + "-" + String(index).padStart(2, "0");
      const row = byImage.get(image);
      if (!row || row.needsDetail) {
        pendingImages++;
        entry.images.push({image, status: "pending_visual_review"});
        continue;
      }
      reviewedImages++;
      const imageAudit = {image, reason: row.reason, linkedTo: row.linkedTo, results: []};
      for (const [candidate, result] of results.filter(r => r.image === image).entries()) {
        const name = result.draft.faces[0].name;
        let status = result.status;
        if (row.token) status = "token";
        if (status !== "compiled") {
          imageAudit.results.push({name, status, error: result.error});
          continue;
        }
        const keys = result.draft.faces.map(face => face.name.trim().toLowerCase());
        if (keys.some(key => names.has(key))) {
          imageAudit.results.push({name, status: "duplicate_name"});
          continue;
        }
        const draft = structuredClone(result.draft);
        for (const face of draft.faces) face.flavorText = post.permalink;
        let preview;
        try {
          if (draft.faces.some(face => face.cardTypes.includes("Creature")
            && (face.power == null || face.toughness == null))) {
            throw new Error("Creature print is missing power or toughness");
          }
          preview = game.previewCustomCard(draft);
          if (!preview.canCreate || /\b(?:unsupported|unimplemented)\b/i.test(JSON.stringify(preview))) {
            throw new Error("Compilation changed for " + name);
          }
          // Verify actual creation, not only the preview gate.
          game.resetEmpty(["Reviewer", "Opponent"], 20);
          game.createCustomCard({draft, playerIndex: 0, zoneName: "hand", skipTriggers: true});
        } catch (error) {
          imageAudit.results.push({name, status: "creation_failed", error: String(error?.message || error)});
          continue;
        }
        keys.forEach(key => names.add(key));
        cards.push({id: post.id + "-" + index + "-" + candidate, draft,
          source: {post: post.permalink, author: post.author, image: post.images[index]}});
        imageAudit.results.push({name, status: "compiled",
          compiledText: preview.faces.map(face => face.compiledText)});
      }
      entry.images.push(imageAudit);
    }
    audit.push(entry);
  }
} finally { game.free(); }
const compilerSha256 = createHash("sha256")
  .update(await readFile(new URL("../web/wasm_demo/pkg/compiler_bg.wasm", import.meta.url))).digest("hex");
const engineSha256 = createHash("sha256")
  .update(await readFile(new URL("../web/wasm_demo/pkg/engine_bg.wasm", import.meta.url))).digest("hex");
const metadata = {
  schemaVersion: 1, subreddit: "custommagic", ranking: manifest.ranking,
  requestedPosts: manifest.requestedPosts, targetCompilingCards: manifest.targetCompilingCards || 1000,
  discoveredPosts: manifest.posts.length,
  reviewedPosts: audit.filter(post => post.images.every(image => image.status !== "pending_visual_review")).length,
  reviewedImages, pendingImages, acceptedCards: cards.length,
  status: cards.length >= (manifest.targetCompilingCards || 1000) ? "target_reached" : "expanding",
  limitation: "The top/all listing ended after 499 posts. Additional cards come from authenticated top/year and new listings; the pool is limited to visually reviewed, compiling, unique-name candidates.",
  fetchedAt: manifest.fetchedAt, compilerSha256, engineSha256,
};
await mkdir(new URL("../web/ui/public/custommagic/", import.meta.url), {recursive: true});
await writeFile(new URL("../reports/custommagic/audit.json", import.meta.url), JSON.stringify({...metadata, audit}, null, 2) + "\n");
await writeFile(new URL("../web/ui/public/custommagic/pool.json", import.meta.url), JSON.stringify({...metadata, cards}, null, 2) + "\n");
console.log(cards.length + " playable cards; " + reviewedImages + " reviewed images from " + audit.length + " posts");
