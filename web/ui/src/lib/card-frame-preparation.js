import { resolveScryfallEnglishPrinting, resolveScryfallFlavorText, resolveScryfallPrintingMetadata, resolveScryfallSetSymbol } from './scryfall';
import { fullCardImageUrl, preloadCardFrameSource } from './card-frame-colors';
import { sampleCardFrameColors } from './card-frame-processing';
import { cardTypography } from './card-typography';
import {registrationForImage, registrationForPrinting, registrationGeometryIsUsable} from './card-region-layout';

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
    let preparedTypography = await typographyRequest;
    let framePrinting = printing;
    const catalog = (await import('./card-region-catalog.generated.js')).default;
    const scanUrl = fullCardImageUrl(imageUrl);
    let registration = registrationForImage(catalog, scanUrl);
    // Localized art resolves to the same printing in another language. Its
    // registration was made on the pinned scan, so the frame masks and shows
    // that scan while every text field carries the live (translated) wording.
    let registeredScanUrl = registration ? scanUrl : '';
    if (!registration && printing) {
      registration = registrationForPrinting(catalog, printing, scanUrl);
      if (registration) registeredScanUrl = registration.source;
    }
    const invalidRegistration = registration && !registrationGeometryIsUsable(registration);
    if (invalidRegistration) {
      registration = null;
      registeredScanUrl = '';
    }
    const registeredScan = registeredScanUrl && registeredScanUrl !== scanUrl
      ? decodeImage(registeredScanUrl).then(() => true, () => false) : Promise.resolve(true);
    let style = invalidRegistration
      ? {'--source-frame-status': 'original', '--source-frame-fallback-reason': 'registration-geometry'}
      : registration ? {} : await sampleCardFrameColors(scanUrl, {
      typography: preparedTypography, printing, setSymbolUrl: await setSymbolRequest,
    });
    // Retry offscreen, publishing only a complete mask. The stage's preview
    // continues to use imageUrl, and a failed English attempt leaves the
    // original localized fallback intact (including its typography/flavor).
    if (!registration && !style?.['--source-frame-image'] && printing?.lang && printing.lang !== 'en') {
      try {
        const english = await resolveScryfallEnglishPrinting(imageUrl, printing);
        const englishUrl = english?.image_uris?.normal
          || fullCardImageUrl(english?.image_uris?.art_crop);
        if (englishUrl && englishUrl !== scanUrl) {
          const englishTypography = await prepareTypography(english);
          const englishStyle = await sampleCardFrameColors(englishUrl, {
            typography: englishTypography, printing: english,
            setSymbolUrl: await resolveScryfallSetSymbol(english),
          });
          if (englishStyle?.['--source-frame-image']) {
            style = englishStyle;
            preparedTypography = englishTypography;
            framePrinting = english;
          }
        }
      } catch { /* Preserve the translated original if the retry is unavailable. */ }
    }
    // CSS backgrounds and border images have their own decode step, even
    // after the canvas work has produced their data URLs.
    const urls = new Set(Object.values(style || {}).flatMap(value =>
      Array.from(String(value).matchAll(/url\("([^"]+)"\)/g), match => match[1])
    ));
    const [flavorText, artReady] = await Promise.all([
      flavor, art, registeredScan,
      Promise.allSettled([...urls].map(decodeImage)),
    ]);
    const result = {key, registration, printing: framePrinting, imageUrl, originalImageUrl: registeredScanUrl || scanUrl || imageUrl, style, typography: preparedTypography, flavorText, artReady};
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
