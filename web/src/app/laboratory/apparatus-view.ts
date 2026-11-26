/**
 * The physical apparatus of the laboratory (spec laboratory-3d, R-11): four
 * animated test benches driven by the same values and engine answers as
 * the mathematical boundary. They show the claim's place relative to the
 * engine's limit; the verdict itself is written in the status panel.
 */
import {
  AdditiveBlending,
  BoxGeometry,
  BufferAttribute,
  BufferGeometry,
  Color,
  CylinderGeometry,
  DoubleSide,
  EdgesGeometry,
  Float32BufferAttribute,
  Group,
  Line,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  PlaneGeometry,
  Points,
  PointsMaterial,
  SphereGeometry,
  TorusGeometry,
  Vector3,
} from 'three';
import { LimitInfo } from '../api.models';
import { LAB } from '../strings';
import {
  Apparatus,
  agitation,
  airy,
  beta,
  boxHalfWidth,
  contraction,
  converterSpeed,
  decayFractions,
  flowShares,
  isLength,
  levelOf,
  orbitRadii,
  peakSeparation,
  shellRadii,
  spectrumRgb,
  trajectory,
  undecayed,
  wellDepth,
} from './apparatus';
import { LabResult } from './lab-model';
import { releaseObject } from './scene';
import { LabTheme } from './theme';
import { hatchTexture, textSprite } from './views';

/** Everything one frame of a bench needs; positions are the sliders' places on their tracks (0 to 1). */
export interface ApparatusData {
  kind: Apparatus;
  direction: LimitInfo['direction'];
  claim: number;
  claimUnit: string;
  result: LabResult | null;
  /** Place of the first parameter on its track; 0.5 without parameter. */
  position: number;
  /** Place of the mass or field strength parameter; null when the law has none. */
  strength: number | null;
  /** Value of the discrete level parameter; null when the law has none. */
  level: number | null;
  /** Wavelength in nm of the beam; null when the law gives none. */
  nanometres: number | null;
  /** Whether the law describes a population decaying over time, and its initial count when stated. */
  decay: boolean;
  initial: number | null;
  /** The pinned probe: its claimed over limit and whether the engine found it past the limit; null when none. */
  pinned: { ratio: number | null; impossible: boolean } | null;
}

/** What a bench shows, for the page and its tests. */
export type BenchState = 'flowing' | 'collapsed' | 'escapes' | 'captured' | 'radius' | 'resolved' | 'merged' | 'shells' | 'population' | 'packet';

type Tick = (seconds: number) => void;

export class ApparatusView {
  private readonly root = new Group();
  private ticks: Tick[] = [];

  constructor(
    stage: Group,
    private readonly theme: LabTheme,
    private readonly reduced: boolean,
  ) {
    stage.add(this.root);
  }

  tick(seconds: number): void {
    if (this.reduced) return;
    for (const t of this.ticks) t(seconds);
  }

  draw(d: ApparatusData): BenchState {
    this.clear();
    if (d.kind === 'flow') return this.flow(d);
    if (d.kind === 'trajectory') return this.field(d);
    if (d.kind === 'wave') return this.optics(d);
    return this.quantum(d);
  }

  clear(): void {
    this.root.traverse((o) => releaseObject(o));
    this.root.clear();
    this.ticks = [];
  }

  dispose(): void {
    this.clear();
    this.root.removeFromParent();
  }

  private impossible(d: ApparatusData): boolean {
    return d.result?.kind === 'impossible';
  }

  private label(text: string, at: Vector3, color = this.theme.textPrimary): void {
    const s = textSprite(text, color, 0.5);
    s.position.copy(at);
    this.root.add(s);
  }

  // ---------- energy and heat ----------

