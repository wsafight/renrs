import '../style.css';
import {
  CheckCheck,
  createIcons,
  FolderMinus,
  FolderOpen,
  GitCompareArrows,
  Import,
  Package,
  Play,
  Plus,
  RefreshCw,
  RotateCw,
  Save,
  Settings2,
  Workflow,
  X,
} from 'lucide';
import type { ZodType } from 'zod';
import type { Impact, Inspection, LauncherState, MigrationReport } from './protocol';
import {
  emptySchema,
  impactSchema,
  inspectionSchema,
  jobSchema,
  launcherStateSchema,
  migrationSchema,
  projectSchema,
  revisionSchema,
  scriptSchema,
  scriptsSchema,
  sdkSchema,
} from './protocol';

type LauncherElement = HTMLElement & {
  disabled: boolean;
  src: string;
  value: string;
  close(): void;
  showModal(): void;
};
type FormValues = Record<string, string>;
type AsyncAction<Args extends unknown[] = []> = (...args: Args) => unknown | Promise<unknown>;

const $ = (id: string): LauncherElement => {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing required element #${id}`);
  return element as LauncherElement;
};

const progress = (id: string): HTMLProgressElement => $(id) as unknown as HTMLProgressElement;

const node = <K extends keyof HTMLElementTagNameMap>(
  tag: K,
  text?: string | null,
  attributes: Partial<HTMLElementTagNameMap[K]> = {},
): HTMLElementTagNameMap[K] => {
  const element = document.createElement(tag);
  element.textContent = text ?? '';
  Object.assign(element, attributes);
  return element;
};

const errorMessage = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);
const icons = (): void =>
  createIcons({
    icons: {
      CheckCheck,
      FolderMinus,
      FolderOpen,
      GitCompareArrows,
      Import,
      Package,
      Play,
      Plus,
      RefreshCw,
      RotateCw,
      Save,
      Settings2,
      Workflow,
      X,
    },
  });

let state: LauncherState = { sdk: { path: '', tools: {}, ready: false }, projects: [], jobs: [] };
let selected: string | undefined;
let currentFile: string | undefined;
let revision: string | undefined;
let original = '';
let tab = 'overview';
let polling: ReturnType<typeof setTimeout> | undefined;
let qualityProject: string | undefined;

async function api<T>(path: string, schema: ZodType<T>, data?: unknown): Promise<T> {
  const response = await fetch(
    `/api/${path}`,
    data === undefined
      ? {}
      : {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(data),
        },
  );
  const result: unknown = await response.json();
  if (!response.ok) {
    const message =
      typeof result === 'object' && result !== null && 'error' in result
        ? String(result.error)
        : `Request failed (${response.status})`;
    throw new Error(message);
  }
  const parsed = schema.safeParse(result);
  if (!parsed.success)
    throw new Error(`Invalid response from /api/${path}: ${parsed.error.message}`);
  return parsed.data;
}

const guard =
  <Args extends unknown[]>(callback: AsyncAction<Args>) =>
  async (...args: Args): Promise<void> => {
    try {
      await callback(...args);
    } catch (error) {
      $('status').textContent = errorMessage(error);
    }
  };

function showTab(name: string): void {
  tab = name;
  for (const button of document.querySelectorAll<HTMLElement>('[data-tab]')) {
    button.setAttribute('aria-pressed', String(button.dataset.tab === name));
  }
  for (const view of document.querySelectorAll<HTMLElement>('.view'))
    view.hidden = view.id !== name;
  if (name === 'quality') void guard(() => loadQuality())();
}

async function refresh(): Promise<void> {
  state = await api('state', launcherStateSchema);
  $('job-count').textContent = String(state.jobs.length);
  const query = $('search').value.toLowerCase();
  $('projects').replaceChildren();
  for (const item of state.projects.filter((item) => item.name.toLowerCase().includes(query))) {
    const button = node('button', item.name);
    button.setAttribute('aria-current', String(item.id === selected));
    button.onclick = guard(() => selectProject(item.id));
    $('projects').append(button);
  }
  renderJobs();
  if (!selected && state.projects.length) await selectProject(state.projects.at(-1)?.id ?? '');
  const active = state.jobs.some((job) => job.code === null);
  for (const id of ['run', 'check', 'graph'])
    $(id).disabled = !selected || active || !state.sdk.ready;
  if (polling !== undefined) clearTimeout(polling);
  if (active) polling = setTimeout(guard(refresh), 700);
}

