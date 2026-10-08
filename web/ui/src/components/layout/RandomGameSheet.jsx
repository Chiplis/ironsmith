import { useRef, useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { useGame } from '@/context/GameContext';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import { createSeededRng } from '@/lib/random-game';
import { buildCatalogRandomGame } from '@/lib/catalog-random-game';
import { PUBLIC_FORMATS } from '@/lib/relay/formats';

const fieldClass = 'fantasy-field w-full px-3 py-2 text-[14px] text-foreground';
const labelClass = 'grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground';

export default function RandomGameSheet({ trigger, onGenerate, disabled = false }) {
  const ui = useUiText();
  const { setStatus, semanticThreshold = 96 } = useGame();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [seed, setSeed] = useState('');
  const [format, setFormat] = useState('modern');
  const [life, setLife] = useState(20);
  const [progress, setProgress] = useState(null);
  const abortRef = useRef(null);
  const generate = async () => {
    if (busy) return;
    const actualSeed = seed.trim() || Math.random().toString(36).slice(2, 10);
    const controller = new AbortController();
    abortRef.current = controller;
    setBusy(true);
    try {
      const payload = await buildCatalogRandomGame({ format, startingLife: life,
        minScore: semanticThreshold / 100, rng: createSeededRng(actualSeed),
        signal: controller.signal, onProgress: setProgress });
      if (controller.signal.aborted) return;
      if (await onGenerate?.(payload, `Random game generated (seed ${actualSeed})`) !== false) setOpen(false);
    } catch (error) {
      if (!controller.signal.aborted) setStatus(`Random game failed: ${error.message || error}`, true);
    } finally {
      setBusy(false);
      setProgress(null);
      abortRef.current = null;
    }
  };
  return <Sheet open={open} onOpenChange={next => {
    if (!next) abortRef.current?.abort();
    setOpen(next);
  }}>
    <SheetTrigger asChild>{trigger}</SheetTrigger>
    <SheetContent side="center" className="fantasy-sheet random-game-sheet w-[min(94vw,620px)] p-0">
      <SheetHeader className="fantasy-sheet-header pr-12">
        <SheetTitle>{ui('Random Game')}</SheetTitle>
        <SheetDescription>{ui('Generate a 1v1 board from two lobby catalog decks. Each card comes from its deck; the remaining cards stay in the library.')}</SheetDescription>
        <Button className="random-game-submit ui-primary-action w-full" onClick={generate} disabled={disabled || busy}>
          {busy ? ui('Collecting {0}/{1}', { 0: progress?.collected || 0, 1: 2 }) : ui('Generate')}
        </Button>
      </SheetHeader>
      <div className="random-game-sheet-body grid gap-4 p-4">
        <label className={labelClass}>{ui('Format')}<select aria-label={ui('Format')} className={fieldClass} value={format} disabled={busy} onChange={event => {
          setFormat(event.target.value); setLife(PUBLIC_FORMATS[event.target.value].startingLife);
        }}>{Object.values(PUBLIC_FORMATS).map(entry => <option key={entry.id} value={entry.id}>{entry.label}</option>)}</select></label>
        <label className={labelClass}>{ui('Starting life')}<input aria-label={ui('Starting life')} type="number" min={1} max={999} className={fieldClass} value={life} disabled={busy} onChange={event => setLife(Math.max(1, Number(event.target.value) || 20))} /></label>
        <label className={labelClass}>{ui('Seed')}<input aria-label={ui('Seed')} className={fieldClass} value={seed} disabled={busy} onChange={event => setSeed(event.target.value)} /></label>
      </div>
    </SheetContent>
  </Sheet>;
}
