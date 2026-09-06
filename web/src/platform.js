import {Capacitor} from '@capacitor/core';
import {App} from '@capacitor/app';
import {Filesystem, Directory, Encoding} from '@capacitor/filesystem';
import {Share} from '@capacitor/share';

export async function nativeDownload(name, data) {
  if (!Capacitor.isNativePlatform()) return false;
  const path = `exports/${name.replace(/[^a-zA-Z0-9._-]/g, '_')}`;
  const {uri} = await Filesystem.writeFile({path, data: typeof data === 'string' ? data : JSON.stringify(data, null, 2),
    directory: Directory.Cache, encoding: Encoding.UTF8, recursive: true});
  await Share.share({title: name, files: [uri]});
  return true;
}

export async function setupPlatform(app) {
  if (!Capacitor.isNativePlatform()) return;
  document.body.classList.add('native');
  let backgrounded = false;
  await App.addListener('appStateChange', ({isActive}) => app.guard(async () => {
    if (!isActive) {
      backgrounded = true;
      if (!app.$('modal').open) await app.openPanel('settings');
      for (const audio of app.voices.values()) audio.pause();
      await app.save('resume');
    } else if (backgrounded) {
      backgrounded = false;
      for (const audio of app.voices.values()) if (!audio.ended) await audio.play().catch(() => {});
    }
  })());
  await App.addListener('backButton', () => app.guard(async () => {
    if (app.$('modal').open) app.closeModal();
    else await app.openPanel('settings');
  })());
}