async function selectProject(id: string): Promise<void> {
  if (!id) return;
  if ($('source').value !== original && !confirm('放弃未保存的脚本修改？')) return;
  selected = id;
  qualityProject = undefined;
  const item = state.projects.find((project) => project.id === id);
  if (!item) throw new Error('项目已不在工作区中');
  $('project-name').textContent = item.name;
  $('project-path').textContent = item.path;
  const files = await api(`scripts?id=${encodeURIComponent(id)}`, scriptsSchema);
  $('scripts').replaceChildren(...files.map((file) => node('option', file, { value: file })));
  $('file-list').replaceChildren();
  for (const file of files) {
    const button = node('button', file);
    button.onclick = guard(async () => {
      showTab('script');
      $('scripts').value = file;
      await loadScript();
    });
    $('file-list').append(button);
  }
  $('preview').src = `/thumbnail?id=${encodeURIComponent(id)}`;
  $('preview').hidden = false;
  $('preview').onerror = () => {
    $('preview').hidden = true;
  };
  const output = document.querySelector<HTMLInputElement>('[name=output]');
  if (output) output.value = `${item.path}-build`;
  $('impact-baseline').replaceChildren(
    ...state.projects
      .filter((project) => project.id !== id)
      .map((project) => node('option', project.name, { value: project.id })),
  );
  $('run-impact').disabled = !$('impact-baseline').value;
  $('impact-status').textContent = '选择基线项目后比较。';
  $('impact-list').replaceChildren();
  await loadScript();
  await refresh();
}

async function loadScript(): Promise<void> {
  if (!selected) return;
  const file = $('scripts').value;
  if (!file) return;
  const data = await api(
    `script?id=${encodeURIComponent(selected)}&file=${encodeURIComponent(file)}`,
    scriptSchema,
  );
  currentFile = file;
  revision = data.revision;
  original = data.text;
  $('source').value = original;
  $('dirty').textContent = '';
}

function table(headers: string[], rows: string[][]): HTMLTableElement {
  const table = node('table');
  const head = node('thead');
  const headRow = node('tr');
  for (const header of headers) headRow.append(node('th', header));
  head.append(headRow);
  const body = node('tbody');
  for (const row of rows) {
    const line = node('tr');
    for (const value of row) line.append(node('td', value));
    body.append(line);
  }
  table.append(head, body);
  return table;
}

function clearQualityLists(): void {
  for (const id of [
    'diagnostic-list',
    'uncovered-list',
    'route-list',
    'variable-list',
    'locale-list',
  ]) {
    $(id).replaceChildren();
  }
}

function renderQuality(envelope: Inspection): void {
  const report = envelope.data;
  const diagnostics = envelope.diagnostics;
  clearQualityLists();
  if (!envelope.ok || !report) {
    $('quality-status').textContent = envelope.error?.message || '项目检查失败';
    for (const item of diagnostics) {
      const row = node('div', null, { className: `diagnostic ${item.severity}` });
      row.append(
        node('strong', item.severity === 'error' ? '错误' : '警告'),
        node('span', `${item.file}:${item.line}:${item.column}`),
        node('p', item.message),
      );
      $('diagnostic-list').append(row);
    }
    for (const id of ['coverage-value', 'route-value', 'unreachable-value', 'locale-value'])
      $(id).textContent = '—';
    progress('coverage-bar').value = 0;
    return;
  }
  const coverage = report.routes.coverage;
  const unreachable = report.labels.filter((label) => !label.statically_reachable);
  const missing = report.localization.reduce((total, locale) => total + locale.missing.length, 0);
  $('quality-status').textContent = `${report.title} · ${report.fingerprint.slice(0, 12)}`;
  $('coverage-value').textContent = `${coverage.percent.toFixed(1)}%`;
  $('route-value').textContent =
    `${report.routes.passed}/${report.routes.passed + report.routes.failed}`;
  $('unreachable-value').textContent = String(unreachable.length);
  $('locale-value').textContent = String(missing);
  progress('coverage-bar').value = coverage.percent;
  if (!diagnostics.length)
    $('diagnostic-list').append(node('p', '没有静态诊断。', { className: 'empty' }));
  for (const item of diagnostics) {
    const row = node('div', null, { className: `diagnostic ${item.severity}` });
    row.append(
      node('strong', item.severity === 'error' ? '错误' : '警告'),
      node('span', `${item.file}:${item.line}:${item.column}`),
      node('p', item.message),
    );
    if (item.hint) row.append(node('small', item.hint));
    $('diagnostic-list').append(row);
  }
  if (!coverage.uncovered_labels.length) {
    $('uncovered-list').append(
      node('p', report.routes.configured ? '所有标签均有路线覆盖。' : '尚未配置 routes.json。', {
        className: 'empty',
      }),
    );
  } else {
    const list = node('ul');
    for (const label of coverage.uncovered_labels) list.append(node('li', label));
    $('uncovered-list').append(list);
  }
  const routeRows = report.routes.results.map((route) => [
    route.name,
    route.passed ? '通过' : '失败',
    route.result?.label || route.expected_label || '—',
    route.error || `${route.result?.visited_instructions || 0} 条指令`,
  ]);
  $('route-list').append(
    routeRows.length
      ? table(['路线', '状态', '结局', '详情'], routeRows)
      : node('p', '尚未配置路线测试。', { className: 'empty' }),
  );
  const variableRows = report.variables.map((variable) => [
    variable.name,
    variable.value_type,
    JSON.stringify(variable.initial_value),
    variable.persistent ? '是' : '否',
    `${variable.file}:${variable.line}`,
  ]);
  $('variable-list').append(
    variableRows.length
      ? table(['名称', '类型', '初始值', '持久化', '位置'], variableRows)
      : node('p', '项目没有变量。', { className: 'empty' }),
  );
  const localeRows = report.localization.map((locale) => [
    locale.language,
    `${locale.translated_messages}/${locale.source_messages}`,
    String(locale.missing.length),
    String(locale.obsolete.length),
    locale.path,
  ]);
  $('locale-list').append(
    localeRows.length
      ? table(['语言', '已翻译', '缺失', '过时', '目录'], localeRows)
      : node('p', '尚未配置本地化目录。', { className: 'empty' }),
  );
}

