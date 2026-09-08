# 0.1 release contract and checklist

`release.json` is the release contract consumed by tooling. Rust crates and the editor package
must use the same version. Starting with `0.1.0-rc.1`, the existing formats below are frozen.
Adding optional object fields is compatible; removing fields, changing their meaning, or changing
acceptance rules requires a new format version. Pre-RC scripts and saves are not guaranteed.

## Compatibility matrix

| Boundary | RC version | Reader policy | Breaking change |
| --- | ---: | --- | --- |
| `.rns` language | `0.1` | Parsed by the same `0.1.x` engine; unknown syntax gets positioned diagnostics | Update language docs and migrator, then enter the next minor |
| Machine JSON protocol | `1` | Check `protocol_version` first; new optional fields are allowed | Raise `protocol_version` and retain the old schema |
| `screens.json` | `1` | Reject unknown versions and fields | Raise file `version` |
| `.renrs` resource archive | `1` | Check magic, version, bounds, and SHA-256 | Raise archive version or provide an explicit repack tool |
| Runtime snapshot | `7` | Accept only the current version | Raise version; old development saves may be discarded |
| Desktop/Web save container | `2` | Accept only the current version and verify checksum | Raise version and update both parsers |
| extension/composition | `1` | Reject unknown versions | Raise the matching manifest version |
| Frame/stream video | `1` / `2` | Strictly validate manifest shape and version | Raise the matching video manifest version |

`tests/release_contract.rs` checks the release version, machine protocol, snapshot, save, and screen
formats against compiled code. A format change must update implementation, fixtures, this matrix,
and upgrade notes, not only `release.json`.

## Local RC gates

```sh
node scripts/verify-local.mjs
node scripts/verify-local.mjs --web
RENRS_FFMPEG=/path/to/full/ffmpeg node scripts/verify-local.mjs --media
node scripts/verify-local.mjs --editor
node scripts/verify-local.mjs --release
# Or run every profile.
RENRS_FFMPEG=/path/to/full/ffmpeg node scripts/verify-local.mjs --full
```

Core covers Rust format/Clippy/tests, Biome, strict TypeScript, protocol unit tests, demo, product
fixture, the 30-60 minute first-party reference fixture, and route acceptance. `--web` does not
need FFmpeg. `--media` generates and verifies parallel animation, frame video, and streaming video.
`--release` checks release binaries, native smoke, SDK, and Capacitor wrapper structure. Run the
release profile again on every target platform.

## External release gates

- [ ] An owner-supplied real mid-size work passes authoring, migration, performance, and shipping acceptance.
- [ ] Packaged players pass on the publisher's target desktop systems.
- [ ] Media, lifecycle, file import, and system share pass on physical Android and iOS devices.
- [ ] Publisher identity, signing, notarization, privacy manifests, store assets, and store review.

These require a work, devices, or credentials. Local fixtures, simulators, and unsigned builds do
not replace them.
