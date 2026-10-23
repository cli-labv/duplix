@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM Lanzador local de Duplix. Todo queda dentro de esta carpeta.
REM   start.bat          -> organiza .\input
REM   start.bat --menu   -> menu interactivo
set ROOT_DIR=%~dp0
set ROOT_DIR=%ROOT_DIR:~0,-1%
set BIN=%ROOT_DIR%\dist\duplix-windows-x86_64.exe

if not exist "%BIN%" (
    echo Compilando ^(solo la primera vez^)...
    pushd "%ROOT_DIR%"
    cargo build --release --target x86_64-pc-windows-msvc
    popd
    if not exist "%ROOT_DIR%\dist" mkdir "%ROOT_DIR%\dist"
    copy "%ROOT_DIR%\target\x86_64-pc-windows-msvc\release\duplix.exe" "%BIN%" >nul
)

REM Aisla los datos de la app en esta carpeta: nada fuera de local.
set DUPLIX_HOME=%ROOT_DIR%
cd /d "%ROOT_DIR%"
"%BIN%" %*
