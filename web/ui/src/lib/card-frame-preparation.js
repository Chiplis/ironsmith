import { resolveScryfallFlavorText, resolveScryfallPrintingMetadata, resolveScryfallSetSymbol } from './scryfall';
import { fullCardImageUrl, preloadCardFrameSource, sampleCardFrameColors } from './card-frame-colors';
import { cardTypography } from './card-typography';
import {registrationForImage} from './card-region-layout';

const preparations = new Map();
const isBasicLand = typeLine => {
  const types = String(typeLine || '').split(/[—–]/)[0];
  return /\bbasic\b/i.test(types) && /\bland\b/i.test(types);
};
export const cardFramePreparationKey = (imageUrl, typeLine) => `${imageUrl}|${isBasicLand(typeLine)}`;

export function cachedCardFrame(imageUrl, typeLine = '') {
  return preparations.get(cardFramePreparationKey(imageUrl, typeLine))?.result || null;
}

async function decodeImage(url) {
  if (!url) return;
  const image = new Image();
  image.referrerPolicy = 'no-referrer';
  image.src = url;
  await image.decode();
}

async function prepareTypography(printing) {
  const typography = cardTypography(printing || {});
  const weight = name => name === 'rules' ? 400 : name === 'stats' ? typography.style['--card-stats-weight'] : typography.titleWeight;
  const sections = ['title', 'type', 'rules', 'stats'];
  await Promise.allSettled([
    ...sections.map(name => document.fonts.load(`${weight(name)} 100px ${typography[name]}`)),
    document.fonts.load(`italic 400 100px ${typography.rules}`),
  ]);
  const ctx = document.createElement('canvas').getContext('2d');
  const style = {...typography.style};
  for (const name of sections) {
    ctx.font = `${weight(name)} 100px ${typography[name]}`;
    const metrics = ctx.measureText(name === 'stats' ? '0' : 'x');
    style[`--card-${name}-glyph-ratio`] = Math.max(.3, (metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent) / 100);
  }
  return {...typography, printingReady: true, style};
}

// A single cached preparation serves hover prewarming and the mounted frame.
// Publish one complete bundle, including failed-request fallbacks, so React
// never reveals a mixture of the default and final printing styles.
export function prepareCardFrame(imageUrl, typeLine = '') {
  const key = cardFramePreparationKey(imageUrl, typeLine);
  if (preparations.has(key)) {
    const entry = preparations.get(key);
    preparations.delete(key);
    preparations.set(key, entry);
    return entry.promise;
  }
  const entry = { promise: null, result: null };
  const request = (async () => {
    void preloadCardFrameSource(imageUrl);
    const art = decodeImage(imageUrl).then(() => true, () => false);
    const flavor = resolveScryfallFlavorText(imageUrl).catch(() => '');
    const printing = await resolveScryfallPrintingMetadata(imageUrl);
    const typographyRequest = prepareTypography(printing);
    const setSymbolRequest = resolveScryfallSetSymbol(printing);
    const preparedTypography = await typographyRequest;
    const registration = registrationForImage((await import('./card-region-catalog.generated.js')).default, fullCardImageUrl(imageUrl));
    const style = registration ? {} : await sampleCardFrameColors(fullCardImageUrl(imageUrl), {
      typography: preparedTypography, printing, setSymbolUrl: await setSymbolRequest,
    });
    // CSS backgrounds and border images have their own decode step, even
    // after the canvas work has produced their data URLs.
    const urls = new Set(Object.values(style || {}).flatMap(value =>
      Array.from(String(value).matchAll(/url\("([^"]+)"\)/g), match => match[1])
    ));
    const [typography, flavorText, artReady] = await Promise.all([
      typographyRequest, flavor, art,
      Promise.allSettled([...urls].map(decodeImage)),
    ]);
    const result = {key, registration, printing, imageUrl, originalImageUrl: fullCardImageUrl(imageUrl) || imageUrl, style, typography, flavorText, artReady};
    if (fullCardImageUrl(imageUrl) && !style) {
      if (preparations.get(key) === entry) preparations.delete(key);
    } else entry.result = result;
    return result;
  })().catch(error => {
    if (preparations.get(key) === entry) preparations.delete(key);
    throw error;
  });
  entry.promise = request;
  preparations.set(key, entry);
  if (preparations.size > 48) preparations.delete(preparations.keys().next().value);
  return request;
}
