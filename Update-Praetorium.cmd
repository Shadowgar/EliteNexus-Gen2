@echo off
rem Rebuilds the local Praetorium copy from this checkout. Close the existing Praetorium copy first.
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build-release.ps1" || exit /b 1
start "" "%~dp0dist\Praetorium\Praetorium.exe"
