import { execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import { registerProjectView } from './project-view';

const execute = promisify(execFile);
let client: LanguageClient | undefined;

interface CommandResult {
  stdout: string;
  stderr: string;
}
interface CheckDiagnostic {
  file: string;
  line: number;
  column: number;
  message: string;
  hint?: string;
  severity?: string;
}
interface MigrationIssue {
  file: string;
  line: number;
  message: string;
  kind: string;
  code: string;
}
interface MigrationReport {
  issues?: MigrationIssue[];
  source_root?: string;
}

function configuration(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration('renrs');
}
function tool(name: string): string {
  const binary = name + (process.platform === 'win32' ? '.exe' : '');
  const directory = configuration().get<string>('toolsPath', '');
  if (directory) return path.join(directory, binary);
  const roots = (vscode.workspace.workspaceFolders || []).map((folder) => folder.uri.fsPath);
  for (const root of [
    path.join(__dirname, 'bin', `${process.platform}-${process.arch}`),
    ...roots.flatMap((root) => [
      path.join(root, 'target', 'release'),
      path.join(root, 'target', 'debug'),
    ]),
  ]) {
    const candidate = path.join(root, binary);
    if (fs.existsSync(candidate)) return candidate;
  }
  return binary;
}
function project(): string {
  const editor = vscode.window.activeTextEditor;
  const folder =
    (editor && vscode.workspace.getWorkspaceFolder(editor.document.uri)) ||
    vscode.workspace.workspaceFolders?.[0];
  if (!folder) throw new Error('Open a RenRS project folder first.');
  return path.resolve(folder.uri.fsPath, configuration().get<string>('projectPath', '.'));
}
function escapeHtml(value: string): string {
  const entities: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  };
  return value.replace(/[&<>"']/g, (character) => entities[character]);
}
function errorProperty(error: unknown, property: string): string {
  if (typeof error !== 'object' || error === null || !(property in error)) return '';
  const value = (error as Record<string, unknown>)[property];
  return typeof value === 'string' ? value : '';
}
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  registerProjectView(context);
  const output = vscode.window.createOutputChannel('RenRS');
  const diagnostics = vscode.languages.createDiagnosticCollection('renrs-project');
  const migrationDiagnostics = vscode.languages.createDiagnosticCollection('renrs-migration');
  context.subscriptions.push(output, diagnostics, migrationDiagnostics);
  async function command(name: string, args: string[], root: string): Promise<CommandResult> {
    output.show(true);
    try {
      const result = await execute(tool(name), args, { cwd: root, maxBuffer: 8 * 1024 * 1024 });
      output.append(result.stdout + result.stderr);
      return result;
    } catch (error) {
      output.append(errorProperty(error, 'stdout') + errorProperty(error, 'stderr'));
      throw error;
    }
  }
  let checkVersion = 0;
  async function check() {
    const root = project();
    const version = ++checkVersion;
    const result = await command('renrs-check', ['--json', root], root).catch((error: unknown) => {
      if (errorProperty(error, 'code') === 'ENOENT')
        throw new Error(
          'RenRS tools were not found. Set renrs.toolsPath or add the tools to PATH.',
        );
      return { stdout: errorProperty(error, 'stdout'), stderr: errorProperty(error, 'stderr') };
    });
    if (version !== checkVersion) return;
    diagnostics.clear();
    const grouped = new Map<string, [vscode.Uri, vscode.Diagnostic[]]>();
    const report = JSON.parse(result.stdout || '{}') as { diagnostics?: CheckDiagnostic[] };
    for (const item of report.diagnostics || []) {
      const uri = vscode.Uri.file(path.resolve(root, item.file));
      const line = Math.max(0, item.line - 1),
        column = Math.max(0, item.column - 1);
      const range = new vscode.Range(line, column, line, column + 1);
      const diagnostic = new vscode.Diagnostic(
        range,
        item.message + (item.hint ? `\n${item.hint}` : ''),
        item.severity === 'warning'
          ? vscode.DiagnosticSeverity.Warning
          : vscode.DiagnosticSeverity.Error,
      );
      diagnostic.source = 'renrs-project';
      const entry = grouped.get(uri.toString()) || [uri, []];
      entry[1].push(diagnostic);
      grouped.set(uri.toString(), entry);
    }
    diagnostics.set([...grouped.values()]);
  }
  function register(name: string, callback: () => unknown | Promise<unknown>): void {
    context.subscriptions.push(
      vscode.commands.registerCommand(name, async () => {
        try {
          await callback();
        } catch (error) {
          vscode.window.showErrorMessage(`RenRS: ${errorMessage(error)}`);
        }
      }),
    );
  }
  async function showJson(content: unknown): Promise<void> {
    const document = await vscode.workspace.openTextDocument({
      language: 'json',
      content: JSON.stringify(content, null, 2),
    });
    await vscode.window.showTextDocument(document, vscode.ViewColumn.Beside);
  }
  function publishMigrationDiagnostics(report: MigrationReport, source: string): void {
    migrationDiagnostics.clear();
    const grouped = new Map<string, [vscode.Uri, vscode.Diagnostic[]]>();
    for (const item of report.issues || []) {
      const uri = vscode.Uri.file(path.resolve(source, item.file));
      const line = Math.max(0, item.line - 1);
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(line, 0, line, 1),
        item.message,
        item.kind === 'unsupported'
          ? vscode.DiagnosticSeverity.Error
          : vscode.DiagnosticSeverity.Warning,
      );
      diagnostic.source = 'renrs-migration';
      diagnostic.code = item.code;
      const entry = grouped.get(uri.toString()) || [uri, []];
      entry[1].push(diagnostic);
      grouped.set(uri.toString(), entry);
    }
    migrationDiagnostics.set([...grouped.values()]);
  }
  register('renrs.check', check);
  register('renrs.setup', async () => {
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      canSelectMany: false,
      openLabel: 'Select RenRS tools',
    });
    if (selected)
      await configuration().update(
        'toolsPath',
        selected[0].fsPath,
        vscode.ConfigurationTarget.Global,
      );
  });
  register('renrs.migrate', async () => {
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: true,
      canSelectFolders: true,
      canSelectMany: false,
      filters: { "Ren'Py project": ['rpy'] },
      openLabel: "Select Ren'Py source",
    });
    if (!selected) return;
    const destination = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(`${selected[0].fsPath}-renrs`),
      saveLabel: 'Migrate',
    });
    if (!destination) return;
    await command(
      'renrs-migrate',
      [selected[0].fsPath, destination.fsPath],
      path.dirname(selected[0].fsPath),
    );
    const reportPath = path.join(destination.fsPath, 'migration-report.json');
    const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as MigrationReport;
    publishMigrationDiagnostics(report, report.source_root || path.dirname(selected[0].fsPath));
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(reportPath));
  });
  register('renrs.migrationReport', async () => {
    const reportPath = path.join(project(), 'migration-report.json');
    if (!fs.existsSync(reportPath)) throw new Error('This project has no migration-report.json.');
    const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as MigrationReport;
    publishMigrationDiagnostics(report, report.source_root || project());
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(reportPath));
  });
  register('renrs.inspect', async () => {
    await vscode.workspace.saveAll(false);
    const root = project();
    const result = await command('renrs-inspect', [root], root);
    await showJson(JSON.parse(result.stdout));
  });
  register('renrs.impact', async () => {
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      canSelectMany: false,
      openLabel: 'Select baseline project',
    });
    if (!selected) return;
    await vscode.workspace.saveAll(false);
    const candidate = project();
    const result = await command('renrs-impact', [selected[0].fsPath, candidate], candidate);
    await showJson(JSON.parse(result.stdout));
  });
  register('renrs.graph', async () => {
    await vscode.workspace.saveAll(false);
    await command('renrs-graph', [project()], project());
  });
  register('renrs.run', async () => {
    await vscode.workspace.saveAll(false);
    const root = project();
    await vscode.tasks.executeTask(
      new vscode.Task(
        { type: 'renrs' },
        vscode.TaskScope.Workspace,
        'Run',
        'RenRS',
        new vscode.ProcessExecution(tool('renrs'), [root], { cwd: root }),
      ),
    );
  });
  register('renrs.build', async () => {
    await vscode.workspace.saveAll(false);
    const root = project();
    const destination = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(path.join(root, 'dist', 'game')),
      saveLabel: 'Build',
    });
    if (destination) await command('renrs-build', [root, destination.fsPath], root);
  });
  register('renrs.webBuild', async () => {
    await vscode.workspace.saveAll(false);
    const root = project();
    const destination = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(path.join(root, 'dist', 'web')),
      saveLabel: 'Build Web',
    });
    if (!destination) return;
    const shell =
      configuration().get<string>('webShellPath', '') ||
      path.join(path.dirname(tool('renrs-web-build')), 'web-shell');
    await command('renrs-web-build', [root, destination.fsPath, '--shell', shell], root);
  });
  register('renrs.accept', async () => {
    await vscode.workspace.saveAll(false);
    await command('renrs-accept', [project()], project());
  });
  register('renrs.checkSaves', async () => {
    const selected = await vscode.window.showOpenDialog({
      canSelectFolders: true,
      canSelectFiles: false,
      openLabel: 'Saved games directory',
    });
    if (selected) {
      await vscode.workspace.saveAll(false);
      await command('renrs-accept', [project(), '--saves', selected[0].fsPath], project());
    }
  });
  for (const [name, file] of [
    ['openScreens', 'screens.json'],
    ['openTheme', 'theme.json'],
  ]) {
    register(`renrs.${name}`, async () => {
      const uri = vscode.Uri.file(path.join(project(), file));
      if (!fs.existsSync(uri.fsPath))
        await vscode.workspace.fs.writeFile(
          uri,
          Buffer.from(file === 'screens.json' ? '{\n  "version": 1\n}\n' : '{}\n'),
        );
      await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
    });
  }
  register('renrs.init', async () => {
    const destination = await vscode.window.showSaveDialog({ saveLabel: 'Create Project' });
    if (!destination) return;
    const title = await vscode.window.showInputBox({
      title: 'Story title',
      value: path.basename(destination.fsPath),
    });
    if (title === undefined) return;
    const id = await vscode.window.showInputBox({
      title: 'Permanent project ID',
      value: `org.story.${path
        .basename(destination.fsPath)
        .toLowerCase()
        .replace(/[^a-z0-9-]/g, '-')}`,
      validateInput: (value) =>
        /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(value)
          ? undefined
          : 'Use letters, numbers, dots, underscores or hyphens.',
    });
    if (id === undefined) return;
    await command(
      'renrs-init',
      [destination.fsPath, '--title', title, '--id', id],
      path.dirname(destination.fsPath),
    );
    await vscode.commands.executeCommand('vscode.openFolder', destination, true);
  });
  register('renrs.test', async () => {
    const root = project();
    await vscode.workspace.saveAll(false);
    await command('renrs-debug', ['test', root, path.join(root, 'routes.json')], root);
  });
  register('renrs.explore', async () => {
    const root = project();
    await vscode.workspace.saveAll(false);
    await command('renrs-debug', ['explore', root], root);
  });
  register('renrs.preview', async () => {
    const root = project();
    const files = await vscode.workspace.findFiles(
      new vscode.RelativePattern(root, '**/*.{png,wav,ogg}'),
      '**/.*',
      5000,
    );
    const selected = await vscode.window.showQuickPick(
      files.map((uri) => ({ label: path.relative(root, uri.fsPath), uri })),
    );
    if (!selected) return;
    const panel = vscode.window.createWebviewPanel(
      'renrsResource',
      selected.label,
      vscode.ViewColumn.Beside,
      { localResourceRoots: [vscode.Uri.file(root)] },
    );
    const url = escapeHtml(panel.webview.asWebviewUri(selected.uri).toString());
    const media =
      path.extname(selected.uri.fsPath).toLowerCase() === '.png'
        ? `<img src="${url}" alt="${escapeHtml(selected.label)}">`
        : `<audio controls src="${url}"></audio>`;
    panel.webview.html = `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${panel.webview.cspSource}; media-src ${panel.webview.cspSource}; style-src 'unsafe-inline';"><style>body{margin:24px;color:var(--vscode-foreground)}img{max-width:100%;max-height:85vh;object-fit:contain}audio{width:100%}</style></head><body>${media}</body></html>`;
  });
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (configuration().get('checkOnSave') && document.languageId === 'renrs') {
        check().catch((error) => output.appendLine(error.message));
      }
    }),
  );
  async function startClient(): Promise<void> {
    if (!vscode.workspace.workspaceFolders?.length) return;
    if (client) await client.stop();
    const root = project();
    client = new LanguageClient(
      'renrs',
      'RenRS',
      { command: tool('renrs-lsp'), transport: TransportKind.stdio, options: { cwd: root } },
      {
        documentSelector: [{ scheme: 'file', language: 'renrs' }],
        outputChannel: output,
        synchronize: {
          fileEvents: vscode.workspace.createFileSystemWatcher(
            new vscode.RelativePattern(root, '**/*.rns'),
          ),
        },
        workspaceFolder: { uri: vscode.Uri.file(root), name: path.basename(root), index: 0 },
      },
    );
    context.subscriptions.push(client);
    try {
      await client.start();
    } catch (error) {
      output.appendLine(errorMessage(error));
      const action = await vscode.window.showErrorMessage(
        'RenRS tools are unavailable.',
        'Locate Tools',
      );
      if (action) await vscode.commands.executeCommand('renrs.setup');
    }
  }
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration('renrs.toolsPath') ||
        event.affectsConfiguration('renrs.projectPath')
      )
        startClient().catch((error) => output.appendLine(errorMessage(error)));
    }),
  );
  await startClient();
}

export async function deactivate(): Promise<void> {
  if (client) await client.stop();
}
