import { createHash, randomUUID } from 'node:crypto';
import { mkdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';

const FAILURE = /cheat detected|sync failed|synchronization failed|checkpoint.*mismatch|command type does not match|match start failed|auto-pass failed|unknown ziffle ceremony/i;
const digest = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');

// This function reads rendered DOM only. It neither touches application stores
// nor calls engine methods. Selectors stay in the adapter; callers use refs.
function readVisibleDom() {
  const visible = element => {
    if (!(element instanceof Element) || element.closest('[hidden],[aria-hidden="true"]')) return false;
    const box = element.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return false;
    for (let ancestor = element; ancestor; ancestor = ancestor.parentElement) {
      const style = getComputedStyle(ancestor);
      if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity) === 0) return false;
    }
    return true;
  };
  const text = element => String(element?.innerText || '').replace(/\s+/g, ' ').trim();
  const labelText = label => {
    const clone = label.cloneNode(true);
    for (const control of clone.querySelectorAll('input,textarea,select,button')) control.remove();
    return String(clone.textContent || '').replace(/\s+/g, ' ').trim();
  };
  const name = element => {
    const labelledBy = element.getAttribute('aria-labelledby');
    if (labelledBy) {
      const value = labelledBy.split(/\s+/).map(id => text(document.getElementById(id))).join(' ').trim();
      if (value) return value;
    }
    return element.getAttribute('aria-label') || [...(element.labels || [])].map(labelText).join(' ')
      || (element.tagName === 'LABEL' ? labelText(element) : text(element))
      || element.getAttribute('title') || element.getAttribute('placeholder') || element.getAttribute('alt') || '';
  };
  const selector = element => {
    const parts = [];
    for (let node = element; node && node.nodeType === 1; node = node.parentElement) {
      if (node.id && document.querySelectorAll(`#${CSS.escape(node.id)}`).length === 1) {
        parts.unshift(`#${CSS.escape(node.id)}`); break;
      }
      const tag = node.tagName.toLowerCase();
      const peers = node.parentElement ? [...node.parentElement.children].filter(peer => peer.tagName === node.tagName) : [node];
      parts.unshift(`${tag}:nth-of-type(${peers.indexOf(node) + 1})`);
    }
    return parts.join(' > ');
  };
  const dialogs = [...document.querySelectorAll('[role="dialog"][aria-modal="true"]')].filter(visible);
  const modal = dialogs.at(-1);
  const elements = [...document.querySelectorAll('button,input:not([type="hidden"]),textarea,select,a[href],[role="button"],[role="radio"],[role="checkbox"],[role="option"],[data-object-id],[data-action-row],[data-player-target],[data-bf-side],[contenteditable="true"],[tabindex],label')]
    .filter(element => visible(element) && (element.tagName !== 'LABEL' || element.control || element.querySelector('input,textarea,select')));
  const controls = elements.map(element => {
    const associated = element.tagName === 'LABEL' ? element.control || element.querySelector('input,textarea,select') : element;
    const tag = element.tagName.toLowerCase();
    const role = element.getAttribute('role') || (tag === 'button' ? 'button' : tag === 'a' ? 'link' : '');
    const type = associated?.getAttribute('type') || '';
    const editable = ['input', 'textarea'].includes(tag) || element.getAttribute('contenteditable') === 'true';
    const attrs = {};
    for (const attr of ['data-object-id', 'data-stable-id', 'data-card-name', 'data-member-object-ids',
      'data-action-row', 'data-player-target', 'data-target-legal', 'data-zone-card', 'data-zone-pile', 'data-zone-owner',
      'data-bf-side', 'data-placement-active', 'data-battlefield-drop-grid', 'data-battlefield-grid-columns',
      'data-battlefield-grid-rows', 'data-battlefield-column-capacity',
      'aria-selected', 'aria-pressed', 'aria-checked', 'aria-expanded', 'aria-controls', 'title', 'placeholder', 'name']) {
      if (element.hasAttribute(attr)) attrs[attr] = element.getAttribute(attr);
    }
    const ancestry = [];
    for (let ancestor = element.parentElement; ancestor && ancestry.length < 6; ancestor = ancestor.parentElement) {
      const attributes = {};
      for (const attr of ['aria-label', 'data-card-navigation-scope', 'data-bf-side', 'data-zone-owner', 'data-local-zone-piles', 'data-player-target']) {
        if (ancestor.hasAttribute(attr)) attributes[attr] = ancestor.getAttribute(attr);
      }
      const classes = String(ancestor.getAttribute('class') || '').split(/\s+/).filter(value => /zone|hand|battlefield|opponent|player|my-zone/.test(value));
      if (classes.length || Object.keys(attributes).length) ancestry.push({ tag: ancestor.tagName.toLowerCase(), classes, attributes });
    }
    const blocked = Boolean(element.closest('[inert]') || (modal && !modal.contains(element)));
    const box = element.getBoundingClientRect();
    const rect = Object.fromEntries(['x', 'y', 'width', 'height'].map(key => [key, Math.round(box[key] * 10) / 10]));
    const controlName = element.hasAttribute('data-bf-side') ? `${element.getAttribute('data-bf-side')} battlefield` : name(element);
    return { selector: selector(element), tag, role, type, name: controlName.slice(0, 1200), rect,
      text: text(element).slice(0, 2400), disabled: Boolean(associated?.disabled || element.getAttribute('aria-disabled') === 'true'),
      blocked, readonly: Boolean(associated?.readOnly),
      value: ['input', 'textarea', 'select'].includes(tag) ? String(element.value ?? '') : undefined,
      checked: ['checkbox', 'radio'].includes(type) ? Boolean(associated?.checked) : undefined,
      options: tag === 'select' ? [...element.options].map(option => ({ value: option.value, label: option.label, disabled: option.disabled, selected: option.selected })) : undefined,
      actions: [...(!blocked ? ['click', 'hover', 'pointer_click', 'drag'] : []), ...(editable && !element.readOnly ? ['fill'] : []),
        ...(!blocked && (role === 'button' || tag === 'label' || (tag === 'input' && type === 'file')) ? ['upload_file'] : []),
        ...(tag === 'select' ? ['select'] : []), 'press'], attributes: attrs,
      className: String(element.getAttribute('class') || '').split(/\s+/)
        .filter(value => /^(?:game-card|hand-card|field-card|stack-card|battlefield-(?:grouped|token)-card|card-action-available|card-targeting-mode|tapped|untapped|playable|summoning-sick|attack-selected|block-selected|blocker-candidate|attack-candidate|target-legal|target-illegal|card-chosen|selected|inspected|glow-[a-z-]+)$/.test(value)).join(' '), ancestry,
    };
  });
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const lines = [];
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (!node.parentElement.closest('script,style,template') && visible(node.parentElement)) {
      const value = node.textContent.replace(/\s+/g, ' ').trim();
      if (value) lines.push(value);
    }
  }
  return { url: location.href, title: document.title, text: lines.join('\n'), controls,
    dialogs: dialogs.map(dialog => ({ name: name(dialog), text: text(dialog).slice(0, 16000) })) };
}

