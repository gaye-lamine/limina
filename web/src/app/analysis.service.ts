import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable, catchError, throwError } from 'rxjs';
import {
  AnalyzeResponse,
  CurveResponse,
  PipelineInfo,
  SolveResponse,
  TranscribeResponse,
} from './analysis.models';
import { ParameterValue } from './api.models';
import { toFailure } from './api.service';

export interface CurveRequest {
  limit_id: string;
  constraints: Record<string, string[]>;
  parameters: Record<string, ParameterValue>;
  free_parameter: string;
  range?: [number, number];
  points?: number;
}

export interface SolveRequest {
  limit_id: string;
  claim: { value: number; unit: string };
  constraints: Record<string, string[]>;
  parameters: Record<string, ParameterValue>;
  free_parameter: string;
}

/** Client of the analysis pipeline. Every number it returns is computed by the API. */
@Injectable({ providedIn: 'root' })
export class AnalysisService {
  private readonly http = inject(HttpClient);

  private post<T>(url: string, body: unknown): Observable<T> {
    return this.http.post<T>(url, body).pipe(catchError((e) => throwError(() => toFailure(e))));
  }

  pipeline(): Observable<PipelineInfo> {
    return this.http
      .get<PipelineInfo>('/v1/pipeline')
      .pipe(catchError((e) => throwError(() => toFailure(e))));
  }

  analyze(text: string): Observable<AnalyzeResponse> {
    return this.post<AnalyzeResponse>('/v1/analyze', { text });
  }

  solve(request: SolveRequest): Observable<SolveResponse> {
    return this.post<SolveResponse>('/v1/solve', request);
  }

  curve(request: CurveRequest): Observable<CurveResponse> {
    return this.post<CurveResponse>('/v1/curve', request);
  }

  transcribe(mime: string, base64: string): Observable<TranscribeResponse> {
    return this.post<TranscribeResponse>('/v1/transcribe', { mime, base64 });
  }
}
