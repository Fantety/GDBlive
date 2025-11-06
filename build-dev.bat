@echo off
REM 开发构建脚本 - 编译 debug 版本并复制到 demo/addons/gdblive

echo 开始构建 GDBLive (Debug)...

REM 执行 cargo build (debug)
cargo build

if %ERRORLEVEL% EQU 0 (
    echo 构建成功！
    
    REM 创建目标目录
    if not exist "demo\addons\gdblive" (
        mkdir "demo\addons\gdblive"
        echo 创建目录: demo\addons\gdblive
    )
    
    REM 复制 Windows DLL
    if exist "target\debug\GDBLive.dll" (
        copy /Y "target\debug\GDBLive.dll" "demo\addons\gdblive\GDBLive.windows.x86_64.dll" >nul
        echo 已复制: GDBLive.windows.x86_64.dll (Debug)
    )
    
    REM 创建 .gdextension 文件（如果不存在）
    if not exist "demo\addons\gdblive\gdblive.gdextension" (
        (
            echo [configuration]
            echo entry_symbol = "gdext_rust_init"
            echo compatibility_minimum = 4.1
            echo reloadable = true
            echo.
            echo [libraries]
            echo windows.debug.x86_64 = "res://addons/gdblive/GDBLive.windows.x86_64.dll"
            echo windows.release.x86_64 = "res://addons/gdblive/GDBLive.windows.x86_64.dll"
        ) > "demo\addons\gdblive\gdblive.gdextension"
        echo 已创建: gdblive.gdextension
    )
    
    echo.
    echo 构建完成！文件已复制到 demo\addons\gdblive
) else (
    echo 构建失败！
    exit /b 1
)
