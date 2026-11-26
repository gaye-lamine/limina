import { Injectable, signal } from '@angular/core';

export interface Toast {
  id: number;
  kind: 'error' | 'info';
  text: string;
}

/** Short messages shown in the corner; errors stay until dismissed, info leaves after 4 s. */
@Injectable({ providedIn: 'root' })
export class ToastService {
  readonly toasts = signal<Toast[]>([]);
  private next = 1;

  show(text: string, kind: Toast['kind'] = 'info'): void {
    const id = this.next++;
    this.toasts.update((t) => [...t.slice(-3), { id, kind, text }]);
    if (kind === 'info') setTimeout(() => this.dismiss(id), 4000);
  }

  dismiss(id: number): void {
    this.toasts.update((t) => t.filter((x) => x.id !== id));
  }
}
