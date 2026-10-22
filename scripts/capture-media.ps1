<#
.SYNOPSIS
  Captures the real LongPathFix window (light and dark) for the README and the website by driving the published exe with
  UI Automation against a throw-away demo fixture. Every image is the app's own pixels. Needs an unlocked desktop.
  Your real files are never touched; %APPDATA%\LongPathFix is restored.
.EXAMPLE
  dotnet publish -c Release -r win-x64 --self-contained -p:PublishSingleFile=true -o dist/stage
  pwsh -File scripts/capture-media.ps1
#>
param(
  [string]$Out = (Join-Path (Split-Path -Parent $PSScriptRoot) 'docs\media\app'),
  [int]$Width = 1280, [int]$Height = 800
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Win {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
public static class Mouse {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  public static void Wheel(int x, int y, int notches) { SetCursorPos(x, y); mouse_event(0x0800, 0, 0, unchecked((uint)(notches * 120)), UIntPtr.Zero); }
  public static void Click(int x, int y) { SetCursorPos(x, y); mouse_event(2, 0, 0, 0, UIntPtr.Zero); mouse_event(4, 0, 0, 0, UIntPtr.Zero); }
}
"@
[void][Win]::SetProcessDPIAware()
$AE = [System.Windows.Automation.AutomationElement]; $TS = [System.Windows.Automation.TreeScope]
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'dist\stage\LongPathFix.exe'
$fixture = Join-Path $PSScriptRoot 'demo-fixture.ps1'
$dataDir = Join-Path $env:APPDATA 'LongPathFix'
if (-not (Test-Path $exe)) { throw "Publish first: dotnet publish LongPathFix.csproj -c Release -r win-x64 --self-contained -p:PublishSingleFile=true -o dist/stage" }
New-Item -ItemType Directory -Force $Out | Out-Null
Get-ChildItem $Out -Filter *.png -ErrorAction SilentlyContinue | Remove-Item -Force   # every shot is regenerated

$dataExisted = Test-Path $dataDir
$before = if ($dataExisted) { Get-ChildItem $dataDir -Recurse -Force | ForEach-Object FullName } else { @() }
$checks = [Collections.Generic.List[string]]::new()
function Check([bool]$ok, [string]$what) { $checks.Add(("{0} {1}" -f $(if ($ok) { 'PASS' } else { 'FAIL' }), $what)); if (-not $ok) { throw "check failed: $what" } }

function Snap($proc, [string]$name, [switch]$Screen) {
  Start-Sleep -Milliseconds 700
  $h = $proc.MainWindowHandle
  $r = New-Object Win+RECT; [void][Win]::GetWindowRect($h, [ref]$r)
  $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  if ($Screen) {   # real screen pixels: includes the separate message-box window
    [void][Win]::SetWindowPos($h, [IntPtr](-1), $r.L, $r.T, 0, 0, 0x0001 -bor 0x0040)   # topmost, no move/size
    Start-Sleep -Milliseconds 300
    $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
    [void][Win]::SetWindowPos($h, [IntPtr](-2), $r.L, $r.T, 0, 0, 0x0001 -bor 0x0040)
  } else {
    $dc = $g.GetHdc(); [void][Win]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc)
  }
  $g.Dispose()
  $file = Join-Path $Out "$name.png"; $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  "captured $name"
}
function Start-App([string]$folder) {
  if ($folder) { $p = Start-Process $exe -ArgumentList "`"$folder`"" -PassThru } else { $p = Start-Process $exe -PassThru }
  for ($i = 0; $i -lt 40 -and $p.MainWindowHandle -eq 0; $i++) { Start-Sleep -Milliseconds 500; $p.Refresh() }
  [void][Win]::SetWindowPos($p.MainWindowHandle, [IntPtr]::Zero, 60, 20, $Width, $Height, 0x0040)
  Start-Sleep -Seconds 5   # first paint
  $p
}
function Win-Root($p) { $AE::FromHandle($p.MainWindowHandle) }
function Find-ByName($scope, [string]$name, $type = $null) {
  $all = $scope.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($e in $all) { if ($e.Current.Name -like $name -and (-not $type -or $e.Current.ControlType.ProgrammaticName -like "*$type")) { return $e } }
  throw "UI element '$name' not found"
}
function Click($e) {
  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  for ($node = $e; $node -and $node -ne $AE::RootElement; $node = $walker.GetParent($node)) {
    $inv = $null; $sel = $null; $tog = $null
    if ($node.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$inv)) { $inv.Invoke(); return }
    if ($node.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$sel)) { $sel.Select(); return }
    if ($node.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$tog)) { $tog.Toggle(); return }
    if ($node.Current.ControlType.ProgrammaticName -like '*Window') { break }
  }
  $r = $e.Current.BoundingRectangle
  [Mouse]::Click([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)); Start-Sleep -Milliseconds 300
}
function Click-Mouse($e) {   # real mouse click: UI Automation Invoke would block while a modal MessageBox is open
  $r = $e.Current.BoundingRectangle
  [Mouse]::Click([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)); Start-Sleep -Milliseconds 300
}
function Nav($p, [int]$index) {   # sidebar order: Scan, History, How it works
  $radios = (Win-Root $p).FindAll($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition ($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::RadioButton)))
  Click $radios[$index]; Start-Sleep -Milliseconds 700
}
function Scroll-Page($p, [int]$notches) {   # negative = down; real wheel events over the page content
  $r = New-Object Win+RECT; [void][Win]::GetWindowRect($p.MainWindowHandle, [ref]$r)
  $step = if ($notches -lt 0) { -1 } else { 1 }
  for ($i = 0; $i -lt [math]::Abs($notches); $i++) { [Mouse]::Wheel($r.L + 800, $r.T + 560, $step); Start-Sleep -Milliseconds 40 }
  Start-Sleep -Milliseconds 500
}
function Drop-Focus($p) {   # click the empty title bar so no control shows a keyboard-focus outline
  $r = New-Object Win+RECT; [void][Win]::GetWindowRect($p.MainWindowHandle, [ref]$r)
  [Mouse]::Click($r.L + 900, $r.T + 18); Start-Sleep -Milliseconds 300
}
function Stop-App($p) { if (-not $p.HasExited) { [void]$p.CloseMainWindow(); Start-Sleep 1; if (-not $p.HasExited) { $p.Kill() } } }
function Message-Box($p) {   # the app's confirmation MessageBox: an owned #32770 window, listed under its owner in UI Automation
  for ($i = 0; $i -lt 30; $i++) {
    $scopes = @($AE::RootElement) + @(Win-Root $p)
    foreach ($scope in $scopes) {
      foreach ($w in $scope.FindAll($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition ($AE::ClassNameProperty, '#32770')))) {
        if ($w.Current.ProcessId -eq $p.Id) { return $w }
      }
    }
    Start-Sleep -Milliseconds 300
  }
  throw 'confirmation dialog did not appear'
}


function Set-Text($e, [string]$text) {
  $vp = $null
  if ($e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp)) { $vp.SetValue($text) } else { throw 'element has no ValuePattern' }
  Start-Sleep -Milliseconds 300
}
function First-Edit($p) {
  (Win-Root $p).FindFirst($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition ($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)))
}
function Set-Theme($p, [string]$theme) {
  $want = (Get-Culture).TextInfo.ToTitleCase($theme)
  for ($i = 0; $i -lt 3 -and (Find-ByName (Win-Root $p) 'Theme:*').Current.Name -notlike "*$want*"; $i++) { Click (Find-ByName (Win-Root $p) 'Theme:*'); Start-Sleep -Milliseconds 700 }
}

$fixture = Join-Path ([IO.Path]::GetTempPath()) 'longpathfix-demo\Project Files'

function Make-File([string]$path) {
  $full = '\\?\' + $path
  [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($full))
  [IO.File]::WriteAllText($full, 'demo')
}

function Prepare {
  if (Test-Path 'X:') { cmd /c 'subst X: /d' | Out-Null }
  $base = Split-Path $fixture
  if (Test-Path ('\\?\' + $base)) { cmd /c "rd /s /q `"\\?\$base`"" | Out-Null }
  $deep = $fixture
  foreach ($n in 'Quarterly reports and supporting evidence for the audit committee', 'Regional breakdown by product family and customer segment', 'Archived drafts and reviewer comments kept for compliance', 'Final signed versions awaiting distribution to stakeholders') { $deep = Join-Path $deep $n }
  Make-File (Join-Path $deep 'Meeting minutes with action items and follow-ups from the steering committee review session.docx')
  Make-File (Join-Path $deep 'Supplier price comparison across all regions and currencies, including freight and duty estimates.xlsx')
  Make-File (Join-Path $fixture 'Budget 2026 draft .xlsx')
  Make-File (Join-Path $fixture 'Notes.txt.')
  Make-File (Join-Path $fixture 'CON.txt')
  Make-File (Join-Path $fixture 'Report Q3 (final).docx ')
  Make-File (Join-Path $fixture 'Photos\ Vacation beach.jpg')
  Make-File (Join-Path $fixture 'Photos\Holiday  .png')
  Make-File (Join-Path $fixture 'AUX.log')
  Make-File (Join-Path $fixture 'Readme.md')
  Make-File (Join-Path $fixture 'Invoices\Invoice 0001.pdf')
  cmd /c "subst X: `"$(Split-Path $fixture)`"" | Out-Null
}

function Cleanup {
  if (Test-Path 'X:') { cmd /c 'subst X: /d' | Out-Null }
  $base = Split-Path $fixture
  if (Test-Path ('\\?\' + $base)) { cmd /c "rd /s /q `"\\?\$base`"" | Out-Null }
}

function Run-Theme($theme) {
  Prepare
  $p = Start-App ''
  Set-Theme $p $theme
  Set-Text (First-Edit $p) 'X:\Project Files'
  Click-Mouse (Find-ByName (Win-Root $p) 'Scan' 'Button')
  Start-Sleep -Seconds 3
  Drop-Focus $p; Snap $p "scan-and-fix-$theme"
  Scroll-Page $p -8; Drop-Focus $p; Snap $p "plan-$theme"
  Scroll-Page $p 20
  Click-Mouse (Find-ByName (Win-Root $p) 'Rename' 'Button')
  $box = Message-Box $p
  Snap $p "rename-confirm-$theme" -Screen
  [void][Win]::SetForegroundWindow([IntPtr]$box.Current.NativeWindowHandle); Start-Sleep -Milliseconds 400
  [System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
  Start-Sleep -Seconds 3
  Check (Test-Path ('\\?\' + (Join-Path $fixture 'Readme.md'))) 'untouched file still exists'
  Check (-not (Test-Path ('\\?\' + (Join-Path $fixture 'CON.txt')))) 'reserved name was renamed'
  Scroll-Page $p 20; Drop-Focus $p; Snap $p "renamed-$theme"
  Nav $p 1; Drop-Focus $p; Snap $p "history-$theme"
  Nav $p 2; Drop-Focus $p; Snap $p "how-it-works-$theme"
  Stop-App $p
}

try {
  Prepare
  foreach ($theme in 'light', 'dark') { Run-Theme $theme }
  $checks | ForEach-Object { $_ }
}
finally {
  Get-Process LongPathFix -ErrorAction SilentlyContinue | ForEach-Object { try { $_.Kill() } catch {} }
  Cleanup
  if (Test-Path $dataDir) {
    Get-ChildItem $dataDir -Recurse -Force | Sort-Object FullName -Descending | Where-Object { $before -notcontains $_.FullName } | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    if (-not $dataExisted) { Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue }
  }
  "fixture removed, app data restored"
}
