/** Types mirroring the analysis, solve, curve and transcribe DTOs of crates/limina-api/src/dto.rs. */
import { EvaluateResponse } from './api.models';

export interface Evidence {
  text: string;
  start: number;
  end: number;
}

export interface AnalyzedParameter {
  value: number | null;
  raw_value: number | null;
  unit: string;
  evidence: Evidence | null;
  flags: string[];
}

export interface TraceStep {
  step: string;
  status: 'done' | 'failed' | 'skipped';
  detail: string;
}

export type LawSource =
  | 'built_in'
  | 'draft_agreed'
  | 'draft_disputed'
  | 'not_authored'
  | 'no_law_identified';

export interface LawAttempt {
  model: string;
  ok: boolean;
  attempts: number;
  errors: string[];
}

export interface Law {
  source: LawSource;
  limit_id: string | null;
  sentence: string | null;
  attempts: LawAttempt[];
  comparison: { samples: number; max_relative_difference: number; reason: string | null } | null;
}

export interface AnalyzedClaim {
  limit_id: string | null;
  quantity: string;
  value: number | null;
  unit: string;
  raw_value: number;
  raw_unit: string;
  evidence: Evidence | null;
  constraints: Record<string, string>;
  parameters: Record<string, AnalyzedParameter>;
  notes: string[];
  flags: string[];
  status: 'evaluated' | 'needs_input' | 'outside_model_domain';
  result: EvaluateResponse | null;
  reason: string | null;
  code: string | null;
  trace: TraceStep[];
  law: Law | null;
}

export interface AnalyzeResponse {
  claims: AnalyzedClaim[];
  missing: string[];
  extraction: { model: string; cached: boolean; ms: number };
}

export interface SolveResponse {
  threshold: number;
  direction: 'at least' | 'at most';
  unit: string;
  sentence: string;
}

export interface CurvePoint {
  x: number;
  b_min: number | null;
  b_max: number | null;
  decisive: number | null;
}

export interface CurveResponse {
  limit_id: string;
  direction: 'Max' | 'Min';
  free_parameter: string;
  range: [number, number];
  points: CurvePoint[];
}

export interface PipelineInfo {
  extraction: boolean;
  law_authoring: boolean;
  vision: boolean;
  models: string[];
}

export interface TranscribeResponse {
  text: string;
  model: string;
}
