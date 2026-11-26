@echo off
rem Rebuilds the local Gen2 copy from this checkout. Close the existing Gen2 copy first.
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build-release.ps1" || exit /b 1
start "" "%~dp0dist\EliteNexus-Gen2\EliteNexus-Gen2.exe"
