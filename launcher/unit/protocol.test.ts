import { describe, expect, test } from 'vitest';
import {
  impactSchema,
  inspectionSchema,
  launcherStateSchema,
  migrationSchema,
} from '../src/protocol';

const diagnostics = [
  {
    code: 'parse_error',
    severity: 'error',
    file: 'script.rns',
    line: 2,
    column: 5,
    message: 'invalid statement',
    hint: null,
  },
] as const;

describe('Launcher HTTP protocol boundaries', () => {
  test('accepts running and completed jobs in Launcher state', () => {
    const base = {
      sdk: { path: '/sdk', tools: { 'renrs-check': true }, ready: true },
      projects: [{ id: 'story', path: '/story', name: 'Story' }],
    };
    expect(
      launcherStateSchema.parse({
        ...base,
        jobs: [
          { id: 'running', name: 'Check', log: '', code: null, started: '2026-09-08T00:00:00Z' },
          { id: 'done', name: 'Build', log: 'ok', code: 0, started: '2026-09-08T00:01:00Z' },
        ],
      }).jobs,
    ).toHaveLength(2);
    expect(() =>
      launcherStateSchema.parse({ ...base, jobs: [{ id: 'bad', code: 0.5 }] }),
    ).toThrow();
  });

  test('enforces inspection protocol version and nullable envelope fields', () => {
    const failure = {
      protocol_version: 1,
      command: 'inspect',
      ok: false,
      data: null,
      diagnostics,
      error: { code: 'invalid_project', message: 'invalid project' },
    };
    expect(inspectionSchema.parse(failure).data).toBeNull();
    expect(() => inspectionSchema.parse({ ...failure, protocol_version: 2 })).toThrow();
    expect(() => inspectionSchema.parse({ ...failure, error: undefined })).toThrow();
  });

  test('accepts migration reports and their absent state', () => {
    expect(migrationSchema.parse(null)).toBeNull();
    expect(
      migrationSchema.parse({
        issues: [{ code: 'dynamic_python', kind: 'unsupported' }],
        summary: { assumptions: 1, unsupported: 1, by_code: { dynamic_python: 1 } },
      })?.summary.unsupported,
    ).toBe(1);
    expect(() =>
      migrationSchema.parse({ issues: [], summary: { assumptions: -1, unsupported: 0 } }),
    ).toThrow();
  });

  test('validates successful and failed impact envelopes', () => {
    const success = {
      protocol_version: 1,
      command: 'impact',
      ok: true,
      data: {
        changed: false,
        labels: { added: [], removed: [] },
        routes: [],
        endings: [],
        new_unreachable_labels: [],
        save_compatibility: { risk: 'none' },
      },
      diagnostics: [],
      error: null,
    };
    expect(impactSchema.parse(success).data?.changed).toBe(false);
    expect(
      impactSchema.parse({ ...success, ok: false, data: null, error: { code: 'io', message: 'x' } })
        .error?.code,
    ).toBe('io');
    expect(() => impactSchema.parse({ ...success, protocol_version: '1' })).toThrow();
  });
});
