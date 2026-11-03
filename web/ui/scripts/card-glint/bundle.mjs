// Build the upstream browser library only. Its full build also generates
// declaration files and the showcase site, neither consumed by Ironsmith.
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {cp, mkdir, readFile, writeFile} from 'node:fs/promises';
import {fileURLToPath, pathToFileURL} from 'node:url';
const source = process.cwd();
const hostScripts = path.dirname(fileURLToPath(import.meta.url));
const presentationSource = path.join(source,'games/magic/src/presentation');
const materialSource = await readFile(path.join(presentationSource,'early-hsb-material.ts'),'utf8');
const materialStart = materialSource.indexOf('async function buildMaterial(');
const materialEnd = materialSource.indexOf('/** Whole-material helper', materialStart);
if (materialStart < 0 || materialEnd < 0) throw new Error('card-glint material worker patch needs updating');
const materialFunction = materialSource.slice(materialStart,materialEnd);
// Use the upstream implementation in both execution contexts. Do not maintain a
// second copy of its pixel/mask algorithms or change derived-material cache keys.
const generated = path.join(source,'.ironsmith/material-raster.ts');
await mkdir(path.dirname(generated),{recursive:true});
await writeFile(generated, [
  `import { rasterCanvas } from ${JSON.stringify(path.join(source,'packages/browser/src/canvas-raster.ts'))};`,
  ...['outsetFrameMask:frame-material-mask','blendFrameMaterial:frame-material-blend','colorFrame:frame-hsb','recolorManaInkPixels:mana-ink-material']
    .map(entry => { const [name,file] = entry.split(':'); return `import { ${name} } from ${JSON.stringify(path.join(presentationSource,`${file}.ts`))};`; }),
  `export ${materialFunction}`,
].join('\n'));
// Match upstream's build prerequisite: fresh checkouts omit generated set data,
// including the eager module imported by the Magic presentation entry point.
execFileSync(process.execPath, ['games/magic/tools/build-set-data.mjs'], {cwd: source, stdio: 'inherit'});
const {build} = await import(pathToFileURL(path.join(source, 'node_modules/vite/dist/node/index.js')).href);
const result = await build({
  configFile:false,
  base:'./',
  plugins:[{
    name:'ironsmith-live-land-palette',
    enforce:'pre',
    transform(code,id) {
      // The component painter and the retained modern plate share a nameplate.
      // Fit the larger title to the available width and center the mana discs.
      const nameplateEdits = {
        'face-layout.ts': [['normalTitleFont: 26,', 'normalTitleFont: 27,']],
        'canvas-face.ts': [
          ["ty + th / 2 + (kind === 'normal' ? 1 : 0)", "kind === 'normal' && !panel ? {center: ty + th / 2} : ty + th / 2 + (kind === 'normal' ? 1 : 0)"],
          ['ty + th / 2 - manaSize / 2 + FACE_TEXT.manaLift / 2', "ty + th / 2 - manaSize / 2 + (kind === 'normal' && !panel ? 0 : FACE_TEXT.manaLift / 2)"],
        ],
        'canvas-text.ts': [
          ['export function paintFittedText(\n  context: CanvasContext,\n  text: string,\n  x: number,\n  y: number,',
            'export function paintFittedText(\n  context: CanvasContext,\n  text: string,\n  x: number,\n  anchor: number | {center: number},'],
          ['  paintTextInk(\n', `  let y: number;
  if (typeof anchor === 'number') y = anchor;
  else {
    const ink = measureFontText(context, text);
    y = anchor.center + (ink.actualBoundingBoxAscent - ink.actualBoundingBoxDescent) / 2;
  }
  paintTextInk(\n`],
        ],
        'frame-plates.ts': [
          ['?? 29.2) : 26,', '?? 29.2) : retained ? FACE_TEXT.normalTitleFont : 26,'],
          ['FACE_RECTS.title[3] / 2 + FACE_TEXT.manaLift / (2 * 680)', 'FACE_RECTS.title[3] / 2'],
        ],
        'frame-plate-paint.ts': [['ty + th / 2 + (retained ? 1 : 0)', 'retained ? {center: ty + th / 2} : ty + th / 2']],
      };
      const edits = id.includes('/presentation/') && nameplateEdits[path.basename(id)];
      if (edits) {
        for (const [before, after] of edits) {
          if (!code.includes(before)) throw new Error(`card-glint nameplate patch needs updating in ${id}`);
          code = code.replaceAll(before, after);
        }
        return code;
      }
      if (id.endsWith('/presentation/assets.ts')) {
        // Use the displayed printing's icon identity independently of its frame
        // and footer. An explicit style may select a different set altogether.
        const before = "set === 'chr' ? (chroniclesSymbol(face.name) ?? '') : set,";
        const end = '      printingSetSymbolSource(face),\n    )';
        if (!code.includes(before) || !code.includes(end)) throw new Error('card-glint set symbol patch needs updating');
        return code.replace(before, "set === 'chr' ? (chroniclesSymbol(face.name) ?? '') : (face.printingSymbol?.set === set ? face.printingSymbol.code ?? set : set),")
          .replace(end, end + " ?? (face.printingSymbol?.set === set && set !== 'chr' ? face.printingSymbol.url : null)");
      }
      if (id.endsWith('/presentation/prepare-frame.ts')) {
        // Prepared has no dedicated colorless stock. Use its neutral artifact
        // plate for colorless faces, without changing their types or colors.
        const before = "  throw Error('The pinned Prepared source has no colorless plate.');";
        if (!code.includes(before)) throw new Error('card-glint neutral Prepared plate patch needs updating');
        return code.replace(before, "  return 'a';");
      }
      if (id.endsWith('/packages/browser/src/gpu/device.ts')) {
        // A WebGL context can exist even when Chrome is rasterizing on the CPU.
        // Fine print/foil shaders can then monopolize the compositor for tens
        // of seconds. Let the existing Canvas2D fallback handle those devices.
        const before = '      antialias: true,';
        if (!code.includes(before)) throw new Error('card-glint GPU capability patch needs updating');
        return code.replace(before, `${before}\n      failIfMajorPerformanceCaveat: true,`);
      }
      if (id.endsWith('/presentation/early-hsb-material.ts')) {
        if (!code.includes(materialFunction)) throw new Error('card-glint material worker patch needs updating');
        return `import { buildMaterialOffThread as buildMaterial } from ${JSON.stringify(path.join(hostScripts,'material-worker-client.mjs'))};\n${code.replace(materialFunction,'')}`;
      }
      if (id.endsWith('/packages/browser/src/canvas-images.ts')) {
        // Image.decode has already produced a bounded SVG image. Copying it
        // through a CPU Canvas2D surface adds a synchronous raster copy.
        // Let the browser create the bitmap directly,
        // as upstream's AssetStore does, without a second raster/readback.
        const start = "  const raster = document.createElement('canvas');\n";
        const raster = `    raster.width = image.naturalWidth;
    raster.height = image.naturalHeight;
    const context = raster.getContext('2d', { willReadFrequently: true });
    if (!context || !raster.width || !raster.height)
      throw new Error('SVG asset could not be rasterized.');
    context.drawImage(image, 0, 0);
    return await createImageBitmap(raster);`;
        const cleanup = '    raster.width = 0;\n    raster.height = 0;\n';
        for (const needle of [start,raster,cleanup]) {
          if (!code.includes(needle)) throw new Error('card-glint SVG bitmap patch needs updating');
        }
        return code.replace(start,'').replace(raster,'    return await createImageBitmap(image);').replace(cleanup,'');
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
  resolve:{alias:{'@card-glint/browser':path.join(source,'packages/browser/src/index.ts'),
    '@ironsmith/card-glint-material':generated}},
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
