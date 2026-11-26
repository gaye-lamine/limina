import { A11yModule } from '@angular/cdk/a11y';
import { Overlay, OverlayRef } from '@angular/cdk/overlay';
import { ComponentPortal } from '@angular/cdk/portal';
import { Component, ElementRef, HostListener, inject, signal, viewChild } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { NavigationEnd, Router, RouterOutlet } from '@angular/router';
import { filter, map } from 'rxjs';
import { LucideAngularModule, X } from 'lucide-angular';
import { ApiFailure, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { LimitsList } from './limits-list';
import { Command, Palette } from './palette';
import { entryTitle, verdictKind, verdictText } from './review-logic';
import { StatusService } from './status.service';
import { STRINGS, UI } from './strings';
import { ToastService } from './toast.service';
import { Workbench } from './workbench';

/**
 * The shell: the top bar (wordmark, engine status, Library, How it works,
 * the Ctrl+K hint), the analysis screen, the Library and How it works
 * drawer, the command palette and the toasts.
 */
@Component({
  selector: 'app-root',
  imports: [LimitsList, Workbench, LucideAngularModule, A11yModule, RouterOutlet],
  templateUrl: './app.html',
})
export class App {
  protected readonly s = STRINGS;
  protected readonly u = UI;
  protected readonly XIcon = X;
  protected readonly status = inject(StatusService);
  protected readonly toasts = inject(ToastService);
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly limitsFailure = signal<ApiFailure | null>(null);
  protected readonly drawerOpen = signal(false);
  protected readonly drawerView = signal<'library' | 'how'>('library');
  private readonly drawer = viewChild<ElementRef<HTMLElement>>('drawer');
  private readonly workbench = viewChild(Workbench);
  private readonly api = inject(ApiService);
  private readonly overlay = inject(Overlay);
  private paletteRef: OverlayRef | null = null;
  private readonly router = inject(Router);
  /** Whether the 3D laboratory is shown; the analysis stays mounted (and kept) behind it. */
  protected readonly inLab = toSignal(
    this.router.events.pipe(
      filter((e): e is NavigationEnd => e instanceof NavigationEnd),
      map((e) => e.urlAfterRedirects.startsWith('/laboratory')),
    ),
    { initialValue: this.router.url.startsWith('/laboratory') },
  );

  /** The Laboratory link: an ordinary link, routed without a page reload. */
  protected openLab(event: MouseEvent): void {
    if (event.ctrlKey || event.metaKey || event.shiftKey || event.button !== 0) return;
    event.preventDefault();
    void this.router.navigateByUrl('/laboratory');
  }

  constructor() {
    this.api.getLimits().subscribe({
      next: (r) => this.limits.set(r.limits),
      error: (f: ApiFailure) => this.limitsFailure.set(f),
    });
  }

  protected openDrawer(view: 'library' | 'how' = 'library'): void {
    this.drawerView.set(view);
    this.drawerOpen.set(true);
    this.api.getLimits().subscribe({
      next: (r) => {
        this.limits.set(r.limits);
        this.limitsFailure.set(null);
      },
      error: () => undefined,
    });
    setTimeout(() => this.drawer()?.nativeElement.focus());
  }

  protected closeDrawer(): void {
    this.drawerOpen.set(false);
  }

  @HostListener('document:keydown.escape')
  protected onEscape(): void {
    this.closeDrawer();
  }

  @HostListener('document:keydown', ['$event'])
  protected onKey(e: KeyboardEvent): void {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      if (this.paletteRef) this.closePalette();
      else this.openPalette();
    }
  }

  /** The commands: every claim of the review, then the actions. */
  protected commands(): Command[] {
    const wb = this.workbench();
    const out: Command[] = [];
    if (wb) {
      wb.entries().forEach((e, i) => out.push({ id: `claim:${i}`, group: 'claims', label: `${i + 1}. ${entryTitle(e)}`, hint: verdictText(e), kind: verdictKind(e) }));
    }
    out.push({ id: 'library', group: 'actions', label: UI.actionLibrary });
    out.push({ id: 'how', group: 'actions', label: UI.actionHow });
    if (wb?.state() === 'reviewed' && wb.entries().length) {
      out.push({ id: 'download', group: 'actions', label: UI.actionDownload });
      out.push({ id: 'trace', group: 'actions', label: UI.actionTrace });
      out.push({ id: 'new', group: 'actions', label: UI.actionNew });
    }
    return out;
  }

  openPalette(): void {
    if (this.paletteRef) return;
    const ref = this.overlay.create({
      hasBackdrop: true,
      backdropClass: 'palette-backdrop',
      panelClass: 'palette-pane',
      positionStrategy: this.overlay.position().global().centerHorizontally().top('14vh'),
      scrollStrategy: this.overlay.scrollStrategies.block(),
    });
    const cmp = ref.attach(new ComponentPortal(Palette));
    cmp.setInput('commands', this.commands());
    cmp.instance.run.subscribe((id) => this.runCommand(id));
    cmp.instance.closed.subscribe(() => this.closePalette());
    ref.backdropClick().subscribe(() => this.closePalette());
    this.paletteRef = ref;
  }

  closePalette(): void {
    this.paletteRef?.dispose();
    this.paletteRef = null;
  }

  protected runCommand(id: string): void {
    this.closePalette();
    const wb = this.workbench();
    if (id.startsWith('claim:') && wb) wb.select(Number(id.slice(6)));
    else if (id === 'library') this.openDrawer('library');
    else if (id === 'how') this.openDrawer('how');
    else if (id === 'download') wb?.download();
    else if (id === 'trace') wb?.toggleHow();
    else if (id === 'new') wb?.newAnalysis();
  }
}
