# 构建脚本 - 编译并复制到 demo/addons/gdblive

Write-Host "开始构建 GDBLive..." -ForegroundColor Green

# 执行 cargo build
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "构建成功！" -ForegroundColor Green
    
    # 创建目标目录
    $targetDir = "demo\addons\gdblive"
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
        Write-Host "创建目录: $targetDir" -ForegroundColor Yellow
    }
    
    # 复制 Windows DLL
    $sourceDll = "target\release\GDBLive.dll"
    if (Test-Path $sourceDll) {
        Copy-Item $sourceDll -Destination "$targetDir\GDBLive.windows.x86_64.dll" -Force
        Write-Host "已复制: GDBLive.windows.x86_64.dll" -ForegroundColor Cyan
    }
    
    # 创建 .gdextension 文件（如果不存在）
    $gdextFile = "$targetDir\gdblive.gdextension"
    if (-not (Test-Path $gdextFile)) {
        $gdextContent = @"
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

[libraries]
windows.debug.x86_64 = "res://addons/gdblive/GDBLive.windows.x86_64.dll"
windows.release.x86_64 = "res://addons/gdblive/GDBLive.windows.x86_64.dll"
"@
        Set-Content -Path $gdextFile -Value $gdextContent -Encoding UTF8
        Write-Host "已创建: gdblive.gdextension" -ForegroundColor Cyan
    }
    
    Write-Host "`n构建完成！文件已复制到 $targetDir" -ForegroundColor Green
} else {
    Write-Host "构建失败！" -ForegroundColor Red
    exit 1
}
