export const ART_RATIO = 1.37;
export const DEFAULT_ART_CROP = { zoom: 1, x: 50, y: 50, width: 822 };

export function loadArtImage(url) {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = 'anonymous';
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('Could not open this image. For a URL that blocks editing, download the image and upload it here.'));
    image.src = url;
  });
}

export function drawArtCrop(canvas, image, crop) {
  const width = Number(crop.width);
  canvas.width = width;
  canvas.height = Math.round(width / ART_RATIO);
  const sourceWidth = Math.min(image.naturalWidth, image.naturalHeight * ART_RATIO) / crop.zoom;
  const sourceHeight = sourceWidth / ART_RATIO;
  canvas.getContext('2d').drawImage(image,
    (image.naturalWidth - sourceWidth) * crop.x / 100,
    (image.naturalHeight - sourceHeight) * crop.y / 100,
    sourceWidth, sourceHeight, 0, 0, canvas.width, canvas.height);
}

export async function readLocalArt(file) {
  if (!/^image\/(png|jpeg|webp|gif|avif)$/i.test(file.type)) {
    throw new Error('Choose a PNG, JPEG, WebP, GIF, or AVIF image.');
  }
  if (file.size > 20 * 1024 * 1024) throw new Error('Choose an image smaller than 20 MB.');
  const url = await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.onerror = () => reject(new Error('Could not read this file. Please choose it again.'));
    reader.readAsDataURL(file);
  });
  const image = await loadArtImage(url);
  // Keep an editable source without storing a full-resolution camera photo.
  const scale = Math.min(1, 2048 / Math.max(image.naturalWidth, image.naturalHeight));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
  canvas.getContext('2d').drawImage(image, 0, 0, canvas.width, canvas.height);
  return canvas.toDataURL('image/webp', 0.94);
}
