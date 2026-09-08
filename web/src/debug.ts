import cytoscape from 'cytoscape';
import { type DebugInspection, parseDebugInspection } from './protocol';
import { download } from './storage';
import type { PlayerApp } from './types';

export function setupDebugger(app: PlayerApp): void {
  const program = app.data.program;
  const labels = Object.entries(program.labels).sort((a, b) => a[1] - b[1]);
  const owners: Array<string | undefined> = [];
  let labelIndex = 0;
  for (let index = 0; index < program.instructions.length; index++) {
    while (labelIndex + 1 < labels.length && labels[labelIndex + 1][1] <= index) labelIndex++;
    owners[index] = labels[labelIndex]?.[1] <= index ? labels[labelIndex][0] : undefined;
  }
  const owner = (index: number): string | undefined => owners[index];
  let inspection: DebugInspection | null = null;
  const edges = new Map<string, { data: { id: string; source: string; target: string } }>();
  for (const [index, instruction] of program.instructions.entries()) {
    const from = owner(index);
    const kind = instruction.kind;
    const targets = [
      kind.Jump?.target,
      kind.Call?.target,
      kind.JumpIfFalse?.target,
      ...(kind.Choice?.options || []).map((option) => option.target),
    ].filter((target): target is number => target != null);
    for (const target of targets) {
      const to = owner(target);
      if (from && to && from !== to)
        edges.set(`${from}:${to}`, {
          data: { id: `edge:${from}:${to}`, source: from, target: to },
        });
    }
  }
  const graph = cytoscape({
    container: app.$('graph'),
    elements: [...labels.map(([label]) => ({ data: { id: label, label } })), ...edges.values()],
    style: [
      {
        selector: 'node',
        style: {
          'background-color': '#176c58',
          label: 'data(label)',
          color: '#172721',
          'font-size': 12,
          'text-valign': 'bottom',
          'text-margin-y': 6,
        },
      },
      {
        selector: 'edge',
        style: {
          width: 2,
          'line-color': '#7e8b93',
          'target-arrow-color': '#7e8b93',
          'target-arrow-shape': 'triangle',
          'curve-style': 'bezier',
        },
      },
      {
        selector: '.current',
        style: { 'background-color': '#c34840', 'border-width': 3, 'border-color': '#ffb05d' },
      },
      { selector: '.visited', style: { 'border-color': '#38ae94', 'border-width': 3 } },
    ],
    layout: { name: 'breadthfirst', directed: true, padding: 30 },
    minZoom: 0.25,
    maxZoom: 3,
  });
  const breakpoints = new Set();
  let selectedLabel = labels[0]?.[0];
  function instructions() {
    const body = app.$('instructions');
    body.replaceChildren();
    for (const [index, instruction] of program.instructions.entries()) {
      if (owner(index) !== selectedLabel) continue;
      const row = app.element('label', null, { className: 'instruction' });
      row.dataset.index = String(index);
      const check = app.element('input', null, {
        type: 'checkbox',
        checked: breakpoints.has(instruction.id),
        ariaLabel: `Breakpoint ${instruction.span.source}:${instruction.span.line}`,
      });
      check.onchange = app.guard(() => {
        if (check.checked) breakpoints.add(instruction.id);
        else breakpoints.delete(instruction.id);
        app.engine.breakpoints(JSON.stringify([...breakpoints]));
      });
      row.append(
        check,
        app.element('span', `${instruction.span.line} ${Object.keys(instruction.kind)[0]}`),
      );
      body.append(row);
    }
  }
  graph.on('tap', 'node', (event) => {
    selectedLabel = event.target.id();
    instructions();
    app.updateDebug?.();
  });
  app.updateDebug = () => {
    if (!app.state || app.$('debug').hidden) return;
    inspection = parseDebugInspection(app.engine.inspect());
    const state = inspection.debug;
    const coverage = new Set(inspection.coverage);
    app.$('coverage').textContent = `${coverage.size} / ${program.instructions.length} visited`;
    app.$('variables').textContent = JSON.stringify(
      { variables: state.variables, call_stack: state.call_stack },
      null,
      2,
    );
    app.$('location').textContent =
      `${state.paused ? 'Paused' : 'Waiting'} ${state.location?.source || ''}:${state.location?.line || 0}`;
    graph.nodes().removeClass('current visited');
    graph
      .nodes()
      .filter((node) => node.id() === state.label)
      .addClass('current');
    const visited = new Set(inspection.coverage.map(owner));
    graph
      .nodes()
      .filter((node) => visited.has(node.id()))
      .addClass('visited');
    for (const row of app.$('instructions').children) {
      if (!(row instanceof HTMLElement)) continue;
      const index = Number(row.dataset.index);
      row.classList.toggle('current', index === state.next_instruction);
      row.classList.toggle('visited', coverage.has(index));
    }
  };
  app.$('debug-toggle').onclick = () => {
    app.$('debug').hidden = !app.$('debug').hidden;
    app.updateDebug?.();
    graph.resize();
    graph.layout({ name: 'breadthfirst', directed: true, padding: 30 }).run();
  };
  app.$('resume').onclick = app.guard(() => app.act('resume'));
  app.$('step').onclick = app.guard(() => app.act('step'));
  app.$('export-coverage').onclick = app.guard(async () => {
    app.updateDebug?.();
    const current = inspection;
    if (!current) throw new Error('Debug inspection unavailable');
    await download('route-coverage.json', {
      project_id: program.project_id,
      fingerprint: program.fingerprint,
      choices: app.route,
      instructions: current.coverage.map(
        (index) => program.instructions[index]?.id ?? `missing:${index}`,
      ),
      debug: current.debug,
    });
  });
  app.graph = graph;
  instructions();
}