function renderMigration(report: MigrationReport): void {
  $('migration-list').replaceChildren();
  if (!report) {
    $('migration-status').textContent = '当前项目没有迁移报告。';
    return;
  }
  $('migration-status').textContent =
    `${report.issues.length} 个问题 · ${report.summary.assumptions} 个假设 · ${report.summary.unsupported} 个不支持项`;
  const rows = Object.entries(report.summary.by_code).map(([code, count]) => [code, String(count)]);
  $('migration-list').append(
    rows.length
      ? table(['分类', '数量'], rows)
      : node('p', '迁移报告没有问题。', { className: 'empty' }),
  );
}

function renderImpact(envelope: Impact): void {
  $('impact-list').replaceChildren();
  if (!envelope.ok || !envelope.data) {
    $('impact-status').textContent = envelope.error?.message || '影响分析失败';
    return;
  }
  const report = envelope.data;
  const labels = report.labels;
  $('impact-status').textContent = report.changed ? '检测到项目影响' : '未检测到语义影响';
  const rows = [
    ['标签新增', String(labels.added.length)],
    ['标签删除', String(labels.removed.length)],
    ['受影响路线', String(report.routes.length)],
    ['受影响结局', String(report.endings.length)],
    ['新增不可达标签', String(report.new_unreachable_labels.length)],
    ['存档风险', report.save_compatibility.risk],
  ];
  $('impact-list').append(table(['项目', '结果'], rows));
}

async function loadQuality(force = false): Promise<void> {
  if (!selected) return;
  if (!force && qualityProject === selected) return;
  qualityProject = selected;
  $('quality-status').textContent = '正在检查项目…';
  try {
    const [inspection, migration] = await Promise.all([
      api(`inspection?id=${encodeURIComponent(selected)}`, inspectionSchema),
      api(`migration-report?id=${encodeURIComponent(selected)}`, migrationSchema),
    ]);
    renderQuality(inspection);
    renderMigration(migration);
  } catch (error) {
    qualityProject = undefined;
    throw error;
  }
}

function renderJobs(): void {
  $('jobs-list').replaceChildren();
  if (!state.jobs.length) {
    $('jobs-list').append(node('p', '暂无任务'));
    return;
  }
  for (const job of state.jobs) {
    const row = node('article', null, { className: 'job' });
    const header = node('div', null, { className: 'job-header' });
    const failed = job.code !== null && job.code !== 0;
    header.append(
      node('span', job.name, { className: 'job-name' }),
      node('span', job.code === null ? '进行中' : job.code === 0 ? '完成' : `失败 (${job.code})`, {
        className: `job-state ${failed ? 'failed' : ''}`,
      }),
    );
    if (job.code === null) {
      const stop = node('button', '取消');
      stop.onclick = guard(async () => {
        await api('cancel', emptySchema, { id: job.id });
        await refresh();
      });
      header.append(stop);
    }
    row.append(header, node('pre', job.log || '…'));
    $('jobs-list').append(row);
  }
}

async function task(action: string, output?: string): Promise<void> {
  if (!selected) throw new Error('请先选择项目');
  await api('task', jobSchema, { id: selected, action, output });
  showTab('jobs');
  await refresh();
}

function field(name: string, label: string, value = '', type = 'text'): HTMLLabelElement {
  const wrapper = node('label', label);
  const input = node('input', null, { name, value, type, required: true });
  wrapper.append(input);
  return wrapper;
}