function fingerprint(dom) {
  // Clock ticks do not invalidate a play, but changes to cards, controls,
  // decisions, input values, or public text do. Retain clocks in observations.
  const normalize = value => typeof value === 'string' ? value
    .replace(/\b\d{1,2}:\d{2}(?::\d{2})?\b/g, '<clock>')
    .replace(/\b\d+(?:\.\d+)?\s*(?:ms|seconds?|minutes?)\b/gi, '<elapsed>') : value;
  const controls = dom.controls.map(({ className, rect: _rect, ...control }) => ({ ...control,
    name: normalize(control.name), text: normalize(control.text),
    visualState: className.split(/\s+/).filter(value => /^(?:tapped|untapped|playable|is-playable|summoning-sick|attack-selected|block-selected|blocker-candidate|attack-candidate|target-legal|target-illegal|card-chosen|selected)$/.test(value)),
  }));
  return digest({ url: dom.url, text: normalize(dom.text), controls });
}

export class PlayerBrowser {
  constructor({ viewport = { width: 1600, height: 1100 }, settleMs = 300, settleTimeoutMs = 1800,
    actionTimeoutMs = 15000, launchOptions = {}, evidenceDir = null, ignoreHTTPSErrors = false } = {}) {
    this.viewport = viewport;
    this.settleMs = settleMs;
    this.settleTimeoutMs = settleTimeoutMs;
    this.actionTimeoutMs = actionTimeoutMs;
    this.launchOptions = launchOptions;
    this.evidenceDir = evidenceDir && path.resolve(evidenceDir);
    this.ignoreHTTPSErrors = ignoreHTTPSErrors;
    this.browsers = new Map();
    this.players = new Map();
  }

