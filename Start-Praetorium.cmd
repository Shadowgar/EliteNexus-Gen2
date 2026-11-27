@echo off
rem Starts the local Praetorium build, building it from source if needed.
setlocal
set "DIR=%~dp0dist\Praetorium"
if not exist "%DIR%\Praetorium.exe" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build-release.ps1" || exit /b 1
)
start "" "%DIR%\Praetorium.exe"
