/**
 * Direct manipulation in the 3D laboratory (spec laboratory-3d, R-13): the
 * draggable handles of the views and the pure maps from a dragged point in
 * the scene to slider updates. A drag only moves sliders; the engine is
 * asked again exactly as when a slider moves.
 */
import { Scale } from './view-state';

/** A point of the scene. */
export interface Point3 {
  x: number;
  y: number;
  z: number;
}

/** A slider to move: to a value in its base unit, or to a place on its track (0 to 1). */
export interface DragUpdate {
  id: string;
  value?: number;
  position?: number;
}

/**
 * A draggable object, stored in `userData.handle` of its mesh: its name
 * (for the end-to-end checks), the plane the pointer moves it in
 * (vertical: facing the camera through the handle; horizontal: at its
 * height) and the map from a point of that plane to slider updates.
 */
export interface Handle {
  name: string;
  plane: 'vertical' | 'horizontal';
  map: (p: Point3) => DragUpdate[];
}

/** Key of a handle in an object's `userData`. */
export const HANDLE_KEY = 'handle';

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

/** The probe of the mathematical boundary: its height is the claim, its width place the first axis' parameter. */
export function probeUpdates(p: Point3, ys: Scale, xs: Scale | null, xId: string | null): DragUpdate[] {
  const out: DragUpdate[] = [];
  const claim = ys.from(p.y);
  if (Number.isFinite(claim) && claim > 0) out.push({ id: 'claim', value: claim });
  if (xs && xId) {
    const x = xs.from(p.x);
    if (Number.isFinite(x)) out.push({ id: xId, value: x });
  }
  return out;
}

/** A claim read as a distance from a centre: the limit sits at `reference` (the chamber's shell, a reference sphere). */
export function radialClaim(distance: number, reference: number, bound: number | null): DragUpdate[] {
  if (bound === null || !(bound > 0) || !(reference > 0) || !Number.isFinite(distance)) return [];
  return [{ id: 'claim', value: bound * Math.max(0.01, distance / reference) }];
}

/** The confinement walls: their distance from the centre is the box half width (inverse of `boxHalfWidth`). */
export function wallPosition(x: number): number {
  return clamp01((Math.abs(x) - 1.5) / 5);
}

/** The aperture: its radius sets its parameter's place (inverse of the optics bench's 0.6 + 1.8 × place). */
export function aperturePosition(radius: number): number {
  return clamp01((Math.abs(radius) - 0.6) / 1.8);
}

/** The hot source, raised or lowered: its parameter's place from its height (1.5 to 4.5, as drawn). */
export function sourcePosition(y: number): number {
  return clamp01((y - 1.5) / 3);
}

/** Height of the hot source for its parameter's place (inverse of `sourcePosition`). */
export function sourceHeight(position: number): number {
  return 1.5 + 3 * clamp01(position);
}

/** Distance of a point from a vertical axis through `centre`, in the floor plane. */
export function radialDistance(p: Point3, centre: Point3): number {
  return Math.hypot(p.x - centre.x, p.z - centre.z);
}
