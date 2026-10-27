import { useEffect, useRef, useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { ART_RATIO, DEFAULT_ART_CROP, drawArtCrop, loadArtImage, readLocalArt } from '@/lib/custom-card-art';

export default function CustomCardArtEditor({ face, updateFace }) {
  const ui = useUiText();
  const [editor, setEditor] = useState(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const canvas = useRef(null);
  const generation = useRef(0);
  const drag = useRef(null);
  useEffect(() => () => { generation.current += 1; }, []);
  useEffect(() => {
    if (editor && canvas.current) drawArtCrop(canvas.current, editor.image, editor.crop);
  }, [editor]);

  async function openEditor(file) {
    const request = ++generation.current;
    setLoading(true);
    setError('');
    try {
      const source = file ? await readLocalArt(file) : (face.artSourceUrl || face.artUrl);
      const image = await loadArtImage(source);
      if (request !== generation.current) return;
      setEditor({ source, image, crop: { ...DEFAULT_ART_CROP, ...(!file ? face.artCrop : {}) } });
    } catch (failure) {
      if (request === generation.current) setError(failure.message);
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }
  function changeCrop(patch) {
    setEditor(current => ({ ...current, crop: { ...current.crop, ...patch } }));
  }
  function apply() {
    try {
      const artUrl = canvas.current.toDataURL('image/webp', 0.94);
      updateFace({ artUrl, artSourceUrl: editor.source, artCrop: editor.crop });
      setEditor(null);
      setError('');
    } catch {
      setError('This image host does not allow cropping. Download the image and upload it here instead.');
    }
  }
  function move(event) {
    if (!drag.current || !editor) return;
    const { image, crop } = editor;
    const sourceWidth = Math.min(image.naturalWidth, image.naturalHeight * ART_RATIO) / crop.zoom;
    const scale = event.currentTarget.getBoundingClientRect().width / sourceWidth;
    const clamp = value => Math.max(0, Math.min(100, value));
    const deltaX = image.naturalWidth - sourceWidth;
    const deltaY = image.naturalHeight - sourceWidth / ART_RATIO;
    changeCrop({
      x: deltaX > 0 ? clamp(drag.current.x - (event.clientX - drag.current.clientX) / scale / deltaX * 100) : 50,
      y: deltaY > 0 ? clamp(drag.current.y - (event.clientY - drag.current.clientY) / scale / deltaY * 100) : 50,
    });
  }
  return <div className="card-forge-inline-art-field">
    <label>
      <span className="card-forge-section-label">{ui('Art URL')}</span>
      <input className="card-forge-input card-forge-input--compact" aria-label={ui('Art URL')}
        placeholder={face.artUrl?.startsWith('data:') ? ui('Local image') : 'https://...'}
        value={face.artUrl?.startsWith('data:') ? '' : (face.artUrl || '')}
        onChange={event => {
          generation.current += 1;
          setLoading(false);
          setEditor(null);
          setError('');
          updateFace({ artUrl: event.target.value, artSourceUrl: '', artCrop: null });
        }} />
    </label>
    <div className="flex flex-wrap items-center gap-2 text-xs">
      <label className="cursor-pointer rounded border border-white/20 px-2 py-1 focus-within:ring-2">
        {ui(loading ? 'Loading image…' : 'Upload image')}
        <input type="file" className="sr-only" aria-label={ui('Upload card art')}
          accept="image/png,image/jpeg,image/webp,image/gif,image/avif" disabled={loading}
          onChange={event => { const file = event.target.files?.[0]; event.target.value = ''; if (file) void openEditor(file); }} />
      </label>
      <button type="button" className="rounded border border-white/20 px-2 py-1 disabled:opacity-40"
        disabled={!face.artUrl || loading} onClick={() => openEditor()}>{ui('Crop / resize')}</button>
      {face.artUrl && <button type="button" onClick={() => {
        generation.current += 1; setLoading(false); setEditor(null); setError('');
        updateFace({ artUrl: '', artSourceUrl: '', artCrop: null });
      }}>{ui('Remove art')}</button>}
    </div>
    {editor && <div className="custom-card-art-crop grid gap-2 rounded border border-white/20 p-2" aria-label={ui('Crop card art')}>
      <p className="text-xs text-muted-foreground">{ui('Drag to position the art, or use the sliders. Apply to update your card.')}</p>
      <canvas ref={canvas} aria-label={ui('Art crop preview')} className="w-full cursor-move rounded" style={{ aspectRatio: ART_RATIO, touchAction: 'none' }}
        onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); drag.current = { clientX: event.clientX, clientY: event.clientY, ...editor.crop }; }}
        onPointerMove={move} onPointerUp={() => { drag.current = null; }} onPointerCancel={() => { drag.current = null; }} />
      {[
        ['zoom', 'Zoom', 1, 4, 0.01], ['x', 'Horizontal position', 0, 100, 1], ['y', 'Vertical position', 0, 100, 1],
      ].map(([key, label, min, max, step]) => <label key={key} className="grid gap-1 text-xs">
        {ui(label)}
        <input type="range" aria-label={ui(label)} min={min} max={max} step={step} value={editor.crop[key]}
          onChange={event => changeCrop({ [key]: Number(event.target.value) })} />
      </label>)}
      <label className="flex items-center justify-between gap-2 text-xs">{ui('Output size')}
        <select aria-label={ui('Output size')} className="card-forge-input card-forge-input--compact" style={{ width: 'auto' }} value={editor.crop.width}
          onChange={event => changeCrop({ width: Number(event.target.value) })}>
          {[411, 822, 1644].map(width => <option key={width} value={width}>{width} × {Math.round(width / ART_RATIO)} px</option>)}
        </select>
      </label>
      <div className="flex flex-wrap gap-3 text-xs">
        <button type="button" onClick={apply}>{ui('Apply art')}</button>
        <button type="button" onClick={() => changeCrop(DEFAULT_ART_CROP)}>{ui('Reset crop')}</button>
        <button type="button" onClick={() => { setEditor(null); setError(''); }}>{ui('Cancel')}</button>
      </div>
    </div>}
    {error && <p role="alert" className="text-xs text-red-300">{ui(error)}</p>}
  </div>;
}
