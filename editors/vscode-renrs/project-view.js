const vscode = require('vscode');

exports.registerProjectView = context => {
  const commands = [
    ['New Project', 'renrs.init', 'new-folder'],
    ['Run', 'renrs.run', 'play'],
    ['Check Project', 'renrs.check', 'check'],
    ['Test Routes', 'renrs.test', 'beaker'],
    ['Explore Branches', 'renrs.explore', 'git-merge'],
    ['Preview Resource', 'renrs.preview', 'file-media'],
    ['Build Desktop', 'renrs.build', 'package'],
    ['Build Web', 'renrs.webBuild', 'globe'],
    ['Release Acceptance', 'renrs.accept', 'verified'],
    ['Check Saved Games', 'renrs.checkSaves', 'save'],
    ['Open Screens', 'renrs.openScreens', 'layout'],
    ['Open Theme', 'renrs.openTheme', 'symbol-color'],
    ['Locate Tools', 'renrs.setup', 'tools'],
  ];
  context.subscriptions.push(vscode.window.registerTreeDataProvider('renrs.project', {
    getChildren: () => commands,
    getTreeItem: ([label, command, icon]) => {
      const item = new vscode.TreeItem(label); item.iconPath = new vscode.ThemeIcon(icon);
      item.command = {command, title: label}; return item;
    },
  }));
};
