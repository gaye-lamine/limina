/**
 * Preparing an image of a text for reading: accepted types, and a resize in
 * the browser so that the upload stays under the size the server accepts.
 */

/** Image types the reader accepts. */
export const IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/webp'] as const;

/** Largest base64 payload sent; the request body must stay under the server's 2 MB limit. */
export const MAX_BASE64 = 1_800_000;

/** Longest side, in pixels, of a resized image: enough to read a page of text. */
export const MAX_SIDE = 2000;

/** Why an image cannot be read, or null when it can. */
export function imageProblem(file: { type: string; size: number }): 'type' | 'empty' | null {
  if (!(IMAGE_TYPES as readonly string[]).includes(file.type)) return 'type';
  if (file.size === 0) return 'empty';
  return null;
}

/** The size that fits within `max` on its longest side, keeping the proportions. */
export function fitWithin(width: number, height: number, max: number): { width: number; height: number } {
  const longest = Math.max(width, height);
  if (longest <= max) return { width, height };
  const k = max / longest;
  return { width: Math.round(width * k), height: Math.round(height * k) };
}

/** The base64 of a data URL, without its "data:...;base64," prefix. */
export function base64Of(dataUrl: string): string {
  return dataUrl.replace(/^data:[^,]*,/, '');
}

function readAsDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('image'));
    img.src = url;
  });
}

/**
 * The image as the reader receives it: unchanged when small enough,
 * otherwise redrawn at most MAX_SIDE pixels on its longest side as JPEG,
 * lowering the quality until it fits MAX_BASE64.
 */
export async function prepareImage(file: File): Promise<{ mime: string; base64: string }> {
  const original = await readAsDataUrl(file);
  if (base64Of(original).length <= MAX_BASE64) return { mime: file.type, base64: base64Of(original) };
  const img = await loadImage(original);
  const size = fitWithin(img.naturalWidth, img.naturalHeight, MAX_SIDE);
  const canvas = document.createElement('canvas');
  canvas.width = size.width;
  canvas.height = size.height;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('canvas');
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, size.width, size.height);
  ctx.drawImage(img, 0, 0, size.width, size.height);
  for (const quality of [0.9, 0.8, 0.7, 0.6, 0.5]) {
    const b64 = base64Of(canvas.toDataURL('image/jpeg', quality));
    if (b64.length <= MAX_BASE64) return { mime: 'image/jpeg', base64: b64 };
  }
  throw new Error('too large');
}
