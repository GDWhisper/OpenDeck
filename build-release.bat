@echo off
setlocal enabledelayedexpansion

REM Build OpenDeck release with MSI installer
REM Usage: build-release.bat [tauri-build-args...]
REM Examples:
REM   build-release.bat              (full build with bundling)
REM   build-release.bat --no-bundle  (portable exe only)

set "PATH=D:\nvm4w\nodejs\node_modules\deno;%PATH%"
set ALL_PROXY=http://127.0.0.1:7897

cargo tauri build %*
if %ERRORLEVEL% neq 0 (
    echo Build failed.
    exit /b %ERRORLEVEL%
)

REM Rename MSI to remove _en-US suffix
for %%f in (src-tauri\target\release\bundle\msi\*_en-US.msi) do (
    set "filename=%%~nf"
    set "newname=!filename:_en-US=!"
    ren "%%f" "!newname!.msi"
    echo Renamed: %%~nxf -^> !newname!.msi
)

echo Done.
