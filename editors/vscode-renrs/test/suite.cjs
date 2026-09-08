const vscode = require('vscode');
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs/promises');

async function until(callback) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const value = await callback();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error('Timed out waiting for language server');
}

exports.run = async function () {
  await vscode.workspace.getConfiguration('renrs').update('toolsPath', process.env.RENRS_TEST_TOOLS, vscode.ConfigurationTarget.Global);
  const extension = vscode.extensions.getExtension('renrs.renrs');
  assert(extension);
  await extension.activate();
  const root = vscode.workspace.workspaceFolders[0].uri.fsPath;
  const main = await vscode.workspace.openTextDocument(path.join(root, 'main.rns'));
  await vscode.window.showTextDocument(main);
  const query = () => vscode.commands.executeCommand('vscode.executeDefinitionProvider', main.uri, new vscode.Position(1, 10));
  assert.equal((await until(async () => { const result = await query(); return result?.length ? result : null; })).length, 1);
  const ending = await vscode.workspace.openTextDocument(path.join(root, 'ending.rns'));
  await vscode.window.showTextDocument(ending);
  await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
  assert.equal((await query()).length, 1, 'closing the defining document must preserve navigation');
  await vscode.commands.executeCommand('renrs.check');
  assert.equal(vscode.languages.getDiagnostics(main.uri).length, 0);
  const fixturePath = path.join(root, 'missing.rns');
  const fixtureUri = vscode.Uri.file(fixturePath);
  try {
    await fs.writeFile(fixturePath, 'label missing_asset:\n    scene "missing-background.png"\n    return\n');
    await vscode.commands.executeCommand('renrs.check');
    assert(vscode.languages.getDiagnostics(fixtureUri).some(item => item.source === 'renrs-project'), 'JSON project checker must report missing resources');
  } finally {
    await fs.rm(fixturePath, {force: true});
  }
  const commands = await vscode.commands.getCommands();
  for (const command of ['renrs.init', 'renrs.run', 'renrs.build', 'renrs.setup', 'renrs.webBuild', 'renrs.accept', 'renrs.checkSaves', 'renrs.openScreens', 'renrs.openTheme', 'renrs.migrate', 'renrs.migrationReport', 'renrs.inspect', 'renrs.impact', 'renrs.graph']) assert(commands.includes(command));
  console.log('RenRS extension host checks passed');
};
