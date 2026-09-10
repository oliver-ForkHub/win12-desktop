@echo off
setlocal enabledelayedexpansion
REM bump-version.bat
REM
REM 自动修改发版前需要更新的版本字段。
REM
REM 用法:
REM   scripts\bump-version.bat 0.2.9                :: 使用今天日期
REM   scripts\bump-version.bat 0.2.9 2026-09-11
REM
REM 会修改:
REM   package.json
REM   tauri\src-tauri\Cargo.toml
REM   tauri\src-tauri\Cargo.lock
REM   tauri\src-tauri\tauri.conf.json
REM   snap\snapcraft.yaml
REM   io.github.win12_online.win12_desktop.metainfo.xml （只新增 release 条目和日期）

set "ROOT_DIR=%~dp0.."

if "%~1"=="" (
    echo 用法: %~nx0 新版本号 [发布日期]
    echo 示例: %~nx0 0.2.9
    echo       %~nx0 0.2.9 2026-09-11
    exit /b 1
)

set "NEW_VERSION=%~1"
set "RELEASE_DATE=%~2"
if "%RELEASE_DATE%"=="" for /f "delims=" %%d in ('powershell -NoProfile -Command "Get-Date -Format yyyy-MM-dd"') do set "RELEASE_DATE=%%d"

REM 简单校验版本号格式 主.次.修
set "VER_OK=0"
echo %NEW_VERSION%| findstr /R "^[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$" >nul && set "VER_OK=1"
if "%VER_OK%"=="0" (
    echo 错误: 版本号应为 主版本.次版本.修订版本 格式，例如 0.2.9，实际为: %NEW_VERSION%
    exit /b 1
)

REM 校验日期格式 YYYY-MM-DD
set "DATE_OK=0"
echo %RELEASE_DATE%| findstr /R "^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$" >nul && set "DATE_OK=1"
if "%DATE_OK%"=="0" (
    echo 错误: 发布日期应为 YYYY-MM-DD 格式，例如 2026-09-11，实际为: %RELEASE_DATE%
    exit /b 1
)

where node >nul 2>nul
if errorlevel 1 (
    echo 错误: 需要 node 来执行 scripts\bump-version.mjs
    exit /b 1
)

node "%ROOT_DIR%\scripts\bump-version.mjs" "%NEW_VERSION%" "%RELEASE_DATE%"
exit /b %errorlevel%
