# Building on Windows

This repository can be built on native Windows with the MSVC Rust toolchain. The steps below are the reference setup for building both the Rust engine and the Tauri Studio application without WSL.

## Supported setup

- Windows 10 or Windows 11, x64
- Rust stable with the `x86_64-pc-windows-msvc` target
- Visual Studio 2022 Build Tools
- Windows 11 SDK `10.0.22621.0` or newer
- LLVM for `libclang.dll`
- Node.js 22.12+ with `npm.cmd`, which vite 7 and vitest 5 require; vitest 5 also leaves out 23 and 25

## Required Windows components

Install Visual Studio 2022 Build Tools with these components:

- `Microsoft.VisualStudio.Workload.VCTools`
- `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`
- `Microsoft.VisualStudio.Component.Windows11SDK.22621`
- `Microsoft.VisualStudio.Component.VC.CoreBuildTools`

Install LLVM so that `libclang.dll` is available, for example under `C:\Program Files\LLVM\bin\libclang.dll`.

## Why the environment matters

The Rust build depends on two pieces of native Windows tooling:

- MSVC and the Windows SDK, which provide `cl.exe`, `link.exe`, and system libraries such as `kernel32.lib`
- LLVM, which provides `libclang.dll` for crates that use `bindgen`

If those tools are installed but not active in the current shell, Cargo may fail with errors such as:

- `link.exe not found`
- `kernel32.lib` not found
- `Unable to find libclang`

## Build the Rust engine

Open `cmd.exe` or PowerShell and run the build from a Visual Studio developer environment.

```cmd
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
%USERPROFILE%\.cargo\bin\cargo.exe build --release --bin open-ontologies
```

Expected output:

- `target\release\open-ontologies.exe`

## Build the Studio frontend

Use the real Node.js installation, not the Windows app execution alias. On some systems `node.exe` may resolve to a packaged app path under `WindowsApps`, while `npm` is missing from `PATH`. If that happens, prepend `C:\Program Files\nodejs` to `PATH` before running `npm.cmd`.

```cmd
set "PATH=C:\Program Files\nodejs;%PATH%"
cd studio
npm.cmd install
npm.cmd run build
```

The build step also runs `npm run prepare-engine`, which copies:

- `target\release\open-ontologies.exe`

to:

- `studio\src-tauri\binaries\open-ontologies-x86_64-pc-windows-msvc.exe`

## Build the full Tauri desktop app

From the same shell, with MSVC and LLVM already configured:

```cmd
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\nodejs;%USERPROFILE%\.cargo\bin;%PATH%"
cd studio
npm.cmd run tauri build
```

Expected outputs:

- `studio\src-tauri\target\release\open-ontologies-studio.exe`
- `studio\src-tauri\target\release\bundle\msi\open-ontologies-studio_0.1.0_x64_en-US.msi`
- `studio\src-tauri\target\release\bundle\nsis\open-ontologies-studio_0.1.0_x64-setup.exe`

## Runtime notes

- `serve-unix` is intentionally unavailable on Windows. Use `serve` or `serve-http` instead.
- `~` path expansion uses `%USERPROFILE%` on Windows.
- The Studio backend no longer depends on `sh`, `lsof`, or Homebrew-specific paths when running on Windows.

## Troubleshooting

### `link.exe` or `kernel32.lib` not found

The Visual Studio Build Tools installation is incomplete, or `vcvars64.bat` was not called in the current shell.

### `Unable to find libclang`

LLVM is not installed, or `LIBCLANG_PATH` does not point to the directory containing `libclang.dll`.

### `npm` is not recognized

Install Node.js from the standard Windows installer and ensure `C:\Program Files\nodejs` is on `PATH`. If a `WindowsApps` alias shadows Node, explicitly prepend the Node.js installation directory before running `npm.cmd`.

### `prepare-engine` says the engine binary does not exist

Build the engine first with:

```cmd
%USERPROFILE%\.cargo\bin\cargo.exe build --release --bin open-ontologies
```
