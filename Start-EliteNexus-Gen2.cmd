@echo off
rem Starts the local Gen2 build, building it from source if needed.
setlocal
set "DIR=%~dp0dist\EliteNexus-Gen2"
if not exist "%DIR%\EliteNexus-Gen2.exe" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build-release.ps1" || exit /b 1
)
start "" "%DIR%\EliteNexus-Gen2.exe"
