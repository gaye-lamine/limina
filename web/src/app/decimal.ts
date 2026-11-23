/**
 * Reads a decimal number typed by the user. Accepts a point or a comma as
 * the decimal separator ("0.60" and "0,60"). Returns null when the text is
 * empty or not a finite number. The text itself is never rewritten.
 */
export function parseDecimal(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === '') return null;
  const commas = (trimmed.match(/,/g) ?? []).length;
  const normalized = commas === 1 && !trimmed.includes('.') ? trimmed.replace(',', '.') : trimmed;
  if (!/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(normalized)) return null;
  const n = Number(normalized);
  return Number.isFinite(n) ? n : null;
}
