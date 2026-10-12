// Build reusable, text-free stock frames from the renderer's verified masks.
// Donor art, expansion symbols, watermarks, stats and collector credits must
// never appear on a different card. Only the frame material is retained.
export async function defaultCardFrameTemplate(style) {
  if (style?.['--source-frame-status'] !== 'masked') throw new Error('Default frame requires a successful source mask');
  const image = new Image();
  image.src = style['--source-frame-image'].slice(5, -2);
  await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = Number(style['--printed-scan-width']);
  canvas.height = Number(style['--printed-scan-height']);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
  const bounds = name => Object.fromEntries(['x', 'y', 'width', 'height'].map(k => [k, Number(style[`--printed-${name}-${k}`])]));
  const rules = bounds('rules'), title = bounds('title'), type = bounds('type');
  const rgb = (x, y) => {
    const pixels = ctx.getImageData(Math.round(x) - 2, Math.round(y) - 2, 5, 5).data;
    return [0, 1, 2].map(channel => {
      const values = Array.from({ length: 25 }, (_, i) => pixels[i * 4 + channel]).sort((a, b) => a - b);
      return values[12];
    });
  };
  const papers = [title, type, rules].map(box => ({ box,
    left: rgb(box.x + box.width * .08, box.y + box.height * .5),
    right: rgb(box.x + box.width * .72, box.y + box.height * .5),
  }));
  ctx.save();
  ctx.beginPath(); ctx.roundRect(0, 0, canvas.width, canvas.height, 20); ctx.clip();
  // Remove the donor's P/T enclosure, security stamp and attribution before
  // rebuilding the blank paper. Live stat badges are supplied by the renderer.
  ctx.fillStyle = '#101112';
  ctx.fillRect(0, rules.y + rules.height - 30, canvas.width, canvas.height);
  // Stock frames have no collector credits or printed P/T enclosure. Give
  // that reserved space back to the rules, leaving only a narrow bottom rim.
  rules.height = canvas.height - rules.y - Math.round(canvas.width * .025);
  for (const { box, left, right } of papers) {
    const gradient = ctx.createLinearGradient(box.x, 0, box.x + box.width, 0);
    gradient.addColorStop(0, `rgb(${left.join(',')})`);
    gradient.addColorStop(1, `rgb(${right.join(',')})`);
    ctx.fillStyle = gradient;
    ctx.strokeStyle = '#34332d'; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.roundRect(box.x, box.y, box.width, box.height, box === rules ? 2 : 10);
    ctx.fill(); ctx.stroke();
  }
  const art = { x: rules.x + 2, y: title.y + title.height + 3,
    width: rules.width - 4, height: type.y - title.y - title.height - 4 };
  ctx.clearRect(art.x, art.y, art.width, art.height);
  ctx.restore();
  const geometry = Object.fromEntries(Object.entries(style).filter(([key, value]) =>
    !key.startsWith('--printed-pt-') && !key.includes('stats') && !key.includes('flavor')
    && !key.includes('text-bounds') && !key.includes('first-line') && !String(value).includes('url(')));
  for (const [key, value] of Object.entries(art)) geometry[`--printed-art-${key}`] = value;
  geometry['--printed-rules-height'] = rules.height;
  for (const key of ['--printed-layout', '--printed-layout-candidates']) {
    if (geometry[key]) {
      const layout = JSON.parse(geometry[key]);
      layout.rules = { ...layout.rules, height: rules.height };
      geometry[key] = JSON.stringify(layout);
    }
  }
  geometry['--printed-type-text-width'] = '100%';
  geometry['--printed-title-font-size'] = 'calc(5.4 * var(--card-frame-width-unit))';
  geometry['--printed-type-font-size'] = 'calc(4.5 * var(--card-frame-width-unit))';
  for (const section of ['title', 'type', 'rules', 'stats']) geometry[`--sampled-${section}-ink`] = '#171717';
  geometry['--printed-rules-padding-top'] = 'calc(2 * var(--card-frame-width-unit))';
  geometry['--printed-rules-font-size'] = 'calc(4.6 * var(--card-frame-width-unit))';
  geometry['--printed-rules-line-height'] = 1.15;
  return { image: canvas.toDataURL('image/webp', .95), style: geometry };
}
