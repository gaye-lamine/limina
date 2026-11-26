/**
 * The 3D views of the laboratory (spec laboratory-3d, R-05, R-06): the
 * surface, the corridor and the chamber, the probe and its beam, the
 * crossing wave and the physical skins. Every height comes from the
 * engine's numbers; the verdict itself is written in the status panel.
 */
import {
  AdditiveBlending,
  BoxGeometry,
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  Color,
  DoubleSide,
  EdgesGeometry,
  Float32BufferAttribute,
  Group,
  Line,
  LineBasicMaterial,
  LineDashedMaterial,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  PlaneGeometry,
  RepeatWrapping,
  RingGeometry,
  SRGBColorSpace,
  SphereGeometry,
  Sprite,
  SpriteMaterial,
  Texture,
  TorusGeometry,
  Vector3,
} from 'three';
import { CurvePoint, CurveResponse, SurfaceResponse } from '../analysis.models';
import { LimitInfo } from '../api.models';
import { LabResult } from './lab-model';
import { releaseObject } from './scene';
import { Skin } from './skin';
import { LabTheme } from './theme';
import { Topology } from './topology';
import { CEIL, FLOOR, HALF, Scale, cellPast, coreRatio, heightRange, pastBound, scale, ticks, zoneSpans } from './view-state';

/** An axis of the view: a slider's range and value. */
export interface Axis {
  lo: number;
  hi: number;
  log: boolean;
  value: number;
  /** Axis name and the texts of its two ends, for the discrete marks. */
  name: string;
  loText: string;
  hiText: string;
}

/** Everything one frame of a view needs. */
export interface ViewData {
  topology: Topology;
  skin: Skin;
  direction: LimitInfo['direction'];
  claim: number;
  result: LabResult | null;
  x: Axis | null;
  z: Axis | null;
  curve: CurveResponse | null;
  surface: SurfaceResponse | null;
  /** Name of the height axis and how a height value is written. */
  yName: string;
  yText: (v: number) => string;
  /** The pinned probe, in the same values; null when none. */
  pinned: { x: number | null; z: number | null; claim: number } | null;
}

const CORRIDOR = 2.5;
const SHELL = 3.5;
/** Distance of the axis marks from the edge of the drawn field. */
const AXIS_GAP = 0.7;

/** A tileable diagonal hatch in one colour on transparent ground: the "past the limit" pattern. */
function hatchTexture(color: Color, size = 64, gap = 16, width = 5): Texture {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const g = canvas.getContext('2d');
  if (g) {
    g.strokeStyle = `#${color.getHexString()}`;
    g.lineWidth = width;
    g.lineCap = 'square';
    for (let k = -size; k <= size * 2; k += gap) {
      g.beginPath();
      g.moveTo(k, 0);
      g.lineTo(k + size, size);
      g.stroke();
    }
  }
  const t = new CanvasTexture(canvas);
  t.wrapS = RepeatWrapping;
  t.wrapT = RepeatWrapping;
  t.colorSpace = SRGBColorSpace;
  return t;
}

/** A text label as a sprite, from the theme's text colour (decorative: the page has the text alternative). */
function textSprite(text: string, color: Color, height = 0.42): Sprite {
  const canvas = document.createElement('canvas');
  const g = canvas.getContext('2d');
  const px = 40;
  const font = `500 ${px}px Inter, system-ui, sans-serif`;
  let width = 8;
  if (g) {
    g.font = font;
    width = Math.ceil(g.measureText(text).width) + 16;
  }
  canvas.width = width;
  canvas.height = px + 16;
  if (g) {
    g.font = font;
    g.fillStyle = `#${color.getHexString()}`;
    g.textBaseline = 'middle';
    g.fillText(text, 8, canvas.height / 2);
  }
  const map = new CanvasTexture(canvas);
  map.colorSpace = SRGBColorSpace;
  const sprite = new Sprite(new SpriteMaterial({ map, transparent: true, depthWrite: false }));
  sprite.scale.set((height * canvas.width) / canvas.height, height, 1);
  return sprite;
}

