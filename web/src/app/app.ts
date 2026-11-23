import { Component, inject, signal } from '@angular/core';
import { ApiFailure, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { EvaluateForm } from './evaluate-form';
import { LimitsList } from './limits-list';
import { STRINGS } from './strings';

@Component({
  selector: 'app-root',
  imports: [EvaluateForm, LimitsList],
  templateUrl: './app.html',
})
export class App {
  protected readonly s = STRINGS;
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly limitsFailure = signal<ApiFailure | null>(null);

  constructor() {
    inject(ApiService)
      .getLimits()
      .subscribe({
        next: (r) => this.limits.set(r.limits),
        error: (f: ApiFailure) => this.limitsFailure.set(f),
      });
  }
}
