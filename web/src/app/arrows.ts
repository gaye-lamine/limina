/** Length of an arrow head along the arrow, in viewBox units. */
const HEAD = 14;
/** How far the head reaches past each side of the shaft. */
const HEAD_FLARE = 7;

function points(pts: number[][]): string {
  return pts.map(([px, py]) => `${px.toFixed(2)},${py.toFixed(2)}`).join(' ');
}

/** Polygon of an arrow pointing down, centred on x, from y0 to y1 (y1 > y0), shaft width w. */
export function arrowDown(x: number, y0: number, y1: number, w: number): string {
  const h = w / 2;
  const head = h + HEAD_FLARE;
  const neck = y1 - HEAD;
  return points([
    [x - h, y0],
    [x + h, y0],
    [x + h, neck],
    [x + head, neck],
    [x, y1],
    [x - head, neck],
    [x - h, neck],
  ]);
}

/** Polygon of an arrow pointing up, centred on x, from y0 up to y1 (y1 < y0), shaft width w. */
export function arrowUp(x: number, y0: number, y1: number, w: number): string {
  const h = w / 2;
  const head = h + HEAD_FLARE;
  const neck = y1 + HEAD;
  return points([
    [x - h, y0],
    [x + h, y0],
    [x + h, neck],
    [x + head, neck],
    [x, y1],
    [x - head, neck],
    [x - h, neck],
  ]);
}

/** Polygon of an arrow pointing right, centred on y, from x0 to x1, shaft width w. */
export function arrowRight(y: number, x0: number, x1: number, w: number): string {
  const h = w / 2;
  const head = h + HEAD_FLARE;
  const neck = x1 - HEAD;
  return points([
    [x0, y - h],
    [neck, y - h],
    [neck, y - head],
    [x1, y],
    [neck, y + head],
    [neck, y + h],
    [x0, y + h],
  ]);
}
