@echo off
setlocal EnableExtensions DisableDelayedExpansion

rem Build Fuji Recipe Manager as a Windows executable.
rem Run this file from File Explorer or from Command Prompt/PowerShell on Windows.

cd /d "%~dp0"

echo.
echo === Fuji Recipe Manager - Windows EXE build ===
echo.

where node >nul 2>&1 || goto :missing_node
where npm >nul 2>&1 || goto :missing_npm
where cargo >nul 2>&1 || goto :missing_rust

if not exist "node_modules\" (
  echo Installing JavaScript dependencies...
  call npm install
  if errorlevel 1 goto :failed
)

echo Checking TypeScript...
call npm run check
if errorlevel 1 goto :failed

echo Building Windows executable...
call npm run tauri -- build --bundles app
if errorlevel 1 goto :failed

set "APP_EXE=target\release\Fuji Recipe Manager.exe"
if not exist "%APP_EXE%" (
  echo.
  echo Build completed, but the expected executable was not found:
  echo   %APP_EXE%
  echo Check the Tauri output above for the actual artifact path.
  exit /b 1
)

echo.
echo Build completed successfully.
echo Executable:
echo   %CD%\%APP_EXE%
echo.
echo This local executable is not code-signed. Sign it before distributing it.
exit /b 0

:missing_node
echo Node.js was not found. Install the current Node.js LTS release, then run this file again.
exit /b 1

:missing_npm
echo npm was not found. Reinstall Node.js LTS, then run this file again.
exit /b 1

:missing_rust
echo Rust was not found. Install Rust with the MSVC toolchain, then run this file again.
echo Tauri also requires Microsoft C++ Build Tools and the WebView2 Runtime on Windows.
exit /b 1

:failed
echo.
echo Build failed. Review the error above; no release artifact was marked successful.
exit /b 1
