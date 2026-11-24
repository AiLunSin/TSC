param(
    [string]$Case = "",
    [switch]$NoRun,
    [switch]$VerboseBuild
)

$vs = "C:\Program Files\Microsoft Visual Studio\18\Community"
$module = Join-Path $vs "Common7\Tools\Microsoft.VisualStudio.DevShell.dll"

# Enter-VsDevShell 把环境变量装进大小写不敏感的字典再逐个 Add。终端里同时存在
# HTTP_PROXY 与 http_proxy（或 HTTPS_PROXY/https_proxy）时会抛
# 「已添加项。字典中的关键字」并导致整个 MSVC 环境加载失败（INCLUDE 为空，
# 随后 libsqlite3-sys 静默 exit 2）。先去掉小写重复项，保留大写值；
# 删完立刻用 .NET API 恢复大写，避免误删后出网代理丢失。
foreach ($lower in @('http_proxy', 'https_proxy', 'all_proxy', 'no_proxy')) {
    $upper = $lower.ToUpperInvariant()
    $value = [Environment]::GetEnvironmentVariable($upper)
    if ($value) {
        Remove-Item "Env:$lower" -ErrorAction SilentlyContinue
        [Environment]::SetEnvironmentVariable($upper, $value)
    }
}

if (-not (Test-Path $module)) {
    Write-Error "找不到 Visual Studio DevShell 模块：$module"
    exit 3
}
try {
    Import-Module $module -ErrorAction Stop
    Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation `
        -DevCmdArguments "-arch=x64 -host_arch=x64" -ErrorAction Stop | Out-Host
} catch {
    Write-Error "加载 VS 开发环境失败：$($_.Exception.Message)"
    exit 3
}

$rustBin = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$env:PATH = "$rustBin;$env:PATH"
# 与 run-u07-43-samples.sh 保持一致：关闭 dev debuginfo 可显著降低 cl.exe 峰值内存，
# 本机空闲内存偏紧时 sqlite3.c 单文件编译会因 OOM 静默失败（cl 返回 2 且不打印错误）。
$env:CARGO_PROFILE_DEV_DEBUG = "0"
$cargo = Join-Path $rustBin "cargo.exe"
if (-not (Test-Path $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}

$includeEntries = (($env:INCLUDE -split ';') | Where-Object { $_ }).Count
Write-Output "cargo: $cargo"
Write-Output "INCLUDE entries: $includeEntries"
if ($includeEntries -lt 1) {
    Write-Error "MSVC 环境未加载（INCLUDE 为空），拒绝编译"
    exit 3
}

$arguments = @("test", "-p", "meeting-copilot-infrastructure", "--features", "llm-https",
               "--test", "search_live")
if ($VerboseBuild) {
    $arguments += "-v"
}
if ($NoRun) {
    $arguments += "--no-run"
}
if ($Case) {
    $arguments += $Case
    if (-not $NoRun) {
        $arguments += @("--", "--ignored", "--test-threads=1", "--nocapture")
    }
}

& $cargo @arguments
exit $LASTEXITCODE