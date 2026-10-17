// Build the upstream browser library only. Its full build also generates
// declaration files and the showcase site, neither consumed by Ironsmith.
import path from 'node:path';
import {cp} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
const source = process.cwd();
const {build} = await import(pathToFileURL(path.join(source, 'node_modules/vite/dist/node/index.js')).href);
const result = await build({
  configFile:false,
  plugins:[{
    name:'ironsmith-live-land-palette',
    enforce:'pre',
    transform(code,id) {
      // Upstream's land Saga plate currently overrides even explicit live
      // frame colors. Keep the neutral plate only when the palette is empty.
      if (id.endsWith('/presentation/profile.ts')) {
        // Land is a rules characteristic, not a forced tan material. Keep
        // neutral lands neutral; colored land palettes also paint the stock.
        const before = "const boxes = land\n    ? 'Land'\n    : artifact";
        if (!code.includes(before)) throw new Error('card-glint land material patch needs updating');
        return code.replace(before, "const boxes = land && !colors.length\n    ? 'Land'\n    : artifact && !(land && colors.length)");
      }
      if (id.endsWith('/presentation/saga-frame.ts')) {
        return code.replace("if (/\\bLand\\b/.test(face.typeLine ?? '') || face.land)",
          "if (!colors.length && (/\\bLand\\b/.test(face.typeLine ?? '') || face.land))");
      }
      if (id.endsWith('/presentation/saga-colors.ts')) {
        return code.replace("face.layout === 'transform' ||\n    face.land ||\n    /\\bLand\\b/.test(face.typeLine ?? '')", "face.layout === 'transform'");
      }
    },
  }],
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
for (const name of ['loadCanvasImages','composeCardTexture','ordinaryTextLayout','retroTextLayout','planeswalkerRuleRows','sagaRuleLayout','faceFrame','hasExtraRules','isRetro','rulesFont',
  'faceRects','printingStyles','printingStyleOptions','resolvePrintingStyle']) {
  if (!presentation?.exports.includes(name)) throw new Error(`card-glint no longer exports required API: ${name}`);
}
for (const name of ['renderer','pixels']) await cp(`packages/browser/generated/${name}_bg.wasm`,`dist/lib/${name}_bg.wasm`);
await cp('games/magic/fonts.css','dist/lib/magic.css');
