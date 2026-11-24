<#
  U-05 第 1 档收口取证脚本  |  必须以管理员身份运行（脚本会自动弹 UAC）

  用法（任选其一）：
    1) 右键本文件 -> 使用 PowerShell 运行
    2) 任意 PowerShell 里执行：
       powershell -ExecutionPolicy Bypass -File .\scripts\u05-evidence\Run-U05-AsAdmin.ps1
    3) 只想手动截图（不做自动点击）：加 -NoAuto -Mode emergency

  做了什么：
    - 自检管理员权限，不足则 -Verb RunAs 重新拉起自己
    - 依次以 emergency / screen 两个隐私态启动 exe（环境变量进程启动时读取，只认严格 "1"）
    - 强制窗口前台、等 WebView2 渲染就绪
    - 自动粘贴会议文本、点「生成卡片」、截屏；紧急遮蔽态再点一次「查看详情」验证不可点
    - 产物写到 .tmp\u05-EVIDENCE-<mode>[-before|-detail].png

  跑完把 .tmp\u05-EVIDENCE-*.png 发回即可，我来更新判定表并收口 U-05。
#>
param(
    [ValidateSet('all', 'emergency', 'screen')]
    [string]$Mode = 'all',
    [switch]$NoAuto,
    [switch]$NoSandbox,
    [string]$Exe = ''
)

$ErrorActionPreference = 'Continue'

