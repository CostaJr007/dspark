@echo off
title DSpark CLI
cd /d %~dp0

where dspark-cli >nul 2>&1
if %ERRORLEVEL%==0 (
  dspark-cli %*
  goto :eof
)

if exist "%~dp0target\release\dspark-cli.exe" (
  "%~dp0target\release\dspark-cli.exe" %*
  goto :eof
)

if exist "%~dp0target\debug\dspark-cli.exe" (
  "%~dp0target\debug\dspark-cli.exe" %*
  goto :eof
)

echo DSpark CLI not found. Install with: cargo install --path .
exit /b 1
