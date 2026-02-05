@echo off
title AI Code Review Platform
chcp 65001 >nul

echo ========================================
echo   AI Code Review Platform - Starting
echo ========================================
echo.

set "ROOT_DIR=%~dp0"

:: Check if frontend dependencies are installed
if not exist "%ROOT_DIR%frontend\node_modules" (
    echo [INFO] Frontend dependencies not installed. Please run install.bat
    pause
    exit /b 1
)

:: Always rebuild Rust backend to ensure latest version
echo [INFO] Building Rust backend (ensuring latest version)...
cd /d "%ROOT_DIR%backend-rs"
cargo build --release
if errorlevel 1 (
    echo [ERROR] Failed to build Rust backend
    pause
    exit /b 1
)
echo [INFO] Rust backend built successfully.
echo.

:: Start frontend (Electron will automatically start the backend)
echo Starting Electron app...
echo (Backend will be started automatically by Electron)
echo.
cd /d "%ROOT_DIR%frontend"
call npm run dev

echo.
echo Application closed.
pause
