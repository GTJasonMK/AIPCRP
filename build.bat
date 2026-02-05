@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

echo ========================================
echo  AI Code Review Platform - Build Script
echo ========================================
echo.

:: Check for build type argument
set BUILD_TYPE=%1
if "%BUILD_TYPE%"=="" set BUILD_TYPE=win

:: Step 1: Build Rust backend
echo [1/3] Building Rust backend...
cd /d "%~dp0backend-rs"

:: Check Rust toolchain
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo Error: Rust/Cargo not found. Please install from https://rustup.rs/
    exit /b 1
)

echo Building Rust backend (release mode)...
cargo build --release

if %ERRORLEVEL% neq 0 (
    echo Error: Backend build failed
    exit /b 1
)

echo Backend build completed.
echo.

:: Step 2: Copy backend exe to frontend resources
echo [2/3] Preparing resources...
cd /d "%~dp0"

:: Create resources directory
if not exist "frontend\resources\backend" mkdir "frontend\resources\backend"

:: Copy Rust backend executable
copy /Y "backend-rs\target\release\backend-rs.exe" "frontend\resources\backend\backend-rs.exe"

echo Resources prepared.
echo.

:: Step 3: Build Electron app
echo [3/3] Building Electron app...
cd /d "%~dp0frontend"

call npm run build:%BUILD_TYPE%

if %ERRORLEVEL% neq 0 (
    echo Error: Electron build failed
    exit /b 1
)

echo.
echo ========================================
echo  Build completed successfully!
echo  Output: frontend\dist\
echo ========================================

cd /d "%~dp0"
