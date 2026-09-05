export const DEFAULT_UI_FONT = "Rajdhani";

const BASE_FALLBACK = '"Avenir Next", "Segoe UI", system-ui, sans-serif';

export const UI_FONT_OPTIONS = [
  { name: "Rajdhani", stack: `"Rajdhani", ${BASE_FALLBACK}` },
  { name: "Alegreya Sans SC", stack: `"Alegreya Sans SC", ${BASE_FALLBACK}` },
  { name: "System UI", stack: `system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` },
  { name: "Avenir Next", stack: `"Avenir Next", "Segoe UI", system-ui, sans-serif` },
  { name: "Segoe UI", stack: `"Segoe UI", system-ui, sans-serif` },
  { name: "Inter", stack: `"Inter", "Avenir Next", "Segoe UI", system-ui, sans-serif` },
  { name: "Helvetica Neue", stack: `"Helvetica Neue", Arial, system-ui, sans-serif` },
  { name: "Arial", stack: `Arial, "Helvetica Neue", system-ui, sans-serif` },
  { name: "Verdana", stack: `Verdana, Geneva, system-ui, sans-serif` },
  { name: "Tahoma", stack: `Tahoma, Geneva, system-ui, sans-serif` },
  { name: "Trebuchet MS", stack: `"Trebuchet MS", "Segoe UI", system-ui, sans-serif` },
  { name: "Optima", stack: `Optima, "Avenir Next", "Segoe UI", system-ui, sans-serif` },
  { name: "Georgia", stack: `Georgia, "Times New Roman", serif` },
  { name: "Times New Roman", stack: `"Times New Roman", Times, serif` },
  { name: "Menlo", stack: `Menlo, Monaco, "SFMono-Regular", Consolas, monospace` },
  { name: "Monaco", stack: `Monaco, Menlo, "SFMono-Regular", Consolas, monospace` },
  { name: "Courier New", stack: `"Courier New", Courier, monospace` },
];

function quoteFontFamily(family) {
  const trimmed = String(family || "").trim();
  if (!trimmed) return `"${DEFAULT_UI_FONT}"`;
  if (/^["'].*["']$/.test(trimmed)) return trimmed;
  if (/^[a-z-]+$/i.test(trimmed)) return trimmed;
  return `"${trimmed.replaceAll('"', '\\"')}"`;
}

export function uiFontStack(fontName) {
  const normalized = String(fontName || DEFAULT_UI_FONT).trim() || DEFAULT_UI_FONT;
  const option = UI_FONT_OPTIONS.find((entry) => entry.name.toLowerCase() === normalized.toLowerCase());
  if (option) return option.stack;
  if (normalized.includes(",")) return normalized;
  return `${quoteFontFamily(normalized)}, ${BASE_FALLBACK}`;
}
