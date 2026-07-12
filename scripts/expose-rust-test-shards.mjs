import fs from "node:fs";
import path from "node:path";

for (const directory of process.argv.slice(2)) {
  for (const name of fs.readdirSync(directory).filter((entry) => /^shard_\d+\.rs$/.test(entry))) {
    const file = path.join(directory, name);
    const lines = fs.readFileSync(file, "utf8").split("\n");
    let inStruct = false;
    let inInherentImpl = false;
    const output = lines.map((line, index) => {
      if (index === 0 && line !== "#![allow(unused_imports)]") {
        // The attribute is inserted below so the existing first line is retained.
      }
      if (/^pub\(super\) struct\s/.test(line) && line.includes("{")) inStruct = true;
      if (/^impl(?:<[^>]+>)?\s/.test(line) && !line.includes(" for ")) inInherentImpl = true;

      let next = line;
      if (inStruct && /^    [A-Za-z_][A-Za-z0-9_]*\s*:/.test(line)) {
        next = line.replace(/^    /, "    pub(super) ");
      }
      if (inInherentImpl && /^    (?:async )?fn\s/.test(line)) {
        next = line.replace(/^    /, "    pub(super) ");
      }

      if (line === "}") {
        inStruct = false;
        inInherentImpl = false;
      }
      return next;
    });
    if (output[0] !== "#![allow(unused_imports)]") output.unshift("#![allow(unused_imports)]");
    fs.writeFileSync(file, output.join("\n"));
  }
}
