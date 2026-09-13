// Zone animations need the previous card markup, not a detached DOM tree.
// Cloning full prepared frames on every phase can block input for hundreds of
// milliseconds. Serialize once and invalidate only when that card changes.
export function createCardVisualSnapshotCache() {
  let snapshots = new WeakMap();
  let observer = null;
  const invalidate = records => {
    for (const record of records) {
      if (record.type === 'attributes'
          && record.oldValue === record.target.getAttribute(record.attributeName)) continue;
      let element = record.target.nodeType === 1 ? record.target : record.target.parentElement;
      for (; element; element = element.parentElement) snapshots.delete(element);
    }
  };
  return {
    read(element) {
      if (!observer) {
        observer = new MutationObserver(invalidate);
        observer.observe(element.ownerDocument.documentElement, {
          subtree: true, childList: true, characterData: true,
          attributes: true, attributeOldValue: true,
        });
      }
      // Layout effects can read before observer callbacks have been delivered.
      invalidate(observer.takeRecords());
      if (!snapshots.has(element)) {
        let html = element.outerHTML;
        if (element.classList.contains('battlefield-row-card--layout-hold')) {
          html = html.replace(/^(<[^>]*\bclass=")([^"]*)"/, (_, prefix, classes) =>
            `${prefix}${classes.split(/\s+/).filter(name => name !== 'battlefield-row-card--layout-hold').join(' ')}"`);
        }
        snapshots.set(element, html);
      }
      return snapshots.get(element);
    },
    dispose() {
      observer?.disconnect();
      observer = null;
      snapshots = new WeakMap();
    },
  };
}
