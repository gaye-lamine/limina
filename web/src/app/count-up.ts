import { Directive, ElementRef, effect, inject, input } from '@angular/core';

/** True when the user asks for reduced motion, or when matchMedia is not available. */
export function reducedMotion(): boolean {
  return typeof window === 'undefined' || typeof window.matchMedia !== 'function'
    ? true
    : window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * Counts a headline integer up from its previous value over 600 ms. Under
 * prefers-reduced-motion the final value is written at once.
 */
@Directive({ selector: '[appCountUp]' })
export class CountUp {
  readonly appCountUp = input.required<number>();
  private readonly el = inject<ElementRef<HTMLElement>>(ElementRef);
  private shown = 0;
  private frame = 0;

  constructor() {
    effect(() => {
      const target = this.appCountUp();
      const from = this.shown;
      if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(this.frame);
      if (reducedMotion() || typeof requestAnimationFrame !== 'function' || from === target) {
        this.write(target);
        return;
      }
      const t0 = performance.now();
      const tick = (now: number) => {
        const t = Math.min(1, (now - t0) / 600);
        const eased = 1 - (1 - t) ** 3;
        this.write(Math.round(from + (target - from) * eased));
        if (t < 1) this.frame = requestAnimationFrame(tick);
      };
      this.frame = requestAnimationFrame(tick);
    });
  }

  private write(v: number): void {
    this.shown = v;
    this.el.nativeElement.textContent = String(v);
  }
}
