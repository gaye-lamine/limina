/**
 * The glow of the laboratory (spec laboratory-3d, R-14): an
 * `UnrealBloomPass` between a `RenderPass` and an `OutputPass`. The
 * threshold lets only bright emissive parts glow (beams, the hot core, the
 * horizon, the crossing wave), not the dark floor; labels are drawn after
 * it, so they stay sharp. Where the context cannot render to floating
 * point targets, there is no glow and the scene is drawn directly.
 */
import { Camera, Scene, Vector2, WebGLRenderer } from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';

/** Calibrated on the dark theme: strength, radius, luminance threshold. */
export const BLOOM = { strength: 0.6, radius: 0.35, threshold: 0.72 } as const;

/** Whether this renderer can hold the half float targets the bloom renders into. */
export function canGlow(renderer: Pick<WebGLRenderer, 'capabilities' | 'getContext'>): boolean {
  try {
    if (!renderer.capabilities.isWebGL2) return false;
    const gl = renderer.getContext();
    return !!(gl.getExtension('EXT_color_buffer_float') ?? gl.getExtension('EXT_color_buffer_half_float'));
  } catch {
    return false;
  }
}

export class Glow {
  private constructor(
    private readonly composer: EffectComposer,
    private readonly bloom: UnrealBloomPass,
    private readonly output: OutputPass,
  ) {}

  /** The glow pipeline, or null when the context cannot hold it. */
  static create(renderer: WebGLRenderer, scene: Scene, camera: Camera, width: number, height: number): Glow | null {
    if (!canGlow(renderer)) return null;
    let composer: EffectComposer | null = null;
    try {
      composer = new EffectComposer(renderer);
      composer.addPass(new RenderPass(scene, camera));
      const bloom = new UnrealBloomPass(new Vector2(width, height), BLOOM.strength, BLOOM.radius, BLOOM.threshold);
      composer.addPass(bloom);
      const output = new OutputPass();
      composer.addPass(output);
      composer.setPixelRatio(renderer.getPixelRatio());
      composer.setSize(width, height);
      return new Glow(composer, bloom, output);
    } catch {
      composer?.dispose();
      return null;
    }
  }

  render(): void {
    this.composer.render();
  }

  setSize(width: number, height: number): void {
    this.composer.setSize(width, height);
  }

  /** Releases the render targets and the materials of the three passes. */
  dispose(): void {
    this.bloom.dispose();
    this.output.dispose();
    this.composer.dispose();
  }
}
