#!/usr/bin/env node
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { REPO_ROOT } from './local-table.mjs';

const args = process.argv.slice(2);
const compactIndex = args.indexOf('--compact');
const compact = compactIndex >= 0;
if (compact) args.splice(compactIndex, 1);

function compactObservation(value) {
  if (!value || typeof value !== 'object') return value;
  if (value.observation) return { ...value, observation: compactObservation(value.observation) };
  if (!value.observationId || !Array.isArray(value.controls)) return value;
  const { cards: _cards, controls, ...observation } = value;
  if (observation.url?.length > 500) {
    const url = new URL(observation.url);
    url.searchParams.delete('deck');
    observation.url = url.href;
  }
  observation.controls = controls.filter(control => !control.blocked).map(control => {
    const output = {};
    for (const field of ['ref', 'name', 'role', 'tag', 'type', 'value', 'disabled', 'readonly', 'checked', 'options', 'attributes', 'className', 'rect']) {
      const entry = control[field];
      if (entry === undefined || entry === '' || (Array.isArray(entry) && !entry.length)
        || (entry && typeof entry === 'object' && !Array.isArray(entry) && !Object.keys(entry).length)) continue;
      output[field] = entry;
    }
    const zone = {};
    for (const ancestor of control.ancestry || []) {
      const attributes = ancestor.attributes || {};
      if (attributes['data-bf-side']) zone.side ||= attributes['data-bf-side'];
      if (attributes['data-card-navigation-scope']) zone.scope ||= attributes['data-card-navigation-scope'];
      if (attributes['data-zone-owner'] !== undefined) zone.owner ??= attributes['data-zone-owner'];
    }
    if (Object.keys(zone).length) output.zone = zone;
    return output;
  });
  return observation;
}
function option(name, fallback) {
  const index = args.indexOf(name);
  if (index < 0) return fallback;
  const value = args[index + 1];
  if (!value) throw new Error(`${name} needs a value`);
  args.splice(index, 2);
  return value;
}
const url = option('--url', process.env.PLAYER_MCP_URL || 'http://127.0.0.1:3847/mcp');
const outputPath = option('--output');
const timeout = Number(option('--timeout', '600000'));
const client = new Client({ name: 'ironsmith-player-cli', version: '0.1.0' });
try {
  await client.connect(new StreamableHTTPClientTransport(new URL(url)));
  const [tool, json = '{}'] = args;
  if (!tool || tool === '--list') console.log(JSON.stringify(await client.listTools(), null, 2));
  else {
    const result = await client.callTool({ name: tool, arguments: JSON.parse(json) }, undefined, { timeout });
    for (const item of result.content || []) {
      if (item.type !== 'image') continue;
      const destination = path.resolve(outputPath || path.join(REPO_ROOT, 'reports/player-mcp', `screenshot-${Date.now()}.png`));
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, Buffer.from(item.data, 'base64'));
      delete item.data;
      item.path = destination;
    }
    if (compact) {
      for (const item of result.content || []) {
        if (item.type !== 'text') continue;
        try { item.text = JSON.stringify(compactObservation(JSON.parse(item.text))); } catch { /* Plain text is already compact. */ }
      }
    }
    let display = result;
    if (compact && result.content?.length === 1 && result.content[0].type === 'text') {
      try {
        display = JSON.parse(result.content[0].text);
        if (result.isError) display = { ...display, isError: true };
      } catch { /* Non-JSON text retains its MCP envelope. */ }
    }
    console.log(JSON.stringify(display, null, 2));
    if (result.isError) process.exitCode = 1;
  }
} catch (error) {
  console.error(error.stack || error);
  process.exitCode = 1;
} finally { await client.close(); }
