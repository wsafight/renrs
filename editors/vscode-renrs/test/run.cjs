const path = require('node:path');
const { runTests } = require('@vscode/test-electron');
runTests({ extensionDevelopmentPath: path.resolve(__dirname, '..'), extensionTestsPath: path.resolve(__dirname, 'suite.cjs'),
  launchArgs: [path.resolve(__dirname, 'fixture'), '--disable-workspace-trust', '--skip-welcome', '--skip-release-notes'],
  extensionTestsEnv: { RENRS_TEST_TOOLS: path.resolve(__dirname, '../../../target/debug') }
}).catch(error => { console.error(error); process.exitCode = 1; });
