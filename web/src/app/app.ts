import { Component, ElementRef, HostListener, inject, signal, viewChild } from '@angular/core';
import { ApiFailure, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { LimitsList } from './limits-list';
import { STRINGS, WB } from './strings';
import { Workbench } from './workbench';

@Component({
  selector: 'app-root',
  imports: [LimitsList, Workbench],
  templateUrl: './app.html',
})
export class App {
  protected readonly s = STRINGS;
  protected readonly wb = WB;
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly limitsFailure = signal<ApiFailure | null>(null);
  protected readonly drawerOpen = signal(false);
  private readonly drawer = viewChild<ElementRef<HTMLElement>>('drawer');
  private readonly api = inject(ApiService);

  constructor() {
    this.api.getLimits().subscribe({
      next: (r) => this.limits.set(r.limits),
      error: (f: ApiFailure) => this.limitsFailure.set(f),
    });
  }

  protected openDrawer(): void {
    this.drawerOpen.set(true);
    this.api.getLimits().subscribe({ next: (r) => this.limits.set(r.limits), error: () => undefined });
    setTimeout(() => this.drawer()?.nativeElement.focus());
  }

  protected closeDrawer(): void {
    this.drawerOpen.set(false);
  }

  @HostListener('document:keydown.escape')
  protected onEscape(): void {
    this.closeDrawer();
  }
}
