import { useLayoutEffect, useState } from 'react';

function displayedCard(objectId, stack) {
  if (objectId == null || typeof document === 'undefined') return null;
  const candidates = [...document.querySelectorAll('[data-object-id][data-card-image-url]')]
    .filter(node => node.dataset.objectId === String(objectId) && node.classList.contains('stack-card') === stack);
  return candidates.find(node => node.classList.contains('battlefield-row-card')) || candidates[0] || null;
}

// The displayed object is authoritative: name-based lookups can select another
// printing, face, variant, or bypass a per-object custom image.
export default function useDisplayedCardImage(objectId, stack = false) {
  const key = `${objectId}|${stack}`;
  const read = () => displayedCard(objectId, stack)?.dataset.cardImageUrl || '';
  const [snapshot, setSnapshot] = useState(() => ({ key, url: read() }));
  useLayoutEffect(() => {
    const node = displayedCard(objectId, stack);
    const update = () => setSnapshot({ key, url: node?.dataset.cardImageUrl || '' });
    update();
    if (!node) return undefined;
    const observer = new MutationObserver(update);
    observer.observe(node, { attributes: true, attributeFilter: ['data-card-image-url'] });
    return () => observer.disconnect();
  }, [key, objectId, stack]);
  return snapshot.key === key ? snapshot.url : read();
}
