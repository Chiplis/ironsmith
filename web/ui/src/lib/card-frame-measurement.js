// DOM ranges report transformed pixels, while font metrics and CSS padding
// use layout pixels. Measure miniatures in their canonical coordinate space;
// restoring the transform synchronously keeps this invisible to the user.
export function measureCardFrameLayout(node, measure) {
  const composition = node.closest('.battlefield-prepared-frame__composition');
  if (!composition) return measure();
  const transform = composition.style.transform;
  try {
    // Cancel battlefield tap/hover transforms too, without touching their
    // animations. Translation does not affect local fitting coordinates.
    let ancestors = new DOMMatrix();
    for (let parent = composition.parentElement; parent; parent = parent.parentElement) {
      const value = getComputedStyle(parent).transform;
      if (value !== 'none') ancestors = new DOMMatrix(value).multiply(ancestors);
    }
    composition.style.transform = ancestors.inverse().toString();
    return measure();
  } finally {
    composition.style.transform = transform;
  }
}
// Read DOM inputs without asking the browser to calculate layout. React often
// recreates rule children just to change their actions; that must not restart
// the write/measure binary search on every priority update.
export function cardFrameFitKey(node) {
  const fittedProperties = new Set([
    'font-size', 'transform', '--card-fitted-rules-font-size',
    '--card-fitted-flavor-font-size', '--card-rules-fit-scale',
    '--card-rules-spacing-scale', '--card-flavor-natural-top',
  ]);
  const attributes = (element, fitted = false) => [
    element.tagName, element.getAttribute('class'),
    Array.from(element.style || []).filter(name => !fitted || !fittedProperties.has(name))
      .map(name => [name, element.style.getPropertyValue(name)]),
    element.getAttribute('src'), element.getAttribute('data-card-era'),
  ];
  const content = (element) => element.nodeType === 3 ? element.textContent
    : element.nodeType === 1 ? [attributes(element, element === node),
      Array.from(element.childNodes, content)] : null;
  const ancestors = [];
  for (let parent = node.parentElement; parent; parent = parent.parentElement) {
    ancestors.push(attributes(parent));
    if (parent.classList.contains('interactive-card-frame-stage')) break;
  }
  return JSON.stringify([content(node), ancestors]);
}
