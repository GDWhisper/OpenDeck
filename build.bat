@echo off
set PATH=D:\nvm4w\nodejs\node_modules\deno;%PATH%
set ALL_PROXY=http://127.0.0.1:7897
cd /d G:\Codes\opendeck\OpenDeck
echo [BUILD] Starting cargo tauri build --no-bundle ...
cargo tauri build --no-bundle
echo [BUILD] Exit code: %ERRORLEVEL%
