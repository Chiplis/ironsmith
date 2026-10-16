// Build the upstream browser library only. Its full build also generates
// declaration files and the showcase site, neither consumed by Ironsmith.
import path from 'node:path';
import {cp} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
const source = process.cwd();
const {build} = await import(pathToFileURL(path.join(source, 'node_modules/vite/dist/node/index.js')).href);
const result = await build({
  configFile:false,
  resolve:{alias:{'@card-glint/browser':path.join(source,'packages/browser/src/index.ts')}},
  build:{
    target:'es2022',outDir:'dist/lib',emptyOutDir:true,
    lib:{entry:{browser:'packages/browser/src/index.ts',magic:'games/magic/src/index.ts',
      'magic-presentation':'games/magic/src/presentation.ts',kernels:'packages/browser/src/kernels.ts'},
    formats:['es'],fileName:(_,name)=>`${name}.js`},
    cssCodeSplit:true,assetsInlineLimit:0,
  },
});
const presentation = [result].flat().flatMap(bundle => bundle.output).find(chunk => chunk.type === 'chunk' && chunk.name === 'magic-presentation');
for (const name of ['loadCanvasImages','composeCardTexture','ordinaryTextLayout','retroTextLayout','planeswalkerRuleRows','faceFrame','hasExtraRules','isRetro','rulesFont']) {
  if (!presentation?.exports.includes(name)) throw new Error(`card-glint no longer exports required API: ${name}`);
}
for (const name of ['renderer','pixels']) await cp(`packages/browser/generated/${name}_bg.wasm`,`dist/lib/${name}_bg.wasm`);
await cp('games/magic/fonts.css','dist/lib/magic.css');
