/** Types mirroring crates/limina-api/src/dto.rs. */

export type Verdict = 'IMPOSSIBLE' | 'CONDITIONAL' | 'COMPATIBLE' | 'INSUFFICIENT_DATA';

export interface AssumptionInfo {
  id: string;
  options: string[];
}

export type ParameterKind = 'AbsoluteTemperature' | 'TemperatureDifference' | 'Plain';

export interface ParameterSpec {
  id: string;
  kind: ParameterKind;
  required: boolean;
  /** Allowed unit symbols, from the engine's unit table. */
  units: string[];
}

export interface LimitInfo {
  id: string;
  direction: 'Max' | 'Min' | 'Equal';
  /** Display title, for example "Betz limit". */
  title: string;
  /** The bound as plain text, for example "Cp <= 16/27". */
  formula: string;
  /** One plain sentence stating what the limit says. */
  statement: string;
  /** Canonical unit symbol of the claimed quantity. */
  claim_unit: string;
  assumptions: AssumptionInfo[];
  parameters: ParameterSpec[];
  /** Display label of each assumption id. */
  assumption_labels: Record<string, string>;
  /** Display label of each option id. */
  option_labels: Record<string, string>;
  /** Display label of each parameter id. */
  parameter_labels: Record<string, string>;
  /** Kind of law, for example 	heorem or 	heoretical_bound; absent when unknown. */
  classification?: string;
  /** References of the law. */
  references?: string[];
  /** Where the law applies, in plain text. */
  validity_domain?: string;
  /** Limits of application, in plain text. */
  limitations?: string[];
}

export interface LimitsResponse {
  limits: LimitInfo[];
}

export interface ParameterValue {
  value: number;
  unit: string;
}

export interface EvaluateRequest {
  limit_id: string;
  claim: { value: number; unit: string; uncertainty?: number };
  constraints: Record<string, string[]>;
  parameters: Record<string, ParameterValue>;
}

export interface Margin {
  low: number;
  high: number;
}

export interface Enclosure {
  b_min: number;
  b_max: number;
  margin: Margin | null;
}

export interface Bounds {
  permissive: Enclosure;
  restrictive: Enclosure;
}

export interface Configuration {
  assumptions: Record<string, string>;
  support: 'Certain' | 'Depends';
  depends_on: string[];
}

export interface MissingParameter {
  id: string;
  status: 'NoDomain' | 'HasDomain';
}

export interface EvaluateResponse {
  verdict: Verdict;
  limit_id: string;
  engine_version: string;
  wording: string;
  bounds: Bounds | null;
  configurations: Configuration[];
  missing_parameters: MissingParameter[];
}

/** An error shown to the user: the API error body, or a local failure. */
export interface ApiFailure {
  status: number;
  code: string;
  message: string;
}
