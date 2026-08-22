@echo off
title DSpark
cd /d %~dp0

where dspark >nul 2>&1
if %ERRORLEVEL%==0 (
  dspark %*
  goto :eof
)

if exist "%~dp0target\release\dspark.exe" (
  "%~dp0target\release\dspark.exe" %*
  goto :eof
)

if exist "%~dp0target\debug\dspark.exe" (
  "%~dp0target\debug\dspark.exe" %*
  goto :eof
)

echo DSpark engine not found. Install with: cargo install --path .
echo (This does not install the TUI; that binary is dspark-cli.)
exit /b 1
