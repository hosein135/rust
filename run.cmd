@echo off
setlocal
cd /d "%~dp0"

REM Elevate if not already running as Administrator (required by vfox-run.ps1)
net session >nul 2>&1
if errorlevel 1 (
  echo Requesting Administrator privileges...
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0vfox-run.ps1" %*
set "EXITCODE=%ERRORLEVEL%"
if not "%EXITCODE%"=="0" (
  echo.
  echo vfox-run.ps1 exited with code %EXITCODE%.
  pause
)
exit /b %EXITCODE%
