import { Injectable, computed, inject, signal } from '@angular/core';
import { PipelineInfo } from './analysis.models';
import { AnalysisService } from './analysis.service';

/** Engine status from GET /v1/pipeline: checking, online (with capabilities) or offline. */
@Injectable({ providedIn: 'root' })
export class StatusService {
  private readonly analysis = inject(AnalysisService);
  readonly info = signal<PipelineInfo | null>(null);
  readonly state = signal<'checking' | 'online' | 'offline'>('checking');
  readonly extraction = computed(() => this.info()?.extraction === true);
  readonly vision = computed(() => this.info()?.vision === true);

  constructor() {
    this.refresh();
  }

  refresh(): void {
    this.analysis.pipeline().subscribe({
      next: (i) => {
        this.info.set(i);
        this.state.set('online');
      },
      error: () => {
        this.info.set(null);
        this.state.set('offline');
      },
    });
  }

  /** Called when a request fails with a network error. */
  markOffline(): void {
    this.state.set('offline');
  }
}
