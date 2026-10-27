# KeyEcho

[中文](README.md) | [English](README.en.md)

A Windows tray utility that binds global key combinations to local audio files.

## Features

- Trigger audio by pressing multiple keys together, such as `CTRL+S+1`.
- A new playback interrupts the current audio.
- Tray menu with “重启” (reload configuration) and “退出” (exit).
- Single instance: additional launches exit automatically.
- Portable configuration and audio files.
- The EXE and tray share an embedded icon; no external ICO is required at runtime.

Currently supports Windows 10/11 only. There is no configuration editor UI.

## Quick Start

1. Download `KeyEcho_<version>_win64.zip` from [Releases](https://github.com/NGTsjc/KeyEcho/releases) and extract it.
2. Add your audio files to `asset/`. Create the directory if it is missing.
3. Edit `config.json` with your hotkeys and actual audio paths.
4. Run `KeyEcho.exe`, find its tray icon, and press a configured hotkey.

Example runtime directory:

```text
KeyEcho/
├── KeyEcho.exe
├── config.json
├── asset/
│   └── hello.wav       # User-provided; not included in the package
└── keyecho.log         # Created at runtime
```

After editing the configuration, right-click the tray icon and select “重启” to reload it. This only reloads configuration. After updating the EXE, select “退出” to stop the old process, then launch the new binary.

## Configuration

Example `config.json`:

```json
{
  "bindings": [
    {
      "id": "hello",
      "hotkey": "CTRL+S+1",
      "audio_path": "asset/hello.wav"
    },
    {
      "id": "warning",
      "hotkey": "CTRL+SHIFT+W",
      "audio_path": "D:/Audio/warning.mp3"
    }
  ]
}
```

| Field | Description |
| --- | --- |
| `id` | A nonempty, unique binding identifier; duplicates are skipped |
| `hotkey` | Case-insensitive key names joined by `+`; all keys in the combination must be held together |
| `audio_path` | An absolute path, or a path relative to the directory containing `config.json` |

Use `/` in Windows paths inside JSON, or escape each backslash as `\\`.

Common key names:

| Type | Names |
| --- | --- |
| Modifiers | `CTRL`, `ALT`, `SHIFT`, `WIN` |
| Letters, digits, function keys | `A`–`Z`, `0`–`9`, `F1`–`F24` |
| Control keys | `SPACE`, `ENTER`, `TAB`, `ESC`, `BACKSPACE`, `DELETE`, `INSERT` |
| Navigation | `UP`, `DOWN`, `LEFT`, `RIGHT`, `HOME`, `END`, `PAGEUP`, `PAGEDOWN` |
| Punctuation | `MINUS`, `EQUAL`, `COMMA`, `PERIOD`, `SLASH`, `BACKSLASH`, `SEMICOLON`, `QUOTE`, `BACKQUOTE` |

Keystrokes still reach other applications. Choose combinations that avoid conflicts with your usual shortcuts.

KeyEcho first looks for `config.json` next to the EXE, then in the current working directory. If loading fails, it stays in the tray but clears the previous bindings. Entries with all required fields but empty values, duplicate IDs, or unsupported keys are skipped. Invalid JSON or entries missing required fields cause the entire configuration to fail. Audio availability and decoding are checked when playback is triggered.

## Run and Package from Source

On Windows, install a Rust toolchain supporting edition 2024, MSVC build tools, and the Windows SDK. Run these commands from the project root:

```powershell
cargo test

# Prepare configuration before the first development run, and add audio to asset/
Copy-Item .\build\windows\config.template.json .\config.json
cargo run
```

If `config.json` already exists, edit it directly to avoid overwriting your configuration with the template. The template references `asset/sample.wav`; provide that file or change the path.

Build and package:

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File .\build\windows\package.ps1
```

| Output | Path |
| --- | --- |
| Debug binary | `TempFile/target/debug/KeyEcho.exe` |
| Release binary | `TempFile/target/release/KeyEcho.exe` |
| Portable package directory | `TempFile/dist/windows/` |

The packaging script builds Release, copies the EXE and configuration template, creates the audio directory, and removes any legacy `asset/logo.ico`. It overwrites `config.json` in the package directory and preserves audio files. Save custom configuration elsewhere before packaging. The script creates a directory; the release workflow creates the ZIP.

Keep `asset/logo.ico` in the source tree: it is a build input. Rebuilding after changing it updates both the EXE and tray icons. The runtime directory needs no ICO, and legacy external ICO files are ignored.

## Releases

The [release workflow](.github/workflows/release.yml) runs when GitHub receives a tag matching `v*`. It builds **the commit referenced by that tag**. Pushing `master` does not trigger a release or move existing tags.

First update the version in `Cargo.toml`, then commit the changes. Run the following commands from the project root. `github` is this project's GitHub remote name; replace the example version with an unused version.

```powershell
# 1. Review and commit the release contents
git status
git add .
git commit -m "Release v1.0.2"

# 2. Test and push the code; continue only after tests pass
cargo test
git push github master

# 3. Create and verify the tag after committing
git tag v1.0.2
git log -1 --oneline
git show -s --oneline v1.0.2

# 4. Confirm the intended commit, then push only this tag
git push github refs/tags/v1.0.2
```

Check that the workflow succeeds in GitHub Actions, then download `KeyEcho_1.0.2_win64.zip` from Releases. CI synchronizes the Cargo version with the tag in its build workspace; it does not change repository commits.

If the artifact contains old code, check the tag with `git show -s --oneline <tag>`. `git add` stages files; only `git commit` creates a commit. If the old tag has already been published, prefer a new patch version tag. Rerunning the workflow for the old tag still builds the old commit.

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| No window appears | Expected behavior; look in the system tray or hidden icons area |
| Launching again does nothing | Only one instance is allowed; exit the existing instance first |
| No sound on hotkey | Check configuration, audio paths, output device, and errors in `keyecho.log` |
| Configuration changes have no effect | Select “重启” in the tray menu; check the configuration path and active binding count in the log |
| Old tray icon after updating | Exit the old process and launch the new EXE; check its path and the release tag's commit |

`keyecho.log` is written next to the selected configuration file and overwritten on normal startup. Save the previous log before restarting if needed for troubleshooting.

## Project Structure

```text
src/
├── main.rs             # Executable entry point
├── lib.rs              # Library entry point
├── app/                # Startup, logging, config reload, event handling
├── config/             # Validation, hotkey parsing, path handling
├── audio/              # Audio playback with rodio
└── platform/           # Windows keyboard hook, tray, single instance
asset/logo.ico          # Shared build resource for EXE and tray icons
build.rs                # Embeds Windows icon resources
build/windows/          # Configuration template and packaging script
.github/workflows/      # Tag-based release workflow
```
