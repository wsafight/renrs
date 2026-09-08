import { z } from 'zod';

export const jobSchema = z.object({
  id: z.string(),
  name: z.string(),
  log: z.string(),
  code: z.number().int().nullable(),
  started: z.string(),
});

export const projectSchema = z.object({
  id: z.string(),
  path: z.string(),
  name: z.string(),
});

export const sdkSchema = z.object({
  path: z.string(),
  tools: z.record(z.string(), z.boolean()),
  ready: z.boolean(),
});

export const launcherStateSchema = z.object({
  sdk: sdkSchema,
  projects: z.array(projectSchema),
  jobs: z.array(jobSchema),
});

export const scriptsSchema = z.array(z.string());
export const scriptSchema = z.object({ text: z.string(), revision: z.string() });
export const revisionSchema = z.object({ revision: z.string() });

const diagnosticSchema = z.object({
  code: z.string(),
  severity: z.enum(['error', 'warning']),
  file: z.string(),
  line: z.number().int().positive(),
  column: z.number().int().positive(),
  message: z.string(),
  hint: z.string().nullable().optional(),
});

const machineErrorSchema = z.object({ code: z.string(), message: z.string() });
const routeResultSchema = z.object({
  name: z.string(),
  passed: z.boolean(),
  expected_label: z.string().nullable().optional(),
  result: z
    .object({ label: z.string().optional(), visited_instructions: z.number().optional() })
    .nullable()
    .optional(),
  error: z.string().nullable().optional(),
});

const inspectionDataSchema = z.object({
  title: z.string(),
  fingerprint: z.string(),
  routes: z.object({
    configured: z.boolean(),
    passed: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
    results: z.array(routeResultSchema),
    coverage: z.object({
      percent: z.number(),
      uncovered_labels: z.array(z.string()),
    }),
  }),
  labels: z.array(z.object({ statically_reachable: z.boolean() })),
  variables: z.array(
    z.object({
      name: z.string(),
      value_type: z.string(),
      initial_value: z.unknown(),
      persistent: z.boolean(),
      file: z.string(),
      line: z.number().int().positive(),
    }),
  ),
  localization: z.array(
    z.object({
      language: z.string(),
      translated_messages: z.number().int().nonnegative(),
      source_messages: z.number().int().nonnegative(),
      missing: z.array(z.unknown()),
      obsolete: z.array(z.unknown()),
      path: z.string(),
    }),
  ),
});

export const inspectionSchema = z.object({
  protocol_version: z.literal(1),
  command: z.string(),
  ok: z.boolean(),
  data: inspectionDataSchema.nullable(),
  diagnostics: z.array(diagnosticSchema),
  error: machineErrorSchema.nullable(),
});

export const migrationSchema = z
  .object({
    issues: z.array(z.unknown()),
    summary: z.object({
      assumptions: z.number().int().nonnegative(),
      unsupported: z.number().int().nonnegative(),
      by_code: z.record(z.string(), z.number().int().nonnegative()),
    }),
  })
  .nullable();

export const impactSchema = z.object({
  protocol_version: z.literal(1),
  command: z.string(),
  ok: z.boolean(),
  data: z
    .object({
      changed: z.boolean(),
      labels: z.object({ added: z.array(z.string()), removed: z.array(z.string()) }),
      routes: z.array(z.unknown()),
      endings: z.array(z.unknown()),
      new_unreachable_labels: z.array(z.string()),
      save_compatibility: z.object({ risk: z.string() }),
    })
    .nullable(),
  diagnostics: z.array(diagnosticSchema),
  error: machineErrorSchema.nullable(),
});

export const emptySchema = z.object({}).passthrough();

export type LauncherState = z.infer<typeof launcherStateSchema>;
export type Inspection = z.infer<typeof inspectionSchema>;
export type MigrationReport = z.infer<typeof migrationSchema>;
export type Impact = z.infer<typeof impactSchema>;