type Tick = (seconds: number) => void;

/** What a draw put in the scene: hatched surface cells past the limit, zones outside the validity domain. */
export interface Drawn {
  hatchedCells: number;
  zones: number;
}

export class LawView {
  private readonly root = new Group();
  private ticks: Tick[] = [];
  private drawn: Drawn = { hatchedCells: 0, zones: 0 };

  constructor(
    stage: Group,
    private readonly theme: LabTheme,
    private readonly reduced: boolean,
  ) {
    stage.add(this.root);
  }

  /** Called on every frame. */
  tick(seconds: number): void {
    if (this.reduced) return;
    for (const t of this.ticks) t(seconds);
  }

  /** Replaces the drawn objects with those of `d`; returns what was drawn, for the page and its tests. */
  draw(d: ViewData): Drawn {
    this.clear();
    const bound = d.result?.bound ?? null;
    if (d.topology === 'chamber') this.chamber(d, bound);
    else this.field(d, bound);
    this.skin(d.skin);
    return { ...this.drawn };
  }

  clear(): void {
    this.root.traverse((o) => releaseObject(o));
    this.root.clear();
    this.ticks = [];
    this.drawn = { hatchedCells: 0, zones: 0 };
  }

  dispose(): void {
    this.clear();
    this.root.removeFromParent();
  }

  // ---------- surface and corridor ----------

  private field(d: ViewData, bound: number | null): void {
    const xs = d.x ? scale(d.x.lo, d.x.hi, -HALF, HALF, d.x.log) : null;
    const zs = d.z ? scale(d.z.lo, d.z.hi, -HALF, HALF, d.z.log) : null;
    const values = d.surface
      ? d.surface.values.flat()
      : [...(d.curve?.points ?? []), ...(d.curve?.lower ?? [])].map((p) => p.decisive);
    const pinnedClaim = d.pinned ? [d.pinned.claim] : [];
    const [lo, hi] = heightRange([...values, ...pinnedClaim], d.claim, bound);
    const ys = scale(lo, hi, FLOOR, CEIL);
    if (d.topology === 'surface' && d.surface && xs && zs) this.surfaceMesh(d.surface, xs, zs, ys, d);
    if (d.topology === 'corridor' && d.curve && xs) {
      this.corridorMesh(d.curve, xs, ys, d.direction);
      this.validityZones(d.curve, xs);
    }
    this.axes(d, xs, zs, ys);
    const px = d.x && xs ? xs.to(d.x.value) : 0;
    const pz = d.z && zs ? zs.to(d.z.value) : 0;
    this.probe(new Vector3(px, ys.to(d.claim), pz), bound === null ? null : ys.to(bound), d.result?.kind === 'impossible', false);
    if (d.pinned) {
      const qx = d.pinned.x !== null && xs ? xs.to(d.pinned.x) : 0;
      const qz = d.pinned.z !== null && zs ? zs.to(d.pinned.z) : 0;
      this.probe(new Vector3(qx, ys.to(d.pinned.claim), qz), null, false, true);
    }
  }

