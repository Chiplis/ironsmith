// Source-only flavor fields belong in the inspector's attribution link, not
// the canvas text layout. Keep the original URL intact for navigation/copying.
export function cardFlavorSource(value) {
  const text=String(value ?? '').trim();
  if (!/^https?:\/\/\S+$/i.test(text)) return null;
  try {
    const url=new URL(text);
    if (!url.hostname || url.username || url.password) return null;
    return {href:url.href,host:url.hostname.replace(/^www\./,'')};
  } catch {return null;}
}
