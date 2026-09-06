# Mobile Distribution

RenRS packages its Rust/WASM Web player inside Capacitor 8. This produces Android
and iOS projects; it does not port the Macroquad desktop renderer to mobile.
The output includes all story assets and works without a game server.

## Build

```sh
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- my-story target/web-story
node scripts/mobile.mjs target/web-story target/mobile-story
npm install --prefix target/mobile-story
npm run android:add --prefix target/mobile-story
npm run ios:add --prefix target/mobile-story
```

Node 22+, JDK 21 and Android SDK 36 are required for Android. Use Android Studio
or run `./gradlew assembleDebug` inside the generated `android` directory.
Set `ANDROID_HOME` to the installed SDK. Release signing belongs in the native
project; the generator refuses to overwrite an existing project.

iOS requires Xcode on macOS. Run `npm run ios --prefix target/mobile-story` to
open the project. Capacitor uses Swift Package Manager and needs network access
to resolve native dependencies. For an unsigned simulator build:

```sh
xcodebuild -quiet -project target/mobile-story/ios/App/App.xcodeproj \
  -scheme App -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath target/ios-build CODE_SIGNING_ALLOWED=NO build
```

For updates, replace only `www` with a fresh validated Web build, then run
`npm run sync` in the mobile project. Keep the same `config id` / Capacitor appId
and native signing identity. Never recreate the project over release settings.
The project is pre-release: retaining app storage does not guarantee that saves
from an earlier compiled story can be loaded. Start a new game after script changes.
Mobile identifiers must be valid reverse-domain identifiers; use letters,
numbers and underscores in segments, not hyphens.

## Native Behavior

- Android's back button closes an open panel or opens Settings, keeping progress.
- Backgrounding pauses presentation and saves a resume slot; foregrounding leaves
  the panel visible until the player resumes. An OS kill can still interrupt an
  in-flight write, so this complements periodic and manual saves.
- Export writes a checked desktop-compatible JSON container to the app cache and
  opens the system share sheet. Import uses the platform file picker.
- Player content observes safe-area insets. IndexedDB and localStorage remain
  isolated to the app; uninstalling can erase them. Export important saves.

The generator pins Capacitor core/platforms 8.5.1 and CLI 8.4.3. The latter avoids
the xcode/uuid advisory in CLI 8.5.1. Commit the generated package-lock.json and
use `npm ci` for repeatable builds.

## Publication

Before submitting, provide app icons/splash art, version/build numbers, a release
signing team/key, store metadata and appropriate privacy declarations. The current
Filesystem plugin uses file timestamps: add `NSPrivacyAccessedAPICategoryFileTimestamp`
with reason `C617.1` to `PrivacyInfo.xcprivacy` in the iOS app target as documented
by Capacitor Filesystem. Check the final archive's manifests and store requirements.
The generated native icons remain Capacitor defaults until the publisher replaces them.

Local verification built an Android debug APK and iOS simulator app, installed and
launched the iOS app, and inspected its rendered title screen. Desktop/mobile browser
tests cover the story and save workflows. Physical-device playback, native share/file
picker interaction, Android lifecycle events, release signing, store approval, Steam
services, cloud saves and purchases still require platform acceptance.