function formValues(form: HTMLElement): FormValues {
  return Object.fromEntries(
    [...new FormData(form as HTMLFormElement)].map(([key, value]) => [key, String(value)]),
  );
}

function dialog(
  title: string,
  fields: HTMLElement[],
  submit: (data: FormValues) => unknown | Promise<unknown>,
): void {
  $('dialog-title').textContent = title;
  $('dialog-fields').replaceChildren(...fields);
  $('dialog-error').textContent = '';
  $('dialog-form').onsubmit = async (event) => {
    event.preventDefault();
    $('dialog-submit').disabled = true;
    try {
      await submit(formValues($('dialog-form')));
      $('dialog').close();
      await refresh();
    } catch (error) {
      $('dialog-error').textContent = errorMessage(error);
    } finally {
      $('dialog-submit').disabled = false;
    }
  };
  $('dialog').showModal();
  icons();
}

$('new').onclick = () => {
  const template = node('label', '模板');
  const select = node('select', null, { name: 'template' });
  select.append(
    node('option', '标准剧情', { value: 'story' }),
    node('option', '数据交互', { value: 'inventory' }),
  );
  template.append(select);
  dialog(
    '新建项目',
    [
      field('title', '项目名称'),
      field('projectId', '项目 ID', 'org.renrs.my-story'),
      field('path', '新项目目录'),
      template,
    ],
    async (data) => {
      await api('create', jobSchema, data);
      showTab('jobs');
    },
  );
};
$('migrate').onclick = () =>
  dialog(
    "迁移 Ren'Py 项目",
    [field('source', "Ren'Py game 目录或 .rpy 文件"), field('output', '新的 RenRS 项目目录')],
    async (data) => {
      await api('migrate', jobSchema, data);
      showTab('jobs');
    },
  );
$('register').onclick = () =>
  dialog('添加已有项目', [field('path', '项目目录')], async (data) => {
    const item = await api('register', projectSchema, data);
    selected = undefined;
    state = await api('state', launcherStateSchema);
    await selectProject(item.id);
  });
$('sdk-button').onclick = () => {
  const tools = node('div', null, { className: 'sdk-tools' });
  for (const [name, ready] of Object.entries(state.sdk.tools)) {
    tools.append(
      node('span', `${ready ? '✓' : '×'} ${name}`, { className: ready ? '' : 'missing' }),
    );
  }
  dialog('SDK 设置', [field('path', 'SDK 目录', state.sdk.path), tools], (data) =>
    api('sdk', sdkSchema, data),
  );
};
$('dismiss').onclick = () => $('dialog').close();
$('search').oninput = guard(refresh);
$('run').onclick = guard(() => task('run'));
$('check').onclick = guard(() => task('check'));
$('graph').onclick = guard(() => task('graph'));
$('remove').onclick = guard(async () => {
  if (!selected) return;
  await api('remove', emptySchema, { id: selected });
  selected = undefined;
  location.reload();
});
$('scripts').onchange = guard(async () => {
  if ($('source').value !== original && !confirm('放弃未保存的脚本修改？')) {
    $('scripts').value = currentFile ?? '';
    return;
  }
  await loadScript();
});
$('reload-script').onclick = guard(async () => {
  if ($('source').value === original || confirm('放弃未保存的脚本修改？')) await loadScript();
});
$('source').oninput = () => {
  $('dirty').textContent = $('source').value === original ? '' : '未保存';
};
$('refresh-quality').onclick = guard(() => loadQuality(true));
$('run-impact').onclick = guard(async () => {
  if (!selected || !$('impact-baseline').value) return;
  $('impact-status').textContent = '正在比较…';
  renderImpact(
    await api('impact', impactSchema, {
      baseline: $('impact-baseline').value,
      candidate: selected,
    }),
  );
});
$('save-script').onclick = guard(async () => {
  if (!selected || !currentFile || !revision) return;
  const data = await api('script', revisionSchema, {
    id: selected,
    file: currentFile,
    text: $('source').value,
    revision,
  });
  revision = data.revision;
  original = $('source').value;
  $('dirty').textContent = '已保存';
  await task('check');
});
$('build-form').onsubmit = guard(async (event: SubmitEvent) => {
  event.preventDefault();
  const data = formValues($('build-form'));
  await task(data.action, data.output);
});
for (const button of document.querySelectorAll<HTMLElement>('[data-tab]')) {
  button.onclick = () => showTab(button.dataset.tab ?? tab);
}
window.addEventListener('beforeunload', (event) => {
  if ($('source').value !== original) event.preventDefault();
});

await guard(refresh)();
icons();