  private surfaceMesh(s: SurfaceResponse, xs: Scale, zs: Scale, ys: Scale, d: ViewData): void {
    const nx = s.x.length;
    const nz = s.y.length;
    const pos = new Float32Array(nx * nz * 3);
    const col = new Float32Array(nx * nz * 3);
    const allowed = this.theme.compatible;
    const past = this.theme.impossible;
    for (let j = 0; j < nz; j++)
      for (let i = 0; i < nx; i++) {
        const v = s.values[j]?.[i] ?? null;
        const k = (j * nx + i) * 3;
        pos[k] = xs.to(s.x[i]);
        pos[k + 1] = v === null ? 0 : ys.to(v);
        pos[k + 2] = zs.to(s.y[j]);
        const c = v !== null && pastBound(d.claim, v, d.direction) ? past : allowed;
        col[k] = c.r;
        col[k + 1] = c.g;
        col[k + 2] = c.b;
      }
    const index: number[] = [];
    const defined = (i: number, j: number) => (s.values[j]?.[i] ?? null) !== null;
    for (let j = 0; j < nz - 1; j++)
      for (let i = 0; i < nx - 1; i++) {
        // A cell with an undefined corner stays a hole.
        if (!defined(i, j) || !defined(i + 1, j) || !defined(i, j + 1) || !defined(i + 1, j + 1)) continue;
        const a = j * nx + i;
        index.push(a, a + nx, a + 1, a + 1, a + nx, a + nx + 1);
      }
    const g = new BufferGeometry();
    g.setAttribute('position', new BufferAttribute(pos, 3));
    g.setAttribute('color', new BufferAttribute(col, 3));
    g.setIndex(index);
    g.computeVertexNormals();
    const mesh = new Mesh(
      g,
      new MeshStandardMaterial({ vertexColors: true, side: DoubleSide, roughness: 0.55, metalness: 0.1, transparent: true, opacity: 0.82 }),
    );
    this.root.add(mesh);
    const wire = new LineSegments(
      new BufferGeometry().setAttribute('position', new BufferAttribute(pos.slice(), 3)).setIndex(index),
      new LineBasicMaterial({ color: this.theme.textSecondary, transparent: true, opacity: 0.12 }),
    );
    this.root.add(wire);
    this.hatchPast(s, pos, d);
  }

