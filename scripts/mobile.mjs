import {cp, mkdir, readFile, writeFile, lstat} from 'node:fs/promises';
import path from 'node:path';

const [source, destination, ...extra] = process.argv.slice(2);
if (!source || !destination || extra.length) throw new Error('Usage: node scripts/mobile.mjs <web-build> <new-mobile-directory>');
const input = path.resolve(source), output = path.resolve(destination);
if (output === input || output.startsWith(`${input}${path.sep}`)) throw new Error('Mobile output must be outside the Web build');
const data = JSON.parse(await readFile(path.join(input, 'project.json'), 'utf8'));
const id = data.program.project_id;
if (!/^[a-zA-Z][\w]*(\.[a-zA-Z][\w]*)+$/.test(id)) throw new Error('Mobile builds need a reverse-domain project id, such as org.studio.game');
await mkdir(output); // Refuse to replace an existing project or signing configuration.
await cp(input, path.join(output, 'www'), {recursive: true, filter: async file => {
  if ((await lstat(file)).isSymbolicLink()) throw new Error(`Symlink in Web build: ${file}`);
  return true;
}});
const json = value => `${JSON.stringify(value, null, 2)}\n`;
await writeFile(path.join(output, 'capacitor.config.json'), json({
  appId: id, appName: data.program.title, webDir: 'www',
  server: {androidScheme: 'https'},
  android: {allowMixedContent: false},
  ios: {contentInset: 'automatic'},
  plugins: {SplashScreen: {launchAutoHide: true}},
}));
await writeFile(path.join(output, 'package.json'), json({
  name: id.replaceAll('.', '-').toLowerCase(), version: '0.1.0', private: true,
  scripts: {'android:add': 'cap add android', 'ios:add': 'cap add ios', sync: 'cap sync',
    android: 'cap open android', ios: 'cap open ios'},
  dependencies: {'@capacitor/core': '8.5.1', '@capacitor/android': '8.5.1', '@capacitor/ios': '8.5.1',
    '@capacitor/app': '8.1.1', '@capacitor/filesystem': '8.1.3', '@capacitor/share': '8.0.1'},
  devDependencies: {'@capacitor/cli': '8.4.3'},
}));
await writeFile(path.join(output, 'README.md'), `# ${data.program.title}\n\nGenerated from a validated RenRS Web build. Rust/WASM runs inside Capacitor.\n\n\`npm install\`, then \`npm run android:add\` and/or \`npm run ios:add\`.\nRun \`npm run sync\` after replacing www with a fresh Web build.\n\nAndroid: JDK 21, Android SDK 36 and Gradle dependencies. Build with\n\`cd android && ./gradlew assembleDebug\`; use Android Studio for release signing.\n\niOS: Xcode on macOS. Open with \`npm run ios\`, select a team and target device.\nThe generated Xcode project uses Swift Package Manager. Add the Filesystem\nprivacy manifest declarations from docs/MOBILE.md before store submission.\n\nKeep appId unchanged across releases to retain local saves. Save export uses\nthe system share sheet; import uses the system file picker. Store signing,\nprivacy labels, accounts, cloud saves and store approval are publisher work.\n`);
console.log(`Mobile project: ${output}\nNext: npm install --prefix ${output}`);
