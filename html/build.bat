@echo off
setlocal
pushd "%~dp0"
cargo build -p cvar-html --release --target wasm32-unknown-unknown
if errorlevel 1 goto :fail
copy /y "..\target\wasm32-unknown-unknown\release\cvar_html.wasm" "public\cvar.wasm" >nul
if errorlevel 1 goto :fail
echo Built public\cvar.wasm
popd
exit /b 0
:fail
popd
exit /b 1
