// A printing request is shared by both sides, but text/typography preparation
// must use the face in the displayed image, regardless of CDN size or revision.
const imageFaceKey = url => String(url || '').match(/\/([^/]+)\/([0-9a-f])\/([0-9a-f])\/([0-9a-f-]{36})\.[a-z]+(?:\?.*)?$/i)?.slice(1).join('/');
export function printingForImageFace(printing, imageUrl) {
  const key = imageFaceKey(imageUrl);
  if (!key || !printing?.card_faces) return printing;
  const face = printing.card_faces.find(face => Object.values(face.image_uris || {}).some(url => imageFaceKey(url) === key));
  return face ? {...printing, ...face} : printing;
}
