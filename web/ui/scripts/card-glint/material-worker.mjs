import { buildMaterial } from '@ironsmith/card-glint-material';

self.onmessage = async ({ data }) => {
  const { id, source, adjustment, images, tone } = data;
  try {
    const bitmap = await buildMaterial(source, adjustment, images, tone);
    self.postMessage({ id, bitmap }, [bitmap]);
  } catch (error) {
    self.postMessage({ id, error: String(error?.message || error) });
  } finally {
    source.close();
    for (const bitmap of images.values()) bitmap.close();
  }
};
