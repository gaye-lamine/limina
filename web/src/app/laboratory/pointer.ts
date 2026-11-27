/**
 * Direct manipulation of the laboratory's handles (spec laboratory-3d,
 * R-13): a raycast picks the first object carrying a handle, the pointer
 * then moves in the handle's plane and its map turns each point into
 * slider updates. The orbit is off during a drag; the cursor shows grab
 * and grabbing. The keyboard controls are not touched.
 */
import { Camera, Object3D, Plane, Raycaster, Vector2, Vector3 } from 'three';
import { DragUpdate, HANDLE_KEY, Handle } from './drag-map';

/** The handle an object or one of its ancestors carries. */
export function handleOf(o: Object3D | null): Handle | null {
  for (let x = o; x; x = x.parent) {
    const h = x.userData?.[HANDLE_KEY] as Handle | undefined;
    if (h && typeof h.map === 'function') return h;
  }
  return null;
}

/** The plane a handle at `at` moves in: horizontal at its height, or vertical facing the camera. */
export function dragPlane(handle: Pick<Handle, 'plane'>, at: Vector3, camera: Camera): Plane {
  if (handle.plane === 'horizontal') return new Plane(new Vector3(0, 1, 0), -at.y);
  const facing = camera.getWorldDirection(new Vector3());
  facing.y = 0;
  if (facing.lengthSq() < 1e-9) facing.set(0, 0, -1);
  facing.normalize();
  return new Plane().setFromNormalAndCoplanarPoint(facing, at);
}

interface Drag {
  handle: Handle;
  plane: Plane;
  pointerId: number;
}

export class DragController {
  private readonly ray = new Raycaster();
  private drag: Drag | null = null;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly camera: Camera,
    private readonly root: Object3D,
    private readonly setOrbit: (on: boolean) => void,
    private readonly onDrag: (updates: DragUpdate[]) => void,
  ) {
    // Registered before the orbit controls, so a press on a handle turns the orbit off first.
    canvas.addEventListener('pointerdown', this.down);
    canvas.addEventListener('pointermove', this.move);
    canvas.addEventListener('pointerup', this.up);
    canvas.addEventListener('pointercancel', this.up);
    canvas.addEventListener('pointerleave', this.leave);
  }

  /** Whether a handle is being dragged. */
  get dragging(): boolean {
    return this.drag !== null;
  }

  private cast(event: PointerEvent): { handle: Handle; point: Vector3 } | null {
    const r = this.canvas.getBoundingClientRect();
    const ndc = new Vector2(((event.clientX - r.left) / r.width) * 2 - 1, -((event.clientY - r.top) / r.height) * 2 + 1);
    this.ray.setFromCamera(ndc, this.camera);
    for (const hit of this.ray.intersectObject(this.root, true)) {
      const handle = handleOf(hit.object);
      if (handle) return { handle, point: hit.point.clone() };
    }
    return null;
  }

  private readonly down = (event: PointerEvent): void => {
    if (event.button !== 0) return;
    const hit = this.cast(event);
    if (!hit) return;
    this.drag = { handle: hit.handle, plane: dragPlane(hit.handle, hit.point, this.camera), pointerId: event.pointerId };
    this.setOrbit(false);
    this.canvas.setPointerCapture?.(event.pointerId);
    this.canvas.style.cursor = 'grabbing';
    event.preventDefault();
  };

  private readonly move = (event: PointerEvent): void => {
    if (!this.drag) {
      this.canvas.style.cursor = this.cast(event) ? 'grab' : '';
      return;
    }
    const r = this.canvas.getBoundingClientRect();
    const ndc = new Vector2(((event.clientX - r.left) / r.width) * 2 - 1, -((event.clientY - r.top) / r.height) * 2 + 1);
    this.ray.setFromCamera(ndc, this.camera);
    const point = this.ray.ray.intersectPlane(this.drag.plane, new Vector3());
    if (!point) return;
    const updates = this.drag.handle.map({ x: point.x, y: point.y, z: point.z });
    if (updates.length > 0) this.onDrag(updates);
  };

  private readonly up = (event: PointerEvent): void => {
    if (!this.drag) return;
    this.canvas.releasePointerCapture?.(event.pointerId);
    this.drag = null;
    this.setOrbit(true);
    this.canvas.style.cursor = this.cast(event) ? 'grab' : '';
  };

  private readonly leave = (): void => {
    if (!this.drag) this.canvas.style.cursor = '';
  };

  /** Screen places of the handles (CSS pixels in the canvas), for the end-to-end checks. */
  handlesOnScreen(): { name: string; x: number; y: number }[] {
    const r = this.canvas.getBoundingClientRect();
    const out: { name: string; x: number; y: number }[] = [];
    this.root.traverse((o) => {
      const h = o.userData?.[HANDLE_KEY] as Handle | undefined;
      if (!h) return;
      const p = o.getWorldPosition(new Vector3()).project(this.camera);
      if (p.z > 1) return;
      out.push({ name: h.name, x: ((p.x + 1) / 2) * r.width, y: ((1 - p.y) / 2) * r.height });
    });
    return out;
  }

  dispose(): void {
    this.canvas.removeEventListener('pointerdown', this.down);
    this.canvas.removeEventListener('pointermove', this.move);
    this.canvas.removeEventListener('pointerup', this.up);
    this.canvas.removeEventListener('pointercancel', this.up);
    this.canvas.removeEventListener('pointerleave', this.leave);
    if (this.drag) this.setOrbit(true);
    this.drag = null;
  }
}
