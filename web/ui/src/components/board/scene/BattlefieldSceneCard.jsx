import { useLayoutEffect, useState } from 'react';
import { createPortal } from 'react-dom';

export default function BattlefieldSceneCard({ scene, objectKey, width, height, children }) {
  const [element] = useState(() => {
    const node = document.createElement('div');
    node.className = 'battlefield-scene-object';
    return node;
  });
  useLayoutEffect(() => {
    Object.assign(element.style, { width: `${width}px`, height: `${height}px` });
    element.style.setProperty('--bf-card-width', `${width}px`);
    element.style.setProperty('--bf-card-height', `${height}px`);
    element.setAttribute('data-scene-object', objectKey);
    scene.register(objectKey, element);
    return () => scene.unregister(objectKey, element);
  }, [element, height, objectKey, scene, width]);
  return createPortal(children, element);
}
