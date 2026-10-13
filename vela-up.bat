@echo off
cd /d "%~dp0"
powershell.exe -NoLogo -NoProfile -ExecutionPolicy RemoteSigned -File "%~dp0vela-start.ps1"
if errorlevel 1 pause
