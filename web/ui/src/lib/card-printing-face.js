// A printing request is shared by both sides, but text/typography preparation
// must use the face in the displayed image, regardless of CDN size or revision.
const imageFaceKey = url => String(url || '').match(/\/([^/]+)\/([0-9a-f])\/([0-9a-f])\/([0-9a-f-]{36})\.[a-z]+(?:\?.*)?$/i)?.slice(1).join('/');
// Keep derived faces stable for a cached printing. React previews use this
// identity to keep the completed canvas; recreating it starts another render.
// Weak keys let evicted printing responses and their derived faces be collected.
const resolvedFaces = new WeakMap();
export function printingForImageFace(printing, imageUrl) {
  const key = imageFaceKey(imageUrl);
  if (!key || !printing?.card_faces) return printing;
  const face = printing.card_faces.find(face => Object.values(face.image_uris || {}).some(url => imageFaceKey(url) === key));
  if (!face) return printing;
  let faces = resolvedFaces.get(printing);
  if (!faces) { faces = new WeakMap(); resolvedFaces.set(printing, faces); }
  if (!faces.has(face)) faces.set(face, {...printing, ...face});
  return faces.get(face);
}