  async openPlayer({ name = 'Player', url, deckText = '', commanderText = '', headless = false }) {
    const target = new URL(url);
    if (!['http:', 'https:'].includes(target.protocol)) throw new Error('Player URL must use HTTP or HTTPS');
    target.searchParams.set('name', name);
    if (deckText) target.searchParams.set('deck', Buffer.from(deckText, 'utf8').toString('base64url'));
    if (commanderText) target.searchParams.set('commander', Buffer.from(commanderText, 'utf8').toString('base64url'));
    const mode = Boolean(headless);
    if (!this.browsers.has(mode)) this.browsers.set(mode, chromium.launch({ ...this.launchOptions, headless: mode }));
    const browser = await this.browsers.get(mode);
    const context = await browser.newContext({ viewport: this.viewport, locale: 'en-US', ignoreHTTPSErrors: this.ignoreHTTPSErrors, acceptDownloads: true });
    const page = await context.newPage();
    page.setDefaultTimeout(this.actionTimeoutMs);
    const playerId = `player-${randomUUID().slice(0, 8)}`;
    const player = { playerId, name, deckText, commanderText, page, context, sequence: 0, observation: null,
      evidence: [], downloads: [], inflight: new Set(), queue: Promise.resolve() };
    const record = (kind, message) => {
      player.evidence.push({ at: new Date().toISOString(), kind, message: String(message).slice(0, 12000) });
      if (player.evidence.length > 100) player.evidence.shift();
    };
    page.on('pageerror', error => record('pageerror', error.stack || error.message));
    page.on('console', message => { if (FAILURE.test(message.text())) record('sync-failure', message.text()); });
    page.on('request', request => player.inflight.add(request));
    page.on('requestfinished', request => player.inflight.delete(request));
    page.on('requestfailed', request => { player.inflight.delete(request); record('requestfailed', `${request.method()} ${request.url()} ${request.failure()?.errorText || ''}`); });
    page.on('download', download => {
      const item = { at: new Date().toISOString(), filename: download.suggestedFilename(), status: 'saving' };
      player.downloads.push(item);
      (async () => {
        if (!this.evidenceDir) { item.status = 'available'; return; }
        await mkdir(this.evidenceDir, { recursive: true });
        item.path = path.join(this.evidenceDir, `${playerId}-${randomUUID().slice(0, 8)}-${path.basename(item.filename)}`);
        await download.saveAs(item.path);
        item.status = 'saved';
      })().catch(error => { item.status = 'failed'; item.error = error.message; record('download-error', error.message); });
    });
    this.players.set(playerId, player);
    try {
      await page.goto(target.href, { waitUntil: 'domcontentloaded', timeout: 60000 });
      await page.locator('body').waitFor({ state: 'visible' });
      await this.settle(player);
      return { playerId, observation: await this.capture(player) };
    } catch (error) {
      await this.closePlayer({ playerId });
      throw error;
    }
  }

  player(playerId) {
    const player = this.players.get(playerId);
    if (!player) throw new Error(`Unknown player: ${playerId}`);
    return player;
  }