  private flow(d: ApparatusData): BenchState {
    const past = this.impossible(d);
    const ratio = d.result?.ratio ?? null;
    const shares = flowShares(ratio, past);
    const hot = new Mesh(
      new CylinderGeometry(1.6, 1.6, 3, 40),
      new MeshStandardMaterial({ color: this.theme.conditional, emissive: this.theme.conditional, emissiveIntensity: 0.45, transparent: true, opacity: 0.85 }),
    );
    hot.position.set(-6.5, 1.5, 0);
    const cold = new Mesh(
      new CylinderGeometry(1.6, 1.6, 3, 40),
      new MeshStandardMaterial({ color: this.theme.accent, emissive: this.theme.accent, emissiveIntensity: 0.25, transparent: true, opacity: 0.7 }),
    );
    cold.position.set(6.5, 1.5, 0);
    this.root.add(hot, cold);
    this.label(LAB.apparatus.source, new Vector3(-6.5, 3.6, 0));
    this.label(LAB.apparatus.sink, new Vector3(6.5, 3.6, 0));

    // The converter: a rotor whose speed follows the claimed over the limit.
    const rotor = new Group();
    const hub = new Mesh(new CylinderGeometry(0.5, 0.5, 1.2, 24), new MeshStandardMaterial({ color: this.theme.textSecondary, metalness: 0.6, roughness: 0.3 }));
    hub.rotation.x = Math.PI / 2;
    rotor.add(hub);
    for (let k = 0; k < 6; k++) {
      const blade = new Mesh(new BoxGeometry(0.22, 1.9, 0.9), new MeshStandardMaterial({ color: this.theme.textPrimary, metalness: 0.4, roughness: 0.4 }));
      blade.position.set(Math.cos((k * Math.PI) / 3) * 1.1, Math.sin((k * Math.PI) / 3) * 1.1, 0);
      blade.rotation.z = (k * Math.PI) / 3 + Math.PI / 2;
      rotor.add(blade);
    }
    rotor.position.set(0, 2, 0);
    this.root.add(rotor);
    const turns = converterSpeed(ratio);
    this.ticks.push((t) => (rotor.rotation.z = -t * turns * Math.PI * 2 * 0.25));
    this.label(LAB.apparatus.converter, new Vector3(0, 4, 0));

    // Particle streams: input along the floor, useful output up, rejected to the sink.
    this.stream([new Vector3(-4.8, 1.5, 0), new Vector3(-1, 2, 0)], Math.round(70 * shares.input), this.theme.conditional, 0.6);
    this.stream([new Vector3(0, 3, 0), new Vector3(0, 7.5, 0)], Math.round(70 * shares.work), this.theme.compatible, 0.5);
    this.label(LAB.apparatus.work, new Vector3(1.6, 7, 0));
    if (shares.rejected > 0.001) {
      this.stream([new Vector3(1, 2, 0), new Vector3(4.8, 1.5, 0)], Math.round(70 * shares.rejected), this.theme.accent, 0.4);
      this.label(LAB.apparatus.rejected, new Vector3(3, 0.6, 1.8), this.theme.textSecondary);
    }
    if (past) this.barrier(new Vector3(3, 2, 0));
    if (d.pinned) {
      const ghost = new Mesh(new TorusGeometry(2.2, 0.04, 8, 64), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true }));
      ghost.position.copy(rotor.position);
      ghost.scale.setScalar(Math.min(1.6, Math.max(0.4, d.pinned.ratio ?? 1)));
      this.root.add(ghost);
    }
    return past ? 'collapsed' : 'flowing';
  }

  /** Particles moving at constant speed along a segment, `count` of them. */
  private stream(path: [Vector3, Vector3], count: number, color: Color, speed: number): void {
    if (count <= 0) return;
    const [a, b] = path;
    const pos = new Float32Array(count * 3);
    const offsets = Array.from({ length: count }, (_, i) => i / count);
    const spread = Array.from({ length: count }, (_, i) => [Math.sin(i * 12.9898) * 0.35, Math.cos(i * 78.233) * 0.35]);
    const place = (t: number) => {
      for (let i = 0; i < count; i++) {
        const f = (offsets[i] + t * speed) % 1;
        pos[i * 3] = a.x + (b.x - a.x) * f + (b.x === a.x ? spread[i][0] : 0);
        pos[i * 3 + 1] = a.y + (b.y - a.y) * f + (b.x !== a.x ? spread[i][1] * 0.6 : 0);
        pos[i * 3 + 2] = a.z + (b.z - a.z) * f + spread[i][1];
      }
      g.getAttribute('position').needsUpdate = true;
    };
    const g = new BufferGeometry().setAttribute('position', new BufferAttribute(pos, 3));
    place(0);
    this.root.add(new Points(g, new PointsMaterial({ color, size: 0.18, transparent: true, opacity: 0.9, blending: AdditiveBlending, depthWrite: false })));
    this.ticks.push(place);
  }

  /** The alert barrier past the limit: a hatched panel and its frame, with its written label. */
  private barrier(at: Vector3): void {
    const panel = new Mesh(
      new PlaneGeometry(3.2, 3.6),
      new MeshBasicMaterial({ map: hatchTexture(this.theme.impossible, 64, 16, 5), transparent: true, opacity: 0.85, side: DoubleSide, depthWrite: false }),
    );
    panel.rotation.y = Math.PI / 2;
    panel.position.copy(at);
    panel.name = 'alert-barrier';
    this.root.add(panel);
    const frame = new LineSegments(
      new BufferGeometry().setAttribute(
        'position',
        new Float32BufferAttribute([0, -1.8, -1.6, 0, 1.8, -1.6, 0, 1.8, -1.6, 0, 1.8, 1.6, 0, 1.8, 1.6, 0, -1.8, 1.6, 0, -1.8, 1.6, 0, -1.8, -1.6], 3),
      ),
      new LineBasicMaterial({ color: this.theme.impossible }),
    );
    frame.position.copy(at);
    this.root.add(frame);
    this.label(LAB.apparatus.barrier, new Vector3(at.x, at.y + 2.4, 0), this.theme.impossible);
    this.ticks.push((t) => ((panel.material as MeshBasicMaterial).opacity = 0.6 + 0.25 * Math.sin(t * 4)));
  }

  // ---------- field, trajectory and relativity ----------

  private field(d: ApparatusData): BenchState {
    const past = this.impossible(d);
    const depth = wellDepth(d.strength ?? d.position);
    const n = 28;
    const size = 20;
    const y = (x: number, z: number) => 4 - depth / (1 + (x * x + z * z) / 6);
    const pts: number[] = [];
    for (let a = 0; a <= n; a++) {
      const u = -size / 2 + (a / n) * size;
      for (let b = 0; b < n; b++) {
        const v0 = -size / 2 + (b / n) * size;
        const v1 = -size / 2 + ((b + 1) / n) * size;
        pts.push(u, y(u, v0), v0, u, y(u, v1), v1, v0, y(v0, u), u, v1, y(v1, u), u);
      }
    }
    this.root.add(
      new LineSegments(
        new BufferGeometry().setAttribute('position', new Float32BufferAttribute(pts, 3)),
        new LineBasicMaterial({ color: this.theme.accent, transparent: true, opacity: 0.35 }),
      ),
    );
    const bodyR = 0.5 + 0.25 * depth;
    const body = new Mesh(new SphereGeometry(bodyR, 40, 24), new MeshStandardMaterial({ color: this.theme.surface3, emissive: this.theme.conditional, emissiveIntensity: 0.25 }));
    body.position.set(0, y(0, 0) + bodyR * 0.2, 0);
    this.root.add(body);
    if (isLength(d.claimUnit)) return this.radius(d, body.position, past);
    if (d.strength !== null) {
      // The horizon sphere: where nothing climbs out of the well.
      const horizon = new Mesh(new SphereGeometry(bodyR * 1.6, 32, 16), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true, transparent: true, opacity: 0.35 }));
      horizon.position.copy(body.position);
      this.root.add(horizon);
    }
    this.label(LAB.apparatus.body, new Vector3(0, body.position.y + bodyR * 2 + 0.6, 0));

    // The probe and its path: it escapes within the limit, is captured past it.
    const escapes = !past;
    const path = trajectory(escapes).map(([px, pz]) => new Vector3(px, y(px, pz) + 0.35, pz));
    this.root.add(
      new Line(new BufferGeometry().setFromPoints(path), new LineBasicMaterial({ color: escapes ? this.theme.compatible : this.theme.impossible, transparent: true, opacity: 0.85 })),
    );
    const probe = new Mesh(new SphereGeometry(0.3, 24, 16), new MeshStandardMaterial({ color: this.theme.accent, emissive: this.theme.accent, emissiveIntensity: 0.9 }));
    // Relativistic squash along the motion for a claimed speed near that of light.
    const squash = contraction(beta(d.claim, d.claimUnit));
    const place = (t: number) => {
      const i = Math.floor(((t * 0.25) % 1) * (path.length - 1));
      const p = path[i];
      const q = path[Math.min(path.length - 1, i + 1)];
      probe.position.copy(p);
      probe.lookAt(q);
      probe.scale.set(1, 1, squash);
    };
    place(this.reduced ? 2 : 0);
    this.root.add(probe);
    this.ticks.push(place);
    this.label(escapes ? LAB.apparatus.escapes : LAB.apparatus.falls, path[path.length - 1].clone().add(new Vector3(0, 1, 0)), escapes ? this.theme.compatible : this.theme.impossible);
    if (d.pinned) {
      const ghost = trajectory(!d.pinned.impossible).map(([px, pz]) => new Vector3(px, y(px, pz) + 0.2, pz));
      this.root.add(new Line(new BufferGeometry().setFromPoints(ghost), new LineBasicMaterial({ color: this.theme.textSecondary, transparent: true, opacity: 0.5 })));
    }
    return escapes ? 'escapes' : 'captured';
  }

  /**
   * A claimed length (a radius, an orbit): a ring at the measured radius
   * around a sphere at the reference radius the engine gives, both scaled
   * by the claimed over the limit.
   */
  private radius(d: ApparatusData, centre: Vector3, past: boolean): BenchState {
    const { measured, reference } = orbitRadii(d.claim, d.result?.bound ?? null);
    const sphere = new Mesh(new SphereGeometry(reference, 40, 20), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true, transparent: true, opacity: 0.4 }));
    sphere.position.copy(centre);
    this.root.add(sphere);
    const color = past ? this.theme.impossible : this.theme.compatible;
    const ring = new Mesh(new TorusGeometry(measured, 0.07, 12, 128), new MeshBasicMaterial({ color }));
    ring.rotation.x = Math.PI / 2;
    ring.position.copy(centre);
    this.root.add(ring);
    // A body on the orbit, and the measured radius as a spoke from the centre.
    const moon = new Mesh(new SphereGeometry(0.22, 20, 12), new MeshStandardMaterial({ color: this.theme.accent, emissive: this.theme.accent, emissiveIntensity: 0.9 }));
    const place = (t: number) => moon.position.set(centre.x + measured * Math.cos(t * 0.6), centre.y, centre.z + measured * Math.sin(t * 0.6));
    place(0);
    this.root.add(moon);
    this.ticks.push(place);
    this.root.add(
      new Line(
        new BufferGeometry().setFromPoints([centre.clone(), centre.clone().add(new Vector3(measured, 0, 0))]),
        new LineBasicMaterial({ color, transparent: true, opacity: 0.9 }),
      ),
    );
    // Outside both the ring and the sphere, and a little above: a sprite is centred, so half its width is added.
    this.label(LAB.apparatus.measuredRadius, centre.clone().add(new Vector3(Math.max(measured, reference) + 2.6, 1.1, 0)), color);
    this.label(LAB.apparatus.referenceRadius, centre.clone().add(new Vector3(0, reference + 0.7, 0)));
    if (d.pinned) {
      const ghost = new Mesh(new TorusGeometry(orbitRadii((d.pinned.ratio ?? 1) * (d.result?.bound ?? 1), d.result?.bound ?? 1).measured, 0.03, 8, 96), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true }));
      ghost.rotation.x = Math.PI / 2;
      ghost.position.copy(centre);
      this.root.add(ghost);
    }
    return 'radius';
  }

  // ---------- waves, beams and optics ----------

  private optics(d: ApparatusData): BenchState {
    const rgb = spectrumRgb(d.nanometres);
    const beam = rgb ? new Color(rgb[0], rgb[1], rgb[2]) : this.theme.accent.clone();
    const emitter = new Mesh(new BoxGeometry(2, 1.4, 1.4), new MeshStandardMaterial({ color: this.theme.surface3, metalness: 0.5, roughness: 0.4 }));
    emitter.position.set(-8, 2, 0);
    this.root.add(emitter);
    this.label(LAB.apparatus.emitter, new Vector3(-8, 3.4, 0));
    const ray = new Mesh(new CylinderGeometry(0.12, 0.12, 5.6, 16), new MeshBasicMaterial({ color: beam, transparent: true, opacity: 0.9, blending: AdditiveBlending }));
    ray.rotation.z = Math.PI / 2;
    ray.position.set(-4.2, 2, 0);
    this.root.add(ray);
    // The lens or aperture, its size from the first parameter's place.
    const r = 0.6 + 1.8 * d.position;
    const lens = new Mesh(new TorusGeometry(r, 0.08, 12, 64), new MeshStandardMaterial({ color: this.theme.textPrimary, metalness: 0.7, roughness: 0.2 }));
    lens.rotation.y = Math.PI / 2;
    lens.position.set(-1.2, 2, 0);
    const glass = new Mesh(new SphereGeometry(r, 32, 16), new MeshBasicMaterial({ color: this.theme.accent, transparent: true, opacity: 0.12, depthWrite: false }));
    glass.scale.set(0.15, 1, 1);
    glass.position.copy(lens.position);
    this.root.add(lens, glass);
    this.label(LAB.apparatus.aperture, new Vector3(-1.2, 2 + r + 0.6, 0));
    // The cone of light to the sensor.
    const cone = new Mesh(new CylinderGeometry(0.05, r, 6, 32, 1, true), new MeshBasicMaterial({ color: beam, transparent: true, opacity: 0.12, side: DoubleSide, depthWrite: false, blending: AdditiveBlending }));
    cone.rotation.z = Math.PI / 2;
    cone.position.set(1.8, 2, 0);
    this.root.add(cone);

    // The sensor: two Airy peaks, merged into one blur past the limit.
    const sep = peakSeparation(d.result?.ratio ?? null, d.direction, this.impossible(d), d.result?.kind === 'compatible');
    this.peaks(sep, beam, new Vector3(7, 0.05, 0), 1);
    if (d.pinned) {
      const pinnedSep = peakSeparation(d.pinned.ratio, d.direction, d.pinned.impossible, false);
      this.peaks(pinnedSep, this.theme.textSecondary, new Vector3(7, 0.05, 0), 0.35, true);
    }
    this.label(LAB.apparatus.sensor, new Vector3(7, 4.2, -3.2));
    return sep < 1 ? 'merged' : 'resolved';
  }

  /** Height field of two Airy patterns `sep` first dark rings apart, on a 4 by 6 sensor. */
  private peaks(sep: number, color: Color, at: Vector3, opacity: number, wire = false): void {
    const width = 0.9;
    const n = 48;
    const g = new PlaneGeometry(4, 6, n, n);
    g.rotateX(-Math.PI / 2);
    const pos = g.getAttribute('position');
    for (let i = 0; i < pos.count; i++) {
      const x = pos.getX(i);
      const z = pos.getZ(i);
      const a = Math.hypot(x, z - (sep * width) / 2);
      const b = Math.hypot(x, z + (sep * width) / 2);
      pos.setY(i, 3 * (airy(a, width) + airy(b, width)) * 0.5);
    }
    g.computeVertexNormals();
    const m = wire
      ? new MeshBasicMaterial({ color, wireframe: true, transparent: true, opacity })
      : new MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.35, side: DoubleSide, transparent: true, opacity: 0.9 * opacity });
    const mesh = new Mesh(g, m);
    mesh.position.copy(at);
    this.root.add(mesh);
  }

  // ---------- quantum confinement and discrete states ----------

  private quantum(d: ApparatusData): BenchState {
    const past = this.impossible(d);
    const glow = past ? this.theme.impossible : this.theme.accent;
    if (d.level !== null) {
      const n = levelOf(d.level);
      const radii = shellRadii(Math.min(8, Math.max(5, n + 1)));
      const centre = new Vector3(0, 4, 0);
      const nucleus = new Mesh(new SphereGeometry(0.25, 24, 16), new MeshStandardMaterial({ color: this.theme.conditional, emissive: this.theme.conditional, emissiveIntensity: 0.8 }));
      nucleus.position.copy(centre);
      this.root.add(nucleus);
      radii.forEach((r, i) => {
        const on = i + 1 === n;
        const ring = new Mesh(new TorusGeometry(r * 0.55, on ? 0.07 : 0.025, 8, 96), new MeshBasicMaterial({ color: on ? glow : this.theme.textSecondary, transparent: true, opacity: on ? 1 : 0.4 }));
        ring.position.copy(centre);
        ring.rotation.x = Math.PI / 2.4;
        this.root.add(ring);
      });
      this.label(`${LAB.apparatus.level} ${n}`, centre.clone().add(new Vector3(0, radii[Math.min(n, radii.length) - 1] * 0.55 + 0.9, 0)));
      if (n > 1) {
        // The transition to the level below: a photon running between the two shells.
        const from = radii[n - 1] * 0.55;
        const to = radii[n - 2] * 0.55;
        const photon = new Mesh(new SphereGeometry(0.18, 16, 12), new MeshBasicMaterial({ color: glow, blending: AdditiveBlending }));
        const place = (t: number) => {
          const f = (t * 0.6) % 1;
          photon.position.set(centre.x + from + (to - from) * f, centre.y, centre.z);
          (photon.material as MeshBasicMaterial).opacity = 1 - f * 0.6;
        };
        place(0.5);
        this.root.add(photon);
        this.ticks.push(place);
      }
      return 'shells';
    }
    if (d.decay) return this.population(d, past);
    // A wave packet between two walls, more agitated in a tighter box.
    const half = boxHalfWidth(d.position);
    for (const x of [-half, half]) {
      const box = new BoxGeometry(0.25, 5, 4);
      const wall = new Mesh(box, new MeshStandardMaterial({ color: this.theme.surface3, transparent: true, opacity: 0.85 }));
      wall.position.set(x, 2.5, 0);
      // A bright outline so the plates stand out on the dark floor.
      const edges = new LineSegments(new EdgesGeometry(box), new LineBasicMaterial({ color: this.theme.accent }));
      edges.position.copy(wall.position);
      this.root.add(wall, edges);
    }
    this.label(LAB.apparatus.box, new Vector3(0, 5.6, 0));
    const amp = agitation(half);
    const count = 200;
    const pos = new Float32Array(count * 3);
    const g = new BufferGeometry().setAttribute('position', new BufferAttribute(pos, 3));
    const shape = (t: number) => {
      for (let i = 0; i < count; i++) {
        const x = -half + (2 * half * i) / (count - 1);
        const envelope = Math.sin((Math.PI * (x + half)) / (2 * half));
        pos[i * 3] = x;
        pos[i * 3 + 1] = 2.5 + amp * 0.5 * envelope * Math.cos((6 * Math.PI * x) / half - t * 4) * Math.cos(t * 2);
        pos[i * 3 + 2] = 0;
      }
      g.getAttribute('position').needsUpdate = true;
    };
    shape(0.3);
    this.root.add(new Line(g, new LineBasicMaterial({ color: glow })));
    this.ticks.push(shape);
    if (d.pinned) {
      const ghost = new Mesh(new BoxGeometry(2 * half, 0.02, 4), new MeshBasicMaterial({ color: this.theme.textSecondary, wireframe: true }));
      ghost.position.set(0, 2.5, 0);
      this.root.add(ghost);
    }
    return 'packet';
  }

  /**
   * A decaying population: a lattice of nuclei, the lit share being the
   * claimed remaining over the initial count; around it, a ring whose arc
   * is the fraction the law expects and an inner arc for the claim.
   */
  private population(d: ApparatusData, past: boolean): BenchState {
    const side = 7;
    const count = side ** 3;
    const { shown, expected } = decayFractions(d.claim, d.result?.bound ?? null, d.initial);
    const lit = undecayed(count, shown);
    const centre = new Vector3(0, 3.2, 0);
    const on: number[] = [];
    const off: number[] = [];
    for (let i = 0; i < count; i++) {
      const x = (i % side) - (side - 1) / 2;
      const y = (Math.floor(i / side) % side) - (side - 1) / 2;
      const z = Math.floor(i / side ** 2) - (side - 1) / 2;
      (lit[i] ? on : off).push(centre.x + x * 0.55, centre.y + y * 0.55, centre.z + z * 0.55);
    }
    const glow = past ? this.theme.impossible : this.theme.conditional;
    const cloud = (pts: number[], color: Color, size: number, opacity: number, additive: boolean) => {
      if (pts.length === 0) return null;
      const p = new Points(
        new BufferGeometry().setAttribute('position', new Float32BufferAttribute(pts, 3)),
        new PointsMaterial({ color, size, transparent: true, opacity, depthWrite: false, blending: additive ? AdditiveBlending : undefined }),
      );
      this.root.add(p);
      return p;
    };
    const bright = cloud(on, glow, 0.32, 1, true);
    cloud(off, this.theme.textSecondary, 0.16, 0.45, false);
    if (bright) this.ticks.push((t) => ((bright.material as PointsMaterial).opacity = 0.8 + 0.2 * Math.sin(t * 3)));
    this.label(`${LAB.apparatus.remaining} ${Math.round(shown * 100)} %`, centre.clone().add(new Vector3(0, 2.8, 0)), glow);

    // The rings: a faint full circle, the expected fraction as an arc, the claimed one inside it.
    const ringY = 0.08;
    const full = new Mesh(new TorusGeometry(4.2, 0.03, 8, 128), new MeshBasicMaterial({ color: this.theme.textSecondary, transparent: true, opacity: 0.35 }));
    full.rotation.x = Math.PI / 2;
    full.position.set(0, ringY, 0);
    this.root.add(full);
    if (expected !== null && expected > 0) {
      const arc = new Mesh(new TorusGeometry(4.2, 0.09, 8, 128, Math.PI * 2 * expected), new MeshBasicMaterial({ color: this.theme.textPrimary }));
      arc.rotation.x = Math.PI / 2;
      arc.position.set(0, ringY, 0);
      this.root.add(arc);
      this.label(LAB.apparatus.expected, new Vector3(0, ringY + 0.6, 4.9));
    }
    if (shown > 0) {
      const claimed = new Mesh(new TorusGeometry(3.7, 0.09, 8, 128, Math.PI * 2 * shown), new MeshBasicMaterial({ color: glow }));
      claimed.rotation.x = Math.PI / 2;
      claimed.position.set(0, ringY, 0);
      this.root.add(claimed);
    }
    if (d.pinned) {
      const pinnedShown = decayFractions((d.pinned.ratio ?? 1) * (d.result?.bound ?? 0), d.result?.bound ?? null, d.initial).shown;
      if (pinnedShown > 0) {
        const ghost = new Mesh(new TorusGeometry(3.3, 0.04, 8, 128, Math.PI * 2 * pinnedShown), new MeshBasicMaterial({ color: this.theme.textSecondary }));
        ghost.rotation.x = Math.PI / 2;
        ghost.position.set(0, ringY, 0);
        this.root.add(ghost);
      }
    }
    return 'population';
  }
}
