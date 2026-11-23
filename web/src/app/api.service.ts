import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable, catchError, throwError } from 'rxjs';
import {
  ApiFailure,
  EvaluateRequest,
  EvaluateResponse,
  LimitsResponse,
} from './api.models';
import { STRINGS } from './strings';

/** Reads `{error: {code, message}}` from an HTTP error; anything else is a local failure. */
export function toFailure(err: unknown): ApiFailure {
  if (!(err instanceof HttpErrorResponse)) {
    return { status: 0, code: 'UnexpectedResponse', message: STRINGS.errorUnexpected };
  }
  if (err.status === 0) {
    return { status: 0, code: 'NetworkError', message: STRINGS.errorNetwork };
  }
  const detail = (err.error as { error?: { code?: unknown; message?: unknown } } | null)?.error;
  if (detail && typeof detail.code === 'string' && typeof detail.message === 'string') {
    return { status: err.status, code: detail.code, message: detail.message };
  }
  return { status: err.status, code: 'UnexpectedResponse', message: STRINGS.errorUnexpected };
}

/** Thin client for the Limina API. Relative URLs: the dev proxy forwards them. */
@Injectable({ providedIn: 'root' })
export class ApiService {
  private readonly http = inject(HttpClient);

  getLimits(): Observable<LimitsResponse> {
    return this.http
      .get<LimitsResponse>('/v1/limits')
      .pipe(catchError((e) => throwError(() => toFailure(e))));
  }

  evaluate(request: EvaluateRequest): Observable<EvaluateResponse> {
    return this.http
      .post<EvaluateResponse>('/v1/evaluate', request)
      .pipe(catchError((e) => throwError(() => toFailure(e))));
  }
}
