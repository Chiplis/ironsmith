import {CardGlintRenderQueue} from './render-queue.mjs';
import {CardGlintImageCache} from './image-cache.mjs';
let library;
export function loadCardGlint() {
  if (!library) library = (async () => {
    const base = new URL(`./card-glint/${__CARD_GLINT_REVISION__}/`, document.baseURI);
    await new Promise((resolve, reject) => {
      const link = document.createElement('link');
      link.rel = 'stylesheet'; link.href = new URL('magic.css', base).href;
      link.onload = resolve; link.onerror = () => {link.remove(); reject(new Error('card-glint fonts stylesheet failed'));};
      document.head.append(link);
    });
    return import(/* @vite-ignore */ new URL('magic-presentation.js', base).href);
  })().catch(error => {library = undefined; throw error;});
  return library;
}

let images;
const queue = new CardGlintRenderQueue(async face => {
  const api = await loadCardGlint();
  images ??= new CardGlintImageCache(api.loadCanvasImages);
  const canvas = await api.composeCardTexture({face,printingId:null,printedImage:null}, 'inspect', new AbortController().signal,
    {loadImages:(urls,signal)=>images.images(urls,signal)});
  return {canvas,face,api};
});
export const renderCardGlint = (face,options) => queue.render(face,options);
export const hasCardGlintFrame = face => queue.has(face);

export const peekCardGlintFrame = face => queue.peek(face);
