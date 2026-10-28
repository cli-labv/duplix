@echo off
setlocal EnableExtensions EnableDelayedExpansion
set ROOT_DIR=%~dp0
set ROOT_DIR=%ROOT_DIR:~0,-1%
set DUPLIX_HOME=%ROOT_DIR%
cd /d "%ROOT_DIR%"
if exist "%ROOT_DIR%\duplix.exe" (
    "%ROOT_DIR%\duplix.exe" %*
) else (
    echo No se encontro duplix.exe en esta carpeta.
    echo Copia el binario Windows aqui o usa start.bat original.
)
