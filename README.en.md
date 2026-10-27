# KeyEcho

[中文](README.md) | [English](README.en.md)

A Windows tray-resident tool that binds global hotkeys to local audio files. Press a hotkey and the bound audio plays.

## Features
- Chord-style global hotkeys: any number of keys pressed simultaneously triggers (e.g. `CTRL+S+1`); holding keys does not repeat playback
- Stays in the system tray with a menu supporting "Restart" (reload config) and "Exit"
- Single instance: launching a second instance exits automatically
- Single audio playback: a new trigger interrupts the current playback
- Portable: copy the whole folder anywhere and it just works
- EXE and tray share an embedded icon; no external `logo.ico` is needed at runtime

## Requirements
- Windows 10/11. Windows only (macOS not implemented yet)

## Quick Start
1. Download `KeyEcho_<version>_win64.zip` from the Releases page and extract it
2. Put your audio files into the `asset/` directory
3. Edit `config.json` to set your hotkeys and audio paths
4. Run `KeyEcho.exe` and press the configured hotkeys to play

## config.json
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
- `id`: unique identifier; duplicates are skipped
- `hotkey`: keys joined by `+`, all keys must be pressed **simultaneously**. Supports modifiers like `CTRL` / `ALT` / `SHIFT` / `WIN`, plus letters, digits, `F1-F24`, arrow keys, `SPACE` / `ENTER`, etc.
- `audio_path`: an absolute path, or a relative path resolved against the directory containing `config.json`

After changing the config, click "Restart" in the tray menu to apply it.

## Build from Source
```powershell
cargo test                     # unit tests
cargo run                      # debug run
powershell -ExecutionPolicy Bypass -File build/windows/package.ps1   # one-click package
```
The packaged output goes to `TempFile/dist/windows/`.

## Release
Pushing a tag matching `v*` to GitHub automatically triggers [GitHub Actions](.github/workflows/release.yml) to build and create a Release:
```powershell
git tag v1.0.0
git push github master --tags
```
