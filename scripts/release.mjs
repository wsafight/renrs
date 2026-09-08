import { cp, mkdir, readFile, writeFile, stat } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
const [command, ...args] = process.argv.slice(2);
const required = (name) => {
  const value = process.env[name];
  if (!value) throw new Error(`Set ${name}`);
  return value;
};
function run(binary, arguments_, input) {
  const result = spawnSync(binary, arguments_, {
    stdio: input ? ['pipe', 'inherit', 'inherit'] : 'inherit',
    input,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${binary} exited ${result.status}`);
}
async function fresh(directory) {
  try {
    await stat(directory);
    throw new Error(`Destination exists: ${directory}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  await mkdir(directory, { recursive: true });
}
function vdf(value, indent = '') {
  return Object.entries(value)
    .map(
      ([key, value]) =>
        `${indent}${JSON.stringify(key)} ${typeof value === 'object' ? `\n${indent}{\n${vdf(value, indent + '  ')}${indent}}` : JSON.stringify(String(value))}\n`,
    )
    .join('');
}
try {
  if (command === 'mac-app') {
    const [distribution, destination] = args;
    if (!distribution || !destination?.endsWith('.app'))
      throw new Error('mac-app <distribution> <output.app>');
    const manifest = JSON.parse(
      await readFile(path.join(distribution, 'renrs-build.json'), 'utf8'),
    );
    await fresh(destination);
    const contents = path.join(destination, 'Contents');
    await mkdir(path.join(contents, 'MacOS'), { recursive: true });
    await cp(distribution, path.join(contents, 'MacOS'), { recursive: true });
    const info = {
      CFBundleIdentifier: manifest.project_id,
      CFBundleName: manifest.title,
      CFBundleDisplayName: manifest.title,
      CFBundleExecutable: manifest.executable,
      CFBundlePackageType: 'APPL',
      CFBundleShortVersionString: manifest.engine_version,
      CFBundleVersion: manifest.engine_version,
      NSHighResolutionCapable: true,
      LSMinimumSystemVersion: '11.0',
    };
    run(
      'plutil',
      ['-convert', 'xml1', '-o', path.join(contents, 'Info.plist'), '-'],
      JSON.stringify(info),
    );
    console.log(`Prepared ${destination}`);
  } else if (command === 'mac-sign') {
    const [app] = args;
    if (!app?.endsWith('.app')) throw new Error('mac-sign <app>');
    run('codesign', [
      '--force',
      '--options',
      'runtime',
      '--timestamp',
      '--sign',
      required('RENRS_MAC_IDENTITY'),
      app,
    ]);
    run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', app]);
  } else if (command === 'mac-notarize') {
    const [app, zip] = args;
    if (!app?.endsWith('.app') || !zip) throw new Error('mac-notarize <signed-app> <new-zip>');
    const profile = required('RENRS_NOTARY_PROFILE');
    try {
      await stat(zip);
      throw new Error('Notarization zip already exists');
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    run('ditto', ['-c', '-k', '--keepParent', app, zip]);
    run('xcrun', ['notarytool', 'submit', zip, '--keychain-profile', profile, '--wait']);
    run('xcrun', ['stapler', 'staple', app]);
    run('spctl', ['--assess', '--type', 'execute', '--verbose=2', app]);
  } else if (command === 'windows-sign') {
    const [executable] = args;
    if (!executable) throw new Error('windows-sign <exe>');
    run('signtool', [
      'sign',
      '/sha1',
      required('RENRS_WINDOWS_CERT_SHA1'),
      '/fd',
      'SHA256',
      '/tr',
      'http://timestamp.digicert.com',
      '/td',
      'SHA256',
      executable,
    ]);
    run('signtool', ['verify', '/pa', executable]);
  } else if (command === 'store-files') {
    const [distribution, output, appId, depotId] = args;
    if (!distribution || !output || !/^\d+$/.test(appId) || !/^\d+$/.test(depotId))
      throw new Error('store-files <distribution> <new-output> <steam-app-id> <depot-id>');
    const manifest = JSON.parse(
      await readFile(path.join(distribution, 'renrs-build.json'), 'utf8'),
    );
    await fresh(output);
    await writeFile(
      path.join(output, 'app_build.vdf'),
      vdf({
        appbuild: {
          appid: appId,
          desc: `${manifest.title} ${manifest.engine_version}`,
          buildoutput: path.resolve(output, 'logs'),
          contentroot: path.resolve(distribution),
          preview: '1',
          depots: {
            [depotId]: {
              FileMapping: { LocalPath: '*', DepotPath: '.', recursive: '1' },
              FileExclusion: '*.pdb',
            },
          },
        },
      }),
    );
    await writeFile(
      path.join(output, '.itch.toml'),
      `[[actions]]\nname = "play"\npath = ${JSON.stringify(manifest.executable)}\n`,
    );
    console.log(`Prepared Steam preview build and itch launcher at ${output}`);
  } else throw new Error('Commands: mac-app, mac-sign, mac-notarize, windows-sign, store-files');
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
