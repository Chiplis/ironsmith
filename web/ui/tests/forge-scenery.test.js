import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import { placeForgeScenery } from '../src/components/board/forge/forge-scenery.js';

test('imported scenery occupies free edge pockets and retreats from crowded zones', () => {
  const obstacles = [{ left: 100, right: 1150, top: 100, bottom: 650 }, { left: 0, right: 300, top: 0, bottom: 80 }];
  const props = placeForgeScenery(1280, 800, obstacles);
  assert.equal(props.length, 6);
  assert.equal(props.filter(p => p.kind === 'pit').length, 2);
  for (const prop of props) for (const r of obstacles) {
    if (!prop.size) continue;
    const distance = Math.hypot(Math.max(r.left - prop.x, 0, prop.x - r.right), Math.max(r.top - prop.y, 0, prop.y - r.bottom));
    assert.ok(distance >= prop.size / 2 + 11.9);
  }
  const crowded = placeForgeScenery(844, 390, [{ left: 0, right: 844, top: 0, bottom: 390 }]);
  assert.ok(crowded.every(p => p.size === 0));
});

test('the glTF models ship with every dependency locally and within the asset budget', async () => {
  let bytes = 0;
  for (const name of ['rock_face_02', 'stone_fire_pit']) {
    const path = new URL(`../public/theme/forge-arena/${name}/${name}.gltf`, import.meta.url);
    const model = JSON.parse(await readFile(path, 'utf8'));
    assert.ok(model.meshes.length > 0);
    for (const item of [...model.buffers, ...model.images]) {
      assert.ok(!/^(https?:|data:|\/)|\.\./.test(item.uri), 'assets must be bundled, not hotlinked');
      const file = new URL(item.uri, path);
      const size = (await stat(file)).size;
      assert.ok(size > 0);
      bytes += size;
    }
  }
  assert.ok(bytes < 7 * 1024 * 1024, `Model assets exceed budget: ${bytes}`);
});
