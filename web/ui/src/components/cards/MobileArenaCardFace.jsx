import LoadingCardArt from './LoadingCardArt';
import { cardArtCropUrl } from '@/lib/card-image-variants';
import { cardArtColors } from '@/lib/card-art-colors';
import { arenaFrameColor, arenaPermanentKind } from '@/lib/mobile-arena';

export default function MobileArenaCardFace({ card, artUrl, pending, primary, secondary }) {
  const kind = arenaPermanentKind(card);
  const statKind = kind === 'planeswalker' ? 'loyalty' : kind === 'battle' ? 'defense' : null;
  const liveCounter = statKind && (card.counters || []).find?.(counter => String(counter.kind).toLowerCase() === statKind);
  const statistic = statKind ? { label: card[statKind] ?? liveCounter?.amount ?? 0, title: statKind } : primary;
  const extraCounter = secondary && !(statKind && Number(secondary.label) === Number(statistic.label)) ? secondary : null;
  const token = Boolean(card.is_token || card.token);
  const chapters = [...new Set(String(card.oracle_text || '').match(/^(?:I|II|III|IV|V|VI)(?=\s*[—,])/gm) || [])];
  return <div className="arena-permanent" data-kind={kind} data-token={token || undefined}
    style={{ '--arena-frame': arenaFrameColor(card) }}>
    <div className="arena-permanent-art">
      <LoadingCardArt src={cardArtCropUrl(artUrl)} sourceKey={artUrl} pending={pending}
        colors={cardArtColors(card)} variant="battlefield" alt="" draggable={false} />
    </div>
    {statistic && <span className="arena-permanent-stat" aria-label={statistic.title}>{statistic.label}</span>}
    {extraCounter && <span className="arena-permanent-counter" aria-label={extraCounter.title}>{extraCounter.label}</span>}
    {kind === 'saga' && chapters.length > 0 && <span className="arena-saga-chapters" aria-label="Saga chapters">{chapters.map(chapter => <span key={chapter} style={{display:'block'}}>{chapter}</span>)}</span>}
    {card.tapped && <span className="arena-tapped-mark" aria-label="Tapped">↷</span>}
  </div>;
}