  async settle(player) {
    const started = Date.now();
    let previous = '', stableSince = started;
    while (Date.now() - started < this.settleTimeoutMs) {
      const dom = await player.page.evaluate(readVisibleDom);
      const current = fingerprint(dom);
      if (current !== previous) { previous = current; stableSince = Date.now(); }
      if (Date.now() - stableSince >= this.settleMs) return;
      await player.page.waitForTimeout(100);
    }
  }

  async capture(player, dom = null) {
    dom ||= await player.page.evaluate(readVisibleDom);
    const stateHash = fingerprint(dom);
    const refs = new Map();
    const controls = dom.controls.map(control => {
      const { selector, ...publicControl } = control;
      const ref = `r-${digest({ selector, tag: control.tag, name: control.name, objectId: control.attributes['data-object-id'] }).slice(0, 12)}`;
      refs.set(ref, { selector, control });
      return { ref, ...publicControl };
    });
    const observationId = `${player.playerId}:${++player.sequence}:${stateHash.slice(0, 12)}`;
    const failures = dom.text.split('\n').filter(line => FAILURE.test(line));
    const observation = { playerId: player.playerId, name: player.name, observationId, observedAt: new Date().toISOString(),
      url: dom.url, title: dom.title, text: dom.text.slice(0, 40000), textTruncated: dom.text.length > 40000,
      controls, cards: controls.filter(control => control.attributes['data-object-id']), dialogs: dom.dialogs,
      pendingRequests: player.inflight.size, evidence: [...player.evidence], syncFailures: failures, downloads: [...player.downloads] };
    player.observation = { observationId, stateHash, refs };
    return observation;
  }

  async observe({ playerId }) { return this.capture(this.player(playerId)); }