  /**
   * The hatched layer over the cells past the limit, just above the surface:
   * the region reads by its pattern, not by its colour alone.
   */
  private hatchPast(s: SurfaceResponse, pos: Float32Array, d: ViewData): void {
    const nx = s.x.length;
    const nz = s.y.length;
    const at = (i: number, j: number) => s.values[j]?.[i] ?? null;
    const index: number[] = [];
    for (let j = 0; j < nz - 1; j++)
      for (let i = 0; i < nx - 1; i++) {
        if (!cellPast([at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1)], d.claim, d.direction)) continue;
        const a = j * nx + i;
        index.push(a, a + nx, a + 1, a + 1, a + nx, a + nx + 1);
      }
    this.drawn.hatchedCells = index.length / 6;
    if (index.length === 0) return;
    const lifted = pos.slice();
    for (let k = 1; k < lifted.length; k += 3) lifted[k] += 0.02;
    const uv = new Float32Array(nx * nz * 2);
    for (let j = 0; j < nz; j++)
      for (let i = 0; i < nx; i++) {
        // About one hatch tile per scene unit, whatever the grid size.
        uv[(j * nx + i) * 2] = lifted[(j * nx + i) * 3] / 1.2;
        uv[(j * nx + i) * 2 + 1] = lifted[(j * nx + i) * 3 + 2] / 1.2;
      }
    const g = new BufferGeometry();
    g.setAttribute('position', new BufferAttribute(lifted, 3));
    g.setAttribute('uv', new BufferAttribute(uv, 2));
    g.setIndex(index);
    const hatch = new Mesh(
      g,
      new MeshBasicMaterial({ map: hatchTexture(this.theme.bg), transparent: true, opacity: 0.75, side: DoubleSide, depthWrite: false }),
    );
    hatch.name = 'past-limit-hatch';
    this.root.add(hatch);
  }

  /** The corridor's zones outside the law's validity domain: hatched, outlined slabs across the corridor. */
  private validityZones(c: CurveResponse, xs: Scale): void {
    const spans = zoneSpans(c.validity_zones, xs);
    this.drawn.zones = spans.length;
    for (const [x0, x1] of spans) {
      const width = x1 - x0;
      const box = new BoxGeometry(width, CEIL + 0.4, CORRIDOR * 2 + 0.4);
      const tex = hatchTexture(this.theme.textSecondary, 64, 12, 3);
      tex.repeat.set(Math.max(1, width / 1.2), (CEIL + 0.4) / 1.2);
      const slab = new Mesh(box, new MeshBasicMaterial({ map: tex, transparent: true, opacity: 0.45, side: DoubleSide, depthWrite: false }));
      slab.position.set((x0 + x1) / 2, (CEIL + 0.4) / 2, 0);
      slab.name = 'outside-validity';
      this.root.add(slab);
      const edges = new LineSegments(new EdgesGeometry(box), new LineBasicMaterial({ color: this.theme.textSecondary, transparent: true, opacity: 0.8 }));
      edges.position.copy(slab.position);
      this.root.add(edges);
    }
  }

  /** Discrete marks on the three axes: a line, five ticks, the ends' values and the axis name. */
  private axes(d: ViewData, xs: Scale | null, zs: Scale | null, ys: Scale): void {
    const color = this.theme.textSecondary;
    const pts: number[] = [];
    const edgeZ = d.topology === 'corridor' ? CORRIDOR + AXIS_GAP : HALF + AXIS_GAP;
    const edgeX = -HALF - AXIS_GAP;
    if (xs && d.x) {
      pts.push(-HALF, 0.02, edgeZ, HALF, 0.02, edgeZ);
      for (const x of ticks(-HALF, HALF)) pts.push(x, 0.02, edgeZ, x, 0.02, edgeZ + 0.3);
      this.label(d.x.loText, new Vector3(-HALF, 0.05, edgeZ + 0.8), color);
      this.label(d.x.hiText, new Vector3(HALF, 0.05, edgeZ + 0.8), color);
      this.label(d.x.name, new Vector3(0, 0.05, edgeZ + 1.4), this.theme.textPrimary);
    }
    if (zs && d.z) {
      const ex = HALF + AXIS_GAP;
      pts.push(ex, 0.02, -HALF, ex, 0.02, HALF);
      for (const z of ticks(-HALF, HALF)) pts.push(ex, 0.02, z, ex + 0.3, 0.02, z);
      this.label(d.z.loText, new Vector3(ex + 1.2, 0.05, -HALF), color);
      this.label(d.z.hiText, new Vector3(ex + 1.2, 0.05, HALF), color);
      this.label(d.z.name, new Vector3(ex + 2.2, 0.05, 0), this.theme.textPrimary);
    }
    const back = d.topology === 'corridor' ? -CORRIDOR - AXIS_GAP : -HALF - AXIS_GAP;
    pts.push(edgeX, FLOOR, back, edgeX, CEIL, back);
    for (const y of ticks(FLOOR, CEIL)) pts.push(edgeX, y, back, edgeX - 0.3, y, back);
    this.label(d.yText(ys.lo), new Vector3(edgeX - 1, FLOOR, back), color);
    this.label(d.yText(ys.hi), new Vector3(edgeX - 1, CEIL, back), color);
    this.label(d.yName, new Vector3(edgeX, CEIL + 0.7, back), this.theme.textPrimary);
    const g = new BufferGeometry().setAttribute('position', new Float32BufferAttribute(pts, 3));
    this.root.add(new LineSegments(g, new LineBasicMaterial({ color, transparent: true, opacity: 0.7 })));
  }

  private label(text: string, at: Vector3, color: Color): void {
    if (!text) return;
    const s = textSprite(text, color);
    s.position.copy(at);
    this.root.add(s);
  }

  private corridorMesh(c: CurveResponse, xs: Scale, ys: Scale, direction: LimitInfo['direction']): void {
    const upper = c.points;
    if (c.kind === 'equality' && c.lower) {
      // A predicted value: the tolerance band between its two edges.
      this.walls(c.lower, upper, xs, ys, this.theme.compatible, 0.3);
      this.ceiling(upper, xs, ys, this.theme.compatible);
      this.ceiling(c.lower, xs, ys, this.theme.compatible);
      return;
    }
    const top = upper.map((p) => ({ ...p, decisive: p.decisive === null ? null : ys.hi }));
    const bottom = upper.map((p) => ({ ...p, decisive: p.decisive === null ? null : ys.lo }));
    // The region past the limit: tinted, then hatched so it reads without colour.
    const [from, to] = direction === 'Min' ? [bottom, upper] : [upper, top];
    this.walls(from, to, xs, ys, this.theme.impossible, 0.16);
    this.walls(from, to, xs, ys, this.theme.impossible, 0.6, hatchTexture(this.theme.impossible, 64, 16, 3));
    this.ceiling(upper, xs, ys, this.theme.accent);
  }

  /** The two side walls of the corridor between two edges, translucent. */
  private walls(lower: CurvePoint[], upper: CurvePoint[], xs: Scale, ys: Scale, color: Color, opacity: number, hatch?: Texture): void {
    for (const z of [-CORRIDOR, CORRIDOR]) {
      const pos: number[] = [];
      const uv: number[] = [];
      const index: number[] = [];
      for (let i = 0; i < upper.length; i++) {
        const a = lower[i]?.decisive ?? null;
        const b = upper[i]?.decisive ?? null;
        const x = xs.to(upper[i].x);
        const ya = a === null ? 0 : ys.to(a);
        const yb = b === null ? 0 : ys.to(b);
        pos.push(x, ya, z, x, yb, z);
        uv.push(x / 1.2, ya / 1.2, x / 1.2, yb / 1.2);
        const ok = a !== null && b !== null && (lower[i + 1]?.decisive ?? null) !== null && (upper[i + 1]?.decisive ?? null) !== null;
        if (i < upper.length - 1 && ok) {
          const k = i * 2;
          index.push(k, k + 2, k + 1, k + 1, k + 2, k + 3);
        }
      }
      const g = new BufferGeometry();
      g.setAttribute('position', new Float32BufferAttribute(pos, 3));
      g.setAttribute('uv', new Float32BufferAttribute(uv, 2));
      g.setIndex(index);
      const material = hatch
        ? new MeshBasicMaterial({ map: z < 0 ? hatch : hatch.clone(), transparent: true, opacity, side: DoubleSide, depthWrite: false })
        : new MeshBasicMaterial({ color, transparent: true, opacity, side: DoubleSide, depthWrite: false });
      this.root.add(new Mesh(g, material));
    }
  }

  /** A horizontal strip across the corridor at the bound: its ceiling or floor. */
  private ceiling(points: CurvePoint[], xs: Scale, ys: Scale, color: Color): void {
    const pos: number[] = [];
    const index: number[] = [];
    points.forEach((p, i) => {
      const y = p.decisive === null ? 0 : ys.to(p.decisive);
      pos.push(xs.to(p.x), y, -CORRIDOR, xs.to(p.x), y, CORRIDOR);
      const next = points[i + 1];
      if (next && p.decisive !== null && next.decisive !== null) {
        const k = i * 2;
        index.push(k, k + 2, k + 1, k + 1, k + 2, k + 3);
      }
    });
    const g = new BufferGeometry();
    g.setAttribute('position', new Float32BufferAttribute(pos, 3));
    g.setIndex(index);
    g.computeVertexNormals();
    this.root.add(
      new Mesh(g, new MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.25, side: DoubleSide, transparent: true, opacity: 0.7 })),
    );
  }

  // ---------- probe, beam and wave ----------

  private probe(at: Vector3, boundY: number | null, crossing: boolean, pinned: boolean): void {
    const color = pinned ? this.theme.textSecondary : this.theme.accent;
    const ball = new Mesh(
      new SphereGeometry(0.28, 24, 16),
      new MeshStandardMaterial({ color, emissive: color, emissiveIntensity: pinned ? 0.2 : 0.9, wireframe: pinned }),
    );
    ball.position.copy(at);
    this.root.add(ball);
    const halo = new Mesh(
      new SphereGeometry(0.5, 24, 16),
      new MeshBasicMaterial({ color, transparent: true, opacity: pinned ? 0.08 : 0.18, blending: AdditiveBlending, depthWrite: false }),
    );
    halo.position.copy(at);
    this.root.add(halo);
    if (!pinned) this.ticks.push((t) => halo.scale.setScalar(1 + 0.12 * Math.sin(t * 3)));
    // The projection: a sharp dashed drop to the floor, a mark there, and its lines to the X and Z axes.
    const foot = new Vector3(at.x, 0.03, at.z);
    this.dashed([at, foot], this.theme.textPrimary, pinned ? 0.45 : 0.85);
    const spot = new Mesh(new RingGeometry(0.16, 0.24, 32), new MeshBasicMaterial({ color: this.theme.textPrimary, side: DoubleSide, transparent: true, opacity: pinned ? 0.5 : 0.9 }));
    spot.rotation.x = -Math.PI / 2;
    spot.position.copy(foot);
    this.root.add(spot);
    if (!pinned) {
      this.dashed([foot, new Vector3(at.x, 0.03, HALF + AXIS_GAP)], this.theme.textSecondary, 0.45);
      this.dashed([foot, new Vector3(HALF + AXIS_GAP, 0.03, at.z)], this.theme.textSecondary, 0.45);
    }
    if (boundY === null) return;
    this.line([at, new Vector3(at.x, boundY, at.z)], crossing ? this.theme.impossible : this.theme.accent, 0.9);
    const mark = new Mesh(new RingGeometry(0.22, 0.32, 32), new MeshBasicMaterial({ color: this.theme.textPrimary, side: DoubleSide }));
    mark.rotation.x = -Math.PI / 2;
    mark.position.set(at.x, boundY, at.z);
    this.root.add(mark);
    if (crossing) this.wave(new Vector3(at.x, boundY, at.z));
  }

  /** The wave of a claim past the limit: rings spreading from where the beam meets it. */
  private wave(at: Vector3): void {
    for (let n = 0; n < 3; n++) {
      const ring = new Mesh(
        new RingGeometry(0.4, 0.48, 48),
        new MeshBasicMaterial({ color: this.theme.impossible, transparent: true, opacity: 0.5, side: DoubleSide, depthWrite: false }),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.position.copy(at);
      ring.scale.setScalar(1 + n * 1.2);
      this.root.add(ring);
      this.ticks.push((t) => {
        const phase = (t * 0.6 + n / 3) % 1;
        ring.scale.setScalar(1 + phase * 4);
        (ring.material as MeshBasicMaterial).opacity = 0.55 * (1 - phase);
      });
    }
  }

  private dashed(points: Vector3[], color: Color, opacity: number): void {
    const l = new Line(
      new BufferGeometry().setFromPoints(points),
      new LineDashedMaterial({ color, transparent: true, opacity, dashSize: 0.22, gapSize: 0.14 }),
    );
    l.computeLineDistances();
    this.root.add(l);
  }

  private line(points: Vector3[], color: Color, opacity: number): void {
    const l = new Line(new BufferGeometry().setFromPoints(points), new LineBasicMaterial({ color, transparent: true, opacity }));
    this.root.add(l);
  }

  // ---------- chamber ----------

  private chamber(d: ViewData, bound: number | null): void {
    const centre = new Vector3(0, SHELL + 0.6, 0);
    const shell = new Mesh(
      new SphereGeometry(SHELL, 48, 24),
      new MeshBasicMaterial({ color: this.theme.accent, wireframe: true, transparent: true, opacity: 0.35 }),
    );
    shell.position.copy(centre);
    this.root.add(shell);
    const r = SHELL * coreRatio(d.claim, bound);
    const past = d.result?.kind === 'impossible';
    const color = past ? this.theme.impossible : this.theme.textPrimary;
    const core = new Mesh(
      new SphereGeometry(1, 40, 24),
      new MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.6, transparent: true, opacity: 0.85 }),
    );
    core.position.copy(centre);
    core.scale.setScalar(r);
    this.root.add(core);
    this.ticks.push((t) => core.scale.setScalar(r * (1 + 0.04 * Math.sin(t * 4))));
    if (d.pinned) {
      const pr = SHELL * coreRatio(d.pinned.claim, bound);
      const ghost = new Mesh(new SphereGeometry(1, 32, 16), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true }));
      ghost.position.copy(centre);
      ghost.scale.setScalar(pr);
      this.root.add(ghost);
    }
    if (past) this.wave(new Vector3(0, 0.05, 0));
  }

  // ---------- skins ----------

  private skin(skin: Skin): void {
    if (skin === 'thermal') this.thermal();
    else if (skin === 'gravitational') this.spacetime();
    else if (skin === 'wave') this.standing();
    else if (skin === 'energy') this.flux();
  }

  /** Incandescent floor: from the dark surface to red, amber and white along the width. */
  private thermal(): void {
    const g = new PlaneGeometry(HALF * 2, 3, 48, 1);
    const stops = [this.theme.surface2, this.theme.impossible, this.theme.conditional, this.theme.textPrimary];
    const pos = g.getAttribute('position');
    const col = new Float32Array(pos.count * 3);
    for (let i = 0; i < pos.count; i++) {
      const t = (pos.getX(i) + HALF) / (HALF * 2);
      const at = Math.min(stops.length - 1.001, t * (stops.length - 1));
      const c = stops[Math.floor(at)].clone().lerp(stops[Math.floor(at) + 1], at - Math.floor(at));
      col.set([c.r, c.g, c.b], i * 3);
    }
    g.setAttribute('color', new BufferAttribute(col, 3));
    const m = new Mesh(g, new MeshBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.35, depthWrite: false }));
    m.rotation.x = -Math.PI / 2;
    m.position.set(0, 0.01, HALF + 2);
    this.root.add(m);
    this.ticks.push((t) => ((m.material as MeshBasicMaterial).opacity = 0.3 + 0.08 * Math.sin(t * 1.5)));
  }

  /** A curved space-time grid under the scene. */
  private spacetime(): void {
    const n = 24;
    const size = HALF * 2.5;
    const pts: number[] = [];
    const y = (x: number, z: number) => -2.2 / (1 + (x * x + z * z) / 12);
    for (let a = 0; a <= n; a++) {
      const u = -size / 2 + (a / n) * size;
      for (let b = 0; b < n; b++) {
        const v0 = -size / 2 + (b / n) * size;
        const v1 = -size / 2 + ((b + 1) / n) * size;
        pts.push(u, y(u, v0), v0, u, y(u, v1), v1, v0, y(v0, u), u, v1, y(v1, u), u);
      }
    }
    const g = new BufferGeometry().setAttribute('position', new Float32BufferAttribute(pts, 3));
    const grid = new LineSegments(g, new LineBasicMaterial({ color: this.theme.accent, transparent: true, opacity: 0.18 }));
    grid.position.y = -0.05;
    this.root.add(grid);
  }

  /** Standing waves behind the scene. */
  private standing(): void {
    for (let n = 1; n <= 3; n++) {
      const count = 160;
      const pos = new Float32Array(count * 3);
      const g = new BufferGeometry().setAttribute('position', new BufferAttribute(pos, 3));
      const shape = (t: number) => {
        for (let i = 0; i < count; i++) {
          const x = -HALF + (i / (count - 1)) * HALF * 2;
          pos[i * 3] = x;
          pos[i * 3 + 1] = 3 + n * 1.4 + 0.6 * Math.sin((n * Math.PI * (x + HALF)) / (HALF * 2)) * Math.cos(t * (1 + n * 0.5));
          pos[i * 3 + 2] = -HALF - 1.5;
        }
        g.getAttribute('position').needsUpdate = true;
      };
      shape(0);
      this.root.add(new Line(g, new LineBasicMaterial({ color: this.theme.accent, transparent: true, opacity: 0.5 - n * 0.1 })));
      this.ticks.push(shape);
    }
  }

  /** Flux rings around the centre of the scene. */
  private flux(): void {
    for (let n = 0; n < 3; n++) {
      const ring = new Mesh(
        new TorusGeometry(HALF * 0.55 + n * 1.3, 0.03, 8, 96),
        new MeshBasicMaterial({ color: this.theme.accent, transparent: true, opacity: 0.4 - n * 0.1 }),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.position.y = 0.05 + n * 0.02;
      this.root.add(ring);
      this.ticks.push((t) => {
        ring.position.y = 0.05 + ((t * 0.5 + n / 3) % 1) * 2;
        (ring.material as MeshBasicMaterial).opacity = 0.35 * (1 - ((t * 0.5 + n / 3) % 1));
      });
    }
  }
}

