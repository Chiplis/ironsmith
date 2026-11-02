import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { discoverRulesUrl } from '../scripts/generate-mtg-keyword-rules.mjs';
test('discovers the current official TXT URL and rejects missing or ambiguous downloads', () => {
  const url = 'https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt';
  assert.equal(discoverRulesUrl(`<a href="${url}">TXT</a><a href="${url}">TXT</a>`), url);
  assert.equal(discoverRulesUrl(`<a href="${url.replace("%20", " ")}">TXT</a>`), url);
  assert.throws(() => discoverRulesUrl('<html>Service unavailable</html>'), /found 0/);
  assert.throws(() => discoverRulesUrl(`<a href="${url}"></a><a href="https://media.wizards.com/2027/MagicCompRules.txt"></a>`), /found 2/);
  assert.throws(() => discoverRulesUrl('https://example.com/MagicCompRules.txt'), /found 0/);
});
test('invalid downloaded rules cannot overwrite the existing registry', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'keyword-refresh-'));
  try {
    const input = join(dir, 'rules.txt'), output = join(dir, 'registry.js');
    await writeFile(input, '<html>Unavailable</html>');
    await writeFile(output, 'existing registry');
    const result = spawnSync(process.execPath, ['web/ui/scripts/generate-mtg-keyword-rules.mjs', '--input', input, '--output', output], {encoding:'utf8'});
    assert.notEqual(result.status, 0);
    assert.equal(await readFile(output, 'utf8'), 'existing registry');
  } finally { await rm(dir, {recursive:true, force:true}); }
});
test('refresh handles whitespace-only separators and records the actual source', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'keyword-refresh-'));
  try {
    const lines = ['Magic: The Gathering Comprehensive Rules', 'These rules are effective as of September 25, 2026.', ...Array(1001).fill(''), '701. Keyword Actions'];
    for (let i = 2; i <= 51; i++) lines.push(`701.${i}. ${i === 2 ? 'Recruit' : `Action ${i}`}`, `701.${i}a Draw a card.`);
    lines.push('702. Keyword Abilities');
    for (let i = 2; i <= 151; i++) lines.push(`702.${i}. Ability ${i}`, `702.${i}a This is a keyword ability.`);
    lines.push('703. Turn-Based Actions', 'Glossary', '\u00a0', 'Recruit', 'A keyword action. See rule 701.2.', '\u00a0', 'Credits');
    const input = join(dir, 'rules.txt'), output = join(dir, 'registry.mjs');
    const url = 'https://media.wizards.com/test/MagicCompRules.txt';
    await writeFile(input, lines.join('\n'));
    const result = spawnSync(process.execPath, ['web/ui/scripts/generate-mtg-keyword-rules.mjs', '--input', input, '--url', url, '--output', output], {encoding:'utf8'});
    assert.equal(result.status, 0, result.stderr);
    const generated = await import(`file://${output}`);
    assert.equal(generated.MTG_KEYWORD_RULESET_SOURCE.url, url);
    assert.equal(generated.MTG_KEYWORD_RULESET_SOURCE.effectiveDate, 'September 25, 2026');
    assert.equal(generated.MTG_KEYWORD_RULES.length, 200);
    assert.equal(generated.MTG_KEYWORD_RULES[0].title, 'Recruit');
    // Moving a keyword to another rule number must not inherit that number's old aliases.
    assert.deepEqual(generated.MTG_KEYWORD_RULES[0].aliases, ['recruit']);
  } finally { await rm(dir, {recursive:true, force:true}); }
});