  async act({ playerId, observationId, ref, action, value, position }) {
    const player = this.player(playerId);
    const run = async () => {
      const prior = player.observation;
      const dom = await player.page.evaluate(readVisibleDom);
      if (!prior || observationId !== prior.observationId || fingerprint(dom) !== prior.stateHash) {
        const error = new Error('The visible page changed. Choose an action from the fresh observation.');
        error.code = 'STALE_OBSERVATION';
        error.observation = await this.capture(player, dom);
        throw error;
      }
      const target = prior.refs.get(ref);
      if (!target) throw new Error(`Unknown action ref: ${ref}`);
      if (target.control.disabled || target.control.blocked) throw new Error('This control is disabled or covered by a modal');
      const locator = player.page.locator(target.selector);
      if (action === 'pointer_click' || action === 'drag') {
        const viewport = await player.page.evaluate(() => ({ width: innerWidth, height: innerHeight }));
        if (!position || !Number.isFinite(position.x) || !Number.isFinite(position.y)
          || position.x < 0 || position.y < 0 || position.x >= viewport.width || position.y >= viewport.height) {
          throw new Error('Pointer position must be inside the visible viewport');
        }
        if (action === 'pointer_click') {
          const uncovered = await locator.evaluate((element, point) => {
            const rect = element.getBoundingClientRect();
            const hit = document.elementFromPoint(point.x, point.y);
            return point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
              && Boolean(hit && (hit === element || element.contains(hit))) && !hit.closest('[inert]');
          }, position);
          if (!uncovered) throw new Error('Pointer position is outside this control or covered by another element');
          // Real pointer movement updates the normal placement preview before
          // the ordinary mouse click releases a keyboard-held card.
          await player.page.mouse.move(position.x, position.y);
          const stillUncovered = await locator.evaluate((element, point) => {
            const hit = document.elementFromPoint(point.x, point.y);
            return Boolean(hit && (hit === element || element.contains(hit))) && !hit.closest('[inert]');
          }, position);
          if (!stillUncovered) throw new Error('Pointer target moved or became covered before clicking');
          await player.page.mouse.click(position.x, position.y);
        } else {
          await locator.scrollIntoViewIfNeeded({ timeout: this.actionTimeoutMs });
          const source = await locator.evaluate(element => {
            const rect = element.getBoundingClientRect();
            const left = Math.max(0, rect.left), right = Math.min(innerWidth, rect.right);
            const top = Math.max(0, rect.top), bottom = Math.min(innerHeight, rect.bottom);
            // A fanned hand overlaps cards, so find an exposed point on the
            // observed source instead of clicking through the card above it.
            for (const y of [0.5, 0.1, 0.9, 0.25, 0.75]) {
              for (const x of [0.5, 0.1, 0.9, 0.25, 0.75]) {
                const point = { x: left + (right - left) * x, y: top + (bottom - top) * y };
                const hit = document.elementFromPoint(point.x, point.y);
                if (hit && (hit === element || element.contains(hit)) && !hit.closest('[inert]')) return point;
              }
            }
            return null;
          });
          if (!source) throw new Error('The drag source is covered or outside the visible viewport');
          await player.page.mouse.move(source.x, source.y);
          const stillOnSource = await locator.evaluate((element, point) => {
            const hit = document.elementFromPoint(point.x, point.y);
            return Boolean(hit && (hit === element || element.contains(hit))) && !hit.closest('[inert]');
          }, source);
          if (!stillOnSource) throw new Error('Drag source moved or became covered before pressing');
          await player.page.mouse.down();
          try { await player.page.mouse.move(position.x, position.y, { steps: 12 }); }
          finally { await player.page.mouse.up(); }
        }
      } else if (action === 'upload_file') {
        if (typeof value !== 'string' || !path.isAbsolute(value)) {
          throw new Error('upload_file requires an absolute file path in value');
        }
        const file = await stat(value).catch(() => null);
        if (!file?.isFile()) throw new Error('upload_file requires an existing regular file');
        // Open the ordinary chooser through the observed visible control. The
        // file input may be hidden and never needs a DOM reference of its own.
        const [chooser] = await Promise.all([
          player.page.waitForEvent('filechooser', { timeout: this.actionTimeoutMs }),
          locator.click(),
        ]);
        await chooser.setFiles(value);
      } else if (action === 'click') await locator.click();
      else if (action === 'fill') await locator.fill(String(value ?? ''));
      else if (action === 'select') await locator.selectOption(String(value));
      else if (action === 'press') await locator.press(String(value));
      else if (action === 'hover') await locator.hover();
      else throw new Error(`Unsupported action: ${action}`);
      await this.settle(player);
      return this.capture(player);
    };
    const result = player.queue.then(run, run);
    player.queue = result.catch(() => {});
    return result;
  }

  async joinLobby({ playerId, lobby }) {
    const player = this.player(playerId);
    const target = /^https?:\/\//i.test(String(lobby)) ? new URL(lobby) : new URL(player.page.url());
    if (!/^https?:\/\//i.test(String(lobby))) target.searchParams.set('lobby', String(lobby));
    target.searchParams.set('name', player.name);
    if (player.deckText) target.searchParams.set('deck', Buffer.from(player.deckText, 'utf8').toString('base64url'));
    if (player.commanderText) target.searchParams.set('commander', Buffer.from(player.commanderText, 'utf8').toString('base64url'));
    await player.page.goto(target.href, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await this.settle(player);
    return this.capture(player);
  }

  async screenshot({ playerId }) {
    let imagePath;
    if (this.evidenceDir) {
      await mkdir(this.evidenceDir, { recursive: true });
      imagePath = path.join(this.evidenceDir, `${playerId}-${Date.now()}.png`);
    }
    return { mimeType: 'image/png', data: await this.player(playerId).page.screenshot({ fullPage: false, path: imagePath }), ...(imagePath ? { path: imagePath } : {}) };
  }

  async closePlayer({ playerId }) {
    const player = this.players.get(playerId);
    if (!player) return { closed: false };
    this.players.delete(playerId);
    await player.context.close();
    return { closed: true, playerId };
  }

  async close() {
    await Promise.all([...this.players.keys()].map(playerId => this.closePlayer({ playerId })));
    await Promise.all([...this.browsers.values()].map(async browser => (await browser).close()));
    this.browsers.clear();
  }
}
