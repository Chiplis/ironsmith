import test from 'node:test';
import assert from 'node:assert/strict';
import { createTablePerspective } from '../src/components/board/forge/table-perspective.js';
for (const [width,height] of [[1440,900],[844,390],[2560,1440]]) {
 test(`perspective screen anchors round-trip at ${width}x${height}`,()=>{
  const view=createTablePerspective();view.resize(width,height);
  assert.equal(view.camera.isPerspectiveCamera,true);
  for (const z of [-60,-48,-22,10]) for(const [x,y] of [[0,0],[width,height],[width/2,height/2],[width*.1,height*.8]]) {
   const p=view.point(x,y,z).project(view.camera);
   assert.ok(Math.abs((p.x+1)*width/2-x)<1e-6);
   assert.ok(Math.abs((1-p.y)*height/2-y)<1e-6);
  }
  const far=view.point(101,100).distanceTo(view.point(100,100));
  const near=view.point(101,height-100).distanceTo(view.point(100,height-100));
  assert.ok(far>near,'farther objects recede in perspective');
 });
}
