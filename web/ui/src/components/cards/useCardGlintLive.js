import useGraphicsQuality from '@/hooks/useGraphicsQuality';
import {useCallback,useEffect,useMemo,useRef,useState} from 'react';
import {createCardGlintLiveView} from '@/lib/card-glint/live.mjs';

// Mount card-glint's WebGL renderer on a canvas and keep it on the given face.
// `shown` stays false until the first face is ready.
export default function useCardGlintLive(face,presentation,{enabled=true}={}) {
  const [quality] = useGraphicsQuality();
  enabled = enabled && quality === 'high';
  const canvas = useRef(null), view = useRef(null);
  const session = useMemo(() => ({enabled}), [enabled]);
  const [loadedSession,setLoadedSession] = useState(null);
  const [mode,setMode] = useState('pending'), [shownKey,setShownKey] = useState(null);
  const key = face ? JSON.stringify(face) : '', presentationKey = JSON.stringify(presentation ?? null);

  useEffect(() => {
    if (!enabled) return undefined;
    let cancelled = false, created = null;
    createCardGlintLiveView(canvas.current,{
      onReady: ready => {if (!cancelled) setShownKey(JSON.stringify(ready));},
      onError: error => console.error('[card-glint] Live render failed',error),
    }).then(value => {
      if (cancelled) {value.dispose(); return;}
      created = value; view.current = value; setShownKey(null); setMode('live');
      setLoadedSession(session);
    }).catch(error => {
      // Without WebGL2 or the renderer module, hosts keep their static face.
      if (!cancelled) {console.warn('[card-glint] Live finishes unavailable',error); setMode('unavailable'); setLoadedSession(session);}
    });
    return () => {cancelled = true; created?.dispose(); view.current = null;};
  },[enabled,session]);

  useEffect(() => {
    if (enabled && loadedSession === session && mode === 'live' && key) view.current?.update(JSON.parse(key),JSON.parse(presentationKey));
  },[enabled,mode,key,presentationKey,loadedSession,session]);

  const pointer = useCallback(point => view.current?.pointer(point),[]);
  const ready = enabled && loadedSession === session;
  return [canvas,{mode:enabled ? (ready ? mode : 'pending') : 'off',shown:ready && Boolean(shownKey),current:ready && shownKey === key,pointer}];
}