# ---------- 1) 管理员权限自检 / 自动提权 ----------
$isAdmin = ([Security.Principal.WindowsPrincipal] `
    ([Security.Principal.WindowsIdentity]::GetCurrent())
).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host '[U05] 需要管理员权限，正在请求 UAC 提权，请在弹窗里点「是」。' -ForegroundColor Yellow
    $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $PSCommandPath), '-Mode', $Mode)
    if ($NoAuto)    { $a += '-NoAuto' }
    if ($NoSandbox) { $a += '-NoSandbox' }
    if ($Exe)       { $a += @('-Exe', ('"{0}"' -f $Exe)) }
    try {
        Start-Process -FilePath 'powershell.exe' -Verb RunAs -ArgumentList $a | Out-Null
    } catch {
        Write-Host "[U05] 提权失败：$($_.Exception.Message)" -ForegroundColor Red
        Write-Host '      请手动「以管理员身份运行 PowerShell」后再执行本脚本。'
        Read-Host '按回车退出'
    }
    return
}
Write-Host '[U05] 已取得管理员权限。' -ForegroundColor Green

# ---------- 2) 路径 ----------
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $Exe) { $Exe = Join-Path $root 'target\debug\meeting-copilot-desktop.exe' }
if (-not (Test-Path $Exe)) {
    Write-Host "[U05] 找不到 exe：$Exe" -ForegroundColor Red
    Write-Host '      请先 build，或用 -Exe 指定完整路径。'
    Read-Host '按回车退出'
    return
}
$outDir = Join-Path $root '.tmp'
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }

# 与 22:41 正常态截图完全一致的取证文本
$Text       = '我们确认 SLA 99.9%，灰度范围限定华东两站点，回滚预案下周三前给。'
$InputLabel = '会议文本（手动粘贴）'
$GenButton  = '生成卡片'
$DetailBtn  = '查看详情'

# ---------- 3) Win32 / UIA / 截图 helper ----------
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
if (-not ('U5' -as [type])) {
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class U5 {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  public static void ForceFront(IntPtr h) {
    uint fgPid; uint fgThread = GetWindowThreadProcessId(GetForegroundWindow(), out fgPid);
    uint me = GetCurrentThreadId();
    AttachThreadInput(me, fgThread, true);
    ShowWindow(h, 9); BringWindowToTop(h); SetForegroundWindow(h);
    AttachThreadInput(me, fgThread, false);
    SetWindowPos(h, (IntPtr)(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0040);
  }
}
'@
}
[U5]::SetProcessDPIAware() | Out-Null

function Save-Shot([IntPtr]$hwnd, [string]$path) {
    [U5]::ForceFront($hwnd) | Out-Null
    Start-Sleep -Milliseconds 700
    $r = New-Object 'U5+RECT'
    [U5]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { Write-Host "  [shot] bad rect ${w}x${h}"; return }
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Host "  [shot] $path  (${w}x${h})" -ForegroundColor Cyan
}

function Find-Uia($ae, [string]$name) {
    try {
        $all = $ae.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                           [System.Windows.Automation.Condition]::TrueCondition)
        foreach ($e in $all) {
            try { if ($e.Current.Name -eq $name) { return $e } } catch { }
        }
    } catch { }
    return $null
}

function Invoke-Uia($e) {
    try {
        $e.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern).DoDefaultAction()
        return 'legacy'
    } catch { }
    try {
        $e.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        return 'invoke'
    } catch { }
    return $null
}

# ---------- 4) 逐态取证 ----------
$modes = if ($Mode -eq 'all') { @('emergency', 'screen') } else { @($Mode) }
$results = @()

foreach ($m in $modes) {
    Write-Host ''
    Write-Host "=== MODE: $m ===" -ForegroundColor Magenta

    & taskkill /F /IM meeting-copilot-desktop.exe 2>&1 | Out-Null
    Start-Sleep -Seconds 2

    # 环境变量在进程启动时读取，只认严格 "1"
    $env:PRIVACY_EMERGENCY_MASKED = $null
    $env:PRIVACY_SCREEN_SHARE = $null
    if ($m -eq 'emergency') { $env:PRIVACY_EMERGENCY_MASKED = '1' }
    if ($m -eq 'screen')    { $env:PRIVACY_SCREEN_SHARE = '1' }
    if ($NoSandbox) { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--no-sandbox' }
    else            { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $null }

    $p = Start-Process -FilePath $Exe -WorkingDirectory $root -PassThru
    Write-Host "  launched pid=$($p.Id)"

    # 等窗口出现
    $deadline = (Get-Date).AddSeconds(70)
    $hwnd = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 700
        $p.Refresh()
        if ($p.HasExited) { break }
        if ($p.MainWindowHandle -ne 0) { $hwnd = $p.MainWindowHandle; break }
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        Write-Host '  NO_WINDOW（应用未创建窗口）' -ForegroundColor Red
        Write-Host '  若反复出现，加 -NoSandbox 再试一次。'
        $results += "FAILED $m : no window"
        continue
    }
    Write-Host "  window hwnd=$hwnd" -ForegroundColor Green

    [U5]::ForceFront($hwnd) | Out-Null
    Start-Sleep -Milliseconds 800

    # 等 WebView2 渲染（UIA 后代 > 30 视为就绪）
    $ae = $null
    for ($i = 0; $i -lt 25; $i++) {
        Start-Sleep -Seconds 1
        $p.Refresh()
        if ($p.HasExited) { Write-Host "  EXITED at t=${i}s"; break }
        try {
            $ae = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
            $n = $ae.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                             [System.Windows.Automation.Condition]::TrueCondition).Count
            if ($i % 3 -eq 0) { Write-Host "  t=${i}s uia_nodes=$n" }
            if ($n -gt 30) { break }
        } catch { }
    }
    if ($p.HasExited -or $null -eq $ae) {
        Write-Host '  应用提前退出 / 未拿到 UIA，跳过自动操作' -ForegroundColor Red
        $results += "FAILED $m : exited before render"
        continue
    }

    $out = Join-Path $outDir "u05-EVIDENCE-$m.png"
    Save-Shot $hwnd ($out.Replace('.png', '-before.png'))

    if ($NoAuto) {
        Write-Host "  -NoAuto：应用已保持在 $m 态，请手动：" -ForegroundColor Yellow
        Write-Host "    1) 在「$InputLabel」粘贴：$Text"
        Write-Host "    2) 点「$GenButton」"
        Write-Host "    3) 截图另存为 $out"
        Read-Host '  完成后按回车继续'
        $results += "MANUAL $m"
        continue
    }

    try {
        # 填文本：优先 ValuePattern，失败退回剪贴板 + Ctrl+V
        $edit = Find-Uia $ae $InputLabel
        $filled = $false
        if ($edit) {
            try {
                $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Text)
                $filled = $true
                Write-Host '  set text ok (ValuePattern)'
            } catch { }
        }
        if (-not $filled) {
            Set-Clipboard -Value $Text
            [U5]::ForceFront($hwnd) | Out-Null
            if ($edit) { try { $edit.SetFocus() } catch { } }
            [System.Windows.Forms.SendKeys]::SendWait('^v')
            Write-Host '  set text via clipboard + Ctrl+V'
        }
        Start-Sleep -Milliseconds 600

        $btn = Find-Uia $ae $GenButton
        if ($btn) {
            $how = Invoke-Uia $btn
            Write-Host "  clicked $GenButton ($how)"
        } else {
            Write-Host '  WARN: 未找到生成卡片按钮，请手动点' -ForegroundColor Yellow
        }
        Start-Sleep -Seconds 2
        Save-Shot $hwnd $out

        # 紧急遮蔽态：再点一次「查看详情」，验证按钮不可点 / 无详情面板
        $d = Find-Uia $ae $DetailBtn
        if ($d) {
            try {
                $how2 = Invoke-Uia $d
                Start-Sleep -Seconds 1
                Save-Shot $hwnd ($out.Replace('.png', '-detail.png'))
                Write-Host "  detail click ($how2) captured"
            } catch { Write-Host '  detail click skipped' }
        }
        $results += "OK $m -> $out"
    } catch {
        Write-Host "  AUTO_FAILED：$($_.Exception.Message)" -ForegroundColor Red
        Write-Host '  窗口保持打开，请手动粘贴/出卡/截图后把图发回。'
        $results += "PARTIAL $m : auto failed, window kept"
        Read-Host '  按回车继续'
    }

    try { if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } } catch { }
    Start-Sleep -Seconds 2
}

Write-Host ''
Write-Host '========== 完成 ==========' -ForegroundColor Green
$results | ForEach-Object { Write-Host "  $_" }
Write-Host ''
Write-Host '请把 .tmp\u05-EVIDENCE-*.png 发回给我，我来更新 U-05 判定表并收口。' -ForegroundColor Cyan
Write-Host '期望现象：' -ForegroundColor Cyan
Write-Host '  emergency：卡片只显示「有 N 条」角标、正文隐藏、按钮「不可点击」，点详情无面板。'
Write-Host '  screen   ：0 张新卡，出现「这条内容未通过卡片生成条件」提示。'
Read-Host '按回车关闭本窗口'
