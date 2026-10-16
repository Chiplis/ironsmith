import test from 'node:test';
import assert from 'node:assert/strict';
import {zonePreviewLayout} from '../src/lib/zone-preview-layout.js';

test('previews avoid decisions and zones across viewport sizes and dock positions', () => {
  for (const [viewportWidth, viewportHeight] of [[1440,900], [1280,720], [600,400]]) {
    for (const zoneTop of [50, viewportHeight / 2, viewportHeight - 130]) {
      const anchor = {left:viewportWidth / 2, right:viewportWidth - 8, top:zoneTop, bottom:zoneTop + 90};
      for (const dockTop of [30, viewportHeight / 2, viewportHeight - 100]) {
        const obstacle = {left:viewportWidth - 300, right:viewportWidth - 8, top:dockTop, bottom:dockTop + 90};
        const p = zonePreviewLayout({anchor, viewportWidth, viewportHeight, obstacles:[obstacle]});
        assert.notEqual(p.visibility, 'hidden');
        const r = {left:p.left, top:p.top, right:p.left + parseFloat(p.width), bottom:p.top + parseFloat(p.height)};
        assert.ok(r.left >= 8 && r.right <= viewportWidth - 8 + 0.001);
        assert.ok(r.top >= 8 && r.bottom <= viewportHeight - 8);
        assert.ok(r.bottom <= anchor.top - 8 || r.top >= anchor.bottom + 8);
        assert.ok(r.right <= obstacle.left - 8 || r.left >= obstacle.right + 8 || r.bottom <= obstacle.top - 8 || r.top >= obstacle.bottom + 8);
      }
    }
  }
});
test('no free space hides the preview instead of covering controls', () => {
  assert.deepEqual(zonePreviewLayout({anchor:{left:0,right:600,top:180,bottom:220},viewportWidth:600,viewportHeight:400,obstacles:[{left:0,right:600,top:0,bottom:400}]}), {visibility:'hidden'});
});
