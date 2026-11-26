import {
  AfterViewInit,
  Component,
  ElementRef,
  NgZone,
  OnDestroy,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { Router } from '@angular/router';
import { LimitInfo } from '../api.models';
import { ApiService } from '../api.service';
import { LAB } from '../strings';
import { Observatory, webglAvailable } from './scene';
import { readTheme } from './theme';

/**
 * The 3D laboratory (spec laboratory-3d), loaded lazily on `/laboratory`.
 * Phase 1: the observatory scene and its status panel; every law comes
 * from `GET /v1/limits`, none is written here.
 */
@Component({
  selector: 'app-laboratory',
  templateUrl: './laboratory.html',
})
export class Laboratory implements AfterViewInit, OnDestroy {
  protected readonly t = LAB;
  private readonly api = inject(ApiService);
  private readonly router = inject(Router);
  private readonly zone = inject(NgZone);
  private readonly host = viewChild.required<ElementRef<HTMLElement>>('stage');
  private observatory: Observatory | null = null;

  protected readonly limits = signal<LimitInfo[] | null>(null);
  protected readonly failed = signal(false);
  protected readonly viewReady = signal(false);
  protected readonly noView = signal(false);
  protected readonly builtIn = computed(() => (this.limits() ?? []).filter((l) => l.status !== 'draft').length);
  protected readonly drafts = computed(() => (this.limits() ?? []).filter((l) => l.status === 'draft').length);

  constructor() {
    this.api.getLimits().subscribe({
      next: (r) => this.limits.set(r.limits),
      error: () => this.failed.set(true),
    });
  }

  ngAfterViewInit(): void {
    if (!webglAvailable()) {
      this.noView.set(true);
      return;
    }
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    // The render loop runs outside Angular: no change detection per frame.
    this.zone.runOutsideAngular(() => {
      this.observatory = new Observatory(this.host().nativeElement, readTheme(), reduced);
    });
    this.viewReady.set(true);
  }

  protected onKey(event: KeyboardEvent): void {
    if (this.observatory?.onKey(event.key)) event.preventDefault();
  }

  protected resetView(): void {
    this.observatory?.resetView();
  }

  protected back(): void {
    void this.router.navigateByUrl('/');
  }

  ngOnDestroy(): void {
    this.observatory?.dispose();
    this.observatory = null;
  }
}
