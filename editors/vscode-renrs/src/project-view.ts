import * as vscode from 'vscode';

type ProjectCommand = readonly [label: string, command: string, icon: string];

export function registerProjectView(context: vscode.ExtensionContext): void {
  const commands: ProjectCommand[] = [
    ['New Project', 'renrs.init', 'new-folder'],
    ["Migrate Ren'Py Project", 'renrs.migrate', 'import'],
    ['Run', 'renrs.run', 'play'],
    ['Check Project', 'renrs.check', 'check'],
    ['Inspect Story', 'renrs.inspect', 'search'],
    ['Analyze Impact', 'renrs.impact', 'git-compare'],
    ['Story Graph', 'renrs.graph', 'type-hierarchy-sub'],
    ['Test Routes', 'renrs.test', 'beaker'],
    ['Explore Branches', 'renrs.explore', 'git-merge'],
    ['Preview Resource', 'renrs.preview', 'file-media'],
    ['Build Desktop', 'renrs.build', 'package'],
    ['Build Web', 'renrs.webBuild', 'globe'],
    ['Release Acceptance', 'renrs.accept', 'verified'],
    ['Check Saved Games', 'renrs.checkSaves', 'save'],
    ['Open Screens', 'renrs.openScreens', 'layout'],
    ['Open Theme', 'renrs.openTheme', 'symbol-color'],
    ['Open Migration Report', 'renrs.migrationReport', 'issues'],
    ['Locate Tools', 'renrs.setup', 'tools'],
  ];
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider('renrs.project', {
      getChildren: () => commands,
      getTreeItem: ([label, command, icon]) => {
        const item = new vscode.TreeItem(label);
        item.iconPath = new vscode.ThemeIcon(icon);
        item.command = { command, title: label };
        return item;
      },
    }),
  );
}
