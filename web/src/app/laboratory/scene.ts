/**
 * The observatory of the 3D laboratory: renderer, camera, dark lighting and
 * a metric grid from the design tokens, orbit controls (pointer and
 * keyboard), render loop and full release of every WebGL resource.
 * It draws no law yet (phase 1); laws are added as objects of `stage`.
 */
import {
  AmbientLight,
  BufferGeometry,
  CircleGeometry,
  DirectionalLight,
  Fog,
  GridHelper,
  Group,
  HemisphereLight,
  Material,
  Mesh,
  MeshStandardMaterial,
  Object3D,
  PerspectiveCamera,
  Scene,
  Spherical,
  Texture,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { LabTheme } from './theme';

/** Where the camera starts and returns on "Reset view". */
const HOME = new Vector3(14, 10, 16);
/** One keyboard step: 5 degrees of rotation, 10 % of zoom. */
const ROTATE_STEP = Math.PI / 36;
const ZOOM_STEP = 1.1;

/** Whether this browser can create a WebGL context. */
export function webglAvailable(): boolean {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
    // The probe context is released at once: only the observatory keeps one.
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
    return !!gl;
  } catch {
    return false;
  }
}

export class Observatory {
  readonly scene = new Scene();
  /** Objects of the law under study; cleared between laws. */
  readonly stage = new Group();
  private readonly camera: PerspectiveCamera;
  private readonly renderer: WebGLRenderer;
  private readonly controls: OrbitControls;
  private readonly resize: ResizeObserver;
  private frame = 0;
  /** Called on every frame with the time in seconds (animations of the law's objects). */
  onFrame: ((seconds: number) => void) | null = null;
  private disposed = false;

  constructor(
    private readonly host: HTMLElement,
    theme: LabTheme,
    private readonly reducedMotion: boolean,
  ) {
    const { width, height } = this.size();
    this.renderer = new WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.setSize(width, height, false);
    this.renderer.setClearColor(theme.bg, 1);
    this.renderer.domElement.classList.add('lab-canvas');
    host.appendChild(this.renderer.domElement);

    this.camera = new PerspectiveCamera(45, width / height, 0.1, 400);
    this.camera.position.copy(HOME);

    this.scene.background = theme.bg.clone();
    this.scene.fog = new Fog(theme.bg, 40, 120);
    this.addObservatory(theme);
    this.scene.add(this.stage);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = !reducedMotion;
    this.controls.dampingFactor = 0.08;
    this.controls.minDistance = 4;
    this.controls.maxDistance = 80;
    this.controls.maxPolarAngle = Math.PI * 0.49;
    this.controls.target.set(0, 0, 0);
    this.controls.update();

    this.resize = new ResizeObserver(() => this.onResize());
    this.resize.observe(host);
    this.loop();
  }

  /** Dark observatory: a faint sky light, a cool key light, a pedestal and a metric grid. */
  private addObservatory(theme: LabTheme): void {
    this.scene.add(new HemisphereLight(theme.textSecondary, theme.bg, 0.55));
    this.scene.add(new AmbientLight(theme.surface3, 0.6));
    const key = new DirectionalLight(theme.textPrimary, 0.9);
    key.position.set(12, 20, 8);
    this.scene.add(key);
    const rim = new DirectionalLight(theme.accent, 0.35);
    rim.position.set(-14, 8, -10);
    this.scene.add(rim);

    const floor = new Mesh(
      new CircleGeometry(24, 96),
      new MeshStandardMaterial({ color: theme.surface1, roughness: 0.95, metalness: 0.05 }),
    );
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -0.02;
    this.scene.add(floor);
    // Metric grid: 1 unit cells, a coarser grid every 5 units in the accent tone.
    const fine = new GridHelper(40, 40, theme.surface3, theme.surface3);
    const coarse = new GridHelper(40, 8, theme.accent, theme.surface3);
    (coarse.material as Material).transparent = true;
    (coarse.material as Material).opacity = 0.35;
    coarse.position.y = 0.001;
    this.scene.add(fine, coarse);
  }

  private size(): { width: number; height: number } {
    return { width: Math.max(1, this.host.clientWidth), height: Math.max(1, this.host.clientHeight) };
  }

  private onResize(): void {
    const { width, height } = this.size();
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height, false);
  }

  private loop = (): void => {
    if (this.disposed) return;
    this.frame = requestAnimationFrame(this.loop);
    this.controls.update();
    this.onFrame?.(performance.now() / 1000);
    this.renderer.render(this.scene, this.camera);
    // The live GPU resources, twice a second, for the release checks of the end-to-end tests.
    if (this.frame % 30 === 0) {
      this.host.dataset['geometries'] = String(this.renderer.info.memory.geometries);
      this.host.dataset['textures'] = String(this.renderer.info.memory.textures);
    }
  };

  /** Back to the starting view. */
  resetView(): void {
    this.camera.position.copy(HOME);
    this.controls.target.set(0, 0, 0);
    this.controls.update();
  }

  /** Arrow keys rotate around the target, + and − zoom, 0 resets. True when the key was used. */
  onKey(key: string): boolean {
    const offset = this.camera.position.clone().sub(this.controls.target);
    const s = new Spherical().setFromVector3(offset);
    switch (key) {
      case 'ArrowLeft':
        s.theta -= ROTATE_STEP;
        break;
      case 'ArrowRight':
        s.theta += ROTATE_STEP;
        break;
      case 'ArrowUp':
        s.phi = Math.max(0.1, s.phi - ROTATE_STEP);
        break;
      case 'ArrowDown':
        s.phi = Math.min(this.controls.maxPolarAngle, s.phi + ROTATE_STEP);
        break;
      case '+':
      case '=':
        s.radius = Math.max(this.controls.minDistance, s.radius / ZOOM_STEP);
        break;
      case '-':
      case '−':
        s.radius = Math.min(this.controls.maxDistance, s.radius * ZOOM_STEP);
        break;
      case '0':
        this.resetView();
        return true;
      default:
        return false;
    }
    this.camera.position.copy(this.controls.target).add(new Vector3().setFromSpherical(s));
    this.controls.update();
    return true;
  }

  /** Releases every WebGL resource: frame, observer, controls, geometries, materials, textures, renderer, context, canvas. */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    cancelAnimationFrame(this.frame);
    this.resize.disconnect();
    this.controls.dispose();
    this.scene.traverse((o: Object3D) => releaseObject(o));
    this.scene.clear();
    this.renderer.renderLists.dispose();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }
}

/** Disposes the geometry, the materials and their textures of one object. */
export function releaseObject(o: Object3D): void {
  const mesh = o as Partial<Mesh> & { geometry?: BufferGeometry; material?: Material | Material[] };
  mesh.geometry?.dispose();
  const materials = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
  for (const m of materials) {
    for (const value of Object.values(m)) if (value instanceof Texture) value.dispose();
    m.dispose();
  }
}
