@echo off
title AI Code Review Platform - Install Dependencies

echo ========================================
echo   AI Code Review Platform - Install
echo ========================================
echo.

set "ROOT_DIR=%~dp0"

:: Check Node.js
where node >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Node.js not found
    echo         Please install from: https://nodejs.org/
    pause
    exit /b 1
)

:: Check Rust/Cargo
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust/Cargo not found
    echo         Please install from: https://rustup.rs/
    pause
    exit /b 1
)

echo ----------------------------------------
echo [1/2] Building Rust backend
echo ----------------------------------------
cd /d "%ROOT_DIR%backend-rs"

echo      Compiling Rust backend (release mode)...
cargo build --release

if %errorlevel% equ 0 (
    echo      [OK] Rust backend built successfully
) else (
    echo      [ERROR] Failed to build Rust backend
    pause
    exit /b 1
)

echo.
echo ----------------------------------------
echo [2/2] Installing frontend dependencies
echo ----------------------------------------
cd /d "%ROOT_DIR%frontend"

echo      Installing npm packages (this may take a few minutes)...
call npm install

if %errorlevel% equ 0 (
    echo      [OK] Frontend dependencies installed
) else (
    echo      [ERROR] Failed to install frontend dependencies
    pause
    exit /b 1
)

echo.
echo ========================================
echo   All dependencies installed!
echo ========================================
echo.
echo   Run start.bat to launch the app
echo.
pause
