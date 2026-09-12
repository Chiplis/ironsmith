# Interface translations

`locales/en.js` and `locales/es.js` own interface copy. `catalog.js` registers locales and handles lookup, interpolation and plural selection. `messages.js` is a compatibility export of the keyed messages; do not add translations there or in components.

- `messages`: existing stable keys used by `useI18n().t()`.
- `ui`: English source keys used by `useUiText()`. Add the source and its translation to every locale together.
- `rules`: localized keyword-help titles and summaries, indexed by the stable rule IDs from the generated rules reference. Spanish lives in `locales/es.rules.js`; the English catalog re-exports the generated reference without duplicating it.
- `optionalMarkers`: locale-specific optional-action grammar for matching printed decision text.

Use the reactive hook at the display boundary:

```jsx
const ui = useUiText();
return <button aria-label={ui('Undo tap of {0}', { 0: card.name })}>
  {ui('Undo')}
</button>;
```

Translate UI-owned fragments before inserting them into another sentence. Keep card names, player names, entered text and protocol values as data. Never translate an engine command, signed multiplayer payload, card identifier or lookup key. The separate card translation loaders continue to read localized printings and generated text assets; these are card content, not interface copy.

For non-React display helpers, use `translateUiText(source, params, locale)`. Prefer explicit templates and parameters. The compatibility matcher also recognizes cataloged templates in existing engine status messages, preserving captured data verbatim. `localizedParameters` in the registry explicitly identifies the few fields that hold UI vocabulary (such as a destination zone), so those fields translate while names remain intact. Unknown messages fall back to their original text.

A plural value can be `{ count: '0', one: '…{0}…', other: '…{0}…' }`. Categories use `Intl.PluralRules`. A locale may declare `omit` for obsolete English suffix parameters in legacy messages; new templates should pass a count rather than an English plural suffix.

To add a locale, copy the English catalog, translate every entry and keyword-help item, supply its optional-action marker if applicable, then register it in `localeCatalogs` and `LOCALES` in `catalog.js`. The language picker uses that registry. There are no component-specific locale switches.

Run `npm run i18n:check` and `npm run test:i18n`. The audit catches raw JSX text, static accessibility labels and missing source keys. Tests check locale/key parity, interpolation data, plural forms and keyword coverage. Browser checks live in `tests/ui-i18n.browser.test.js`; they exercise actual dialogs, tooltips and live switching while preserving input.
