export function cardArtCropUrl(imageUrl) {
  const url = String(imageUrl || '');
  if (!/^https:\/\/cards\.scryfall\.io\/(?:small|normal|large|png|border_crop|art_crop)\//.test(url)) return url;
  return url.replace(/(cards\.scryfall\.io\/)\w+\//, '$1art_crop/').replace(/\.png(?=\?|$)/, '.jpg');
}
