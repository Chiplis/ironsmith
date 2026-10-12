export function cardArtCropUrl(imageUrl) {
  const url = String(imageUrl || '');
  if (/\/custommagic\/images\/[^/]+-full\.webp(?:\?|$)/.test(url)) return url.replace(/-full\.webp(?=\?|$)/, '-art.webp');
  if (!/^https:\/\/cards\.scryfall\.io\/(?:small|normal|large|png|border_crop|art_crop)\//.test(url)) return url;
  return url.replace(/(cards\.scryfall\.io\/)\w+\//, '$1art_crop/').replace(/\.png(?=\?|$)/, '.jpg');
}
