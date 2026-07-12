import fs from "node:fs";
import path from "node:path";

const [sourcePath, targetDirectory, requestedSize = "3600"] = process.argv.slice(2);
if (!sourcePath || !targetDirectory) {
  throw new Error("usage: shard-rust-tests.mjs SOURCE TARGET_DIRECTORY [TARGET_LINES]");
}

const targetSize = Number(requestedSize);
const lines = fs.readFileSync(sourcePath, "utf8").split("\n");
const firstTest = lines.findIndex((line) => line === "#[test]" || line.trim() === "#[test]");
if (firstTest < 0) throw new Error(`${sourcePath}: no test attribute found`);

function attributeStart(index) {
  let start = index;
  while (start > 0 && lines[start - 1].trimStart().startsWith("#[")) start -= 1;
  return start;
}

const starts = [];
let nextTarget = firstTest + targetSize;
for (let index = firstTest + 1; index < lines.length; index += 1) {
  if (index < nextTarget || lines[index].trim() !== "#[test]") continue;
  const start = attributeStart(index);
  if (start > firstTest && starts.at(-1) !== start) {
    starts.push(start);
    nextTarget = start + targetSize;
  }
}

const boundaries = [firstTest, ...starts, lines.length];
const chunks = [];
for (let index = 0; index < boundaries.length - 1; index += 1) {
  chunks.push(lines.slice(boundaries[index], boundaries[index + 1]));
}

function exposeTopLevelItems(chunk) {
  return chunk.map((line) => {
    if (/^(?:async )?fn\s/.test(line)) return `pub(super) ${line}`;
    if (/^(?:struct|enum|const|static|type|trait)\s/.test(line)) return `pub(super) ${line}`;
    return line;
  });
}

fs.mkdirSync(targetDirectory, { recursive: true });
const moduleNames = chunks.map((_, index) => `shard_${String(index).padStart(2, "0")}`);
const header = lines.slice(0, firstTest);
const declarations = moduleNames.flatMap((name) => [`mod ${name};`]);
fs.writeFileSync(path.join(targetDirectory, "mod.rs"), [...header, ...declarations, ""].join("\n"));

for (let index = 0; index < chunks.length; index += 1) {
  const imports = ["use super::*;"];
  for (let sibling = 0; sibling < moduleNames.length; sibling += 1) {
    if (sibling !== index) imports.push(`use super::${moduleNames[sibling]}::*;`);
  }
  const output = [...imports, "", ...exposeTopLevelItems(chunks[index]), ""].join("\n");
  fs.writeFileSync(path.join(targetDirectory, `${moduleNames[index]}.rs`), output);
}

fs.unlinkSync(sourcePath);
console.log(`${sourcePath}: ${chunks.length} shards in ${targetDirectory}`);
