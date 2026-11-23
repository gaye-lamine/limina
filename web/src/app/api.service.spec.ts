import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApiService } from './api.service';
import { ApiFailure, EvaluateRequest, EvaluateResponse, LimitsResponse } from './api.models';

const request: EvaluateRequest = {
  limit_id: 'betz',
  claim: { value: 0.7, unit: 'dimensionless' },
  constraints: { reference_area: ['unshrouded_rotor'] },
  parameters: {},
};

describe('ApiService', () => {
  let service: ApiService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(ApiService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('gets the limits from /v1/limits', () => {
    const body: LimitsResponse = {
      limits: [
        {
          id: 'betz',
          direction: 'Max',
          title: 'Betz limit',
          formula: 'Cp <= 16/27',
          statement: 'A statement.',
          claim_unit: 'dimensionless',
          assumptions: [],
          parameters: [],
          assumption_labels: {},
          option_labels: {},
          parameter_labels: {},
        },
      ],
    };
    let received: LimitsResponse | undefined;
    service.getLimits().subscribe((r) => (received = r));
    const req = http.expectOne('/v1/limits');
    expect(req.request.method).toBe('GET');
    req.flush(body);
    expect(received).toEqual(body);
  });

  it('posts the request to /v1/evaluate and returns the response unchanged', () => {
    const body: EvaluateResponse = {
      verdict: 'IMPOSSIBLE',
      limit_id: 'betz',
      engine_version: '0.1.0',
      wording: 'inconsistent with limit betz under assumptions reference_area = unshrouded_rotor',
      bounds: null,
      configurations: [],
      missing_parameters: [],
    };
    let received: EvaluateResponse | undefined;
    service.evaluate(request).subscribe((r) => (received = r));
    const req = http.expectOne('/v1/evaluate');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(request);
    req.flush(body);
    expect(received).toEqual(body);
  });

  it('maps a 422 body to an ApiFailure with its code and message', () => {
    let failure: ApiFailure | undefined;
    service.evaluate(request).subscribe({ error: (e: ApiFailure) => (failure = e) });
    http
      .expectOne('/v1/evaluate')
      .flush(
        { error: { code: 'NotApplicable', message: 'limit betz is not applicable' } },
        { status: 422, statusText: 'Unprocessable Entity' },
      );
    expect(failure).toEqual({
      status: 422,
      code: 'NotApplicable',
      message: 'limit betz is not applicable',
    });
  });

  it('maps a 400 body to an ApiFailure', () => {
    let failure: ApiFailure | undefined;
    service.evaluate(request).subscribe({ error: (e: ApiFailure) => (failure = e) });
    http
      .expectOne('/v1/evaluate')
      .flush(
        { error: { code: 'InvalidJson', message: 'invalid request body: x' } },
        { status: 400, statusText: 'Bad Request' },
      );
    expect(failure?.status).toBe(400);
    expect(failure?.code).toBe('InvalidJson');
  });

  it('reports NetworkError when the server cannot be reached', () => {
    let failure: ApiFailure | undefined;
    service.getLimits().subscribe({ error: (e: ApiFailure) => (failure = e) });
    http.expectOne('/v1/limits').error(new ProgressEvent('error'));
    expect(failure?.status).toBe(0);
    expect(failure?.code).toBe('NetworkError');
  });

  it('reports UnexpectedResponse when the error body has no error shape', () => {
    let failure: ApiFailure | undefined;
    service.evaluate(request).subscribe({ error: (e: ApiFailure) => (failure = e) });
    http.expectOne('/v1/evaluate').flush('boom', { status: 500, statusText: 'Server Error' });
    expect(failure?.status).toBe(500);
    expect(failure?.code).toBe('UnexpectedResponse');
  });
});
