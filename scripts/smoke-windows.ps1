param([Parameter(Mandatory=$true)][string]$Target)
$ErrorActionPreference = 'Stop'

$runtimeRoots = @(
  [Environment]::GetFolderPath('ProgramFilesX86'),
  [Environment]::GetFolderPath('ProgramFiles'),
  [Environment]::GetFolderPath('LocalApplicationData')
)
$runtimeInstalled = $false
foreach ($root in $runtimeRoots) {
  if ($root -and (Test-Path (Join-Path $root 'Microsoft/EdgeWebView/Application'))) {
    $runtimeInstalled = $true
  }
}
if (-not $runtimeInstalled) {
  $bootstrap = Join-Path $env:RUNNER_TEMP 'MicrosoftEdgeWebview2Setup.exe'
  Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $bootstrap
  $signature = Get-AuthenticodeSignature $bootstrap
  if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notlike '*Microsoft Corporation*') {
    throw 'The WebView2 bootstrapper signature is invalid'
  }
  $runtime = Start-Process $bootstrap -ArgumentList '/silent /install' -PassThru
  if (-not $runtime.WaitForExit(180000)) { $runtime.Kill(); throw 'WebView2 setup timed out' }
  if ($runtime.ExitCode -ne 0) { throw "WebView2 setup failed: $($runtime.ExitCode)" }
}

function Invoke-Installer([string]$Executable, [string]$Arguments, [string]$Operation) {
  $process = Start-Process $Executable -ArgumentList $Arguments -PassThru -Wait:$false
  try {
    if (-not $process.WaitForExit(120000)) { $process.Kill(); throw "$Operation timed out" }
    if ($process.ExitCode -notin @(0, 3010)) { throw "$Operation failed: $($process.ExitCode)" }
  } finally { $process.Dispose() }
}

function Assert-AppStarts([string]$Directory) {
  $executable = Join-Path $Directory 'prism-relay.exe'
  foreach ($file in @('prism-relay.exe', 'LICENSE', 'THIRD_PARTY_NOTICES.txt')) {
    if (-not (Test-Path (Join-Path $Directory $file))) { throw "Installed file is missing: $file" }
  }
  $app = Start-Process $executable -PassThru
  try {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    $windowFound = $false
    while ([DateTime]::UtcNow -lt $deadline) {
      $app.Refresh()
      if ($app.HasExited) { throw "The installed app exited during startup: $($app.ExitCode)" }
      if ($app.MainWindowHandle -ne 0 -and $app.MainWindowTitle -eq 'Prism Relay') {
        $windowFound = $true
        break
      }
      Start-Sleep -Milliseconds 250
    }
    if (-not $windowFound) { throw 'The installed app did not create its main window' }
    Start-Sleep -Seconds 2
    $app.Refresh()
    if ($app.HasExited) { throw 'The installed app exited after creating its window' }
    Write-Output 'Installed application started and its main window is available'
  } finally {
    $app.Refresh()
    if (-not $app.HasExited) {
      $null = $app.CloseMainWindow()
      if (-not $app.WaitForExit(5000)) { $app.Kill(); $app.WaitForExit() }
    }
    $app.Dispose()
  }
}

function Assert-AppRemoved([string]$Directory) {
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  $executable = Join-Path $Directory 'prism-relay.exe'
  while ((Test-Path $executable) -and [DateTime]::UtcNow -lt $deadline) {
    Start-Sleep -Milliseconds 250
  }
  if (Test-Path $executable) { throw 'The test installation was not removed' }
}

$msis = @(Get-ChildItem "src-tauri/target/$Target/release/bundle/msi/*.msi")
$setups = @(Get-ChildItem "src-tauri/target/$Target/release/bundle/nsis/*-setup.exe")
if ($msis.Count -ne 1 -or $setups.Count -ne 1) { throw 'Expected one MSI and one setup executable' }

$msiDirectory = Join-Path $env:RUNNER_TEMP 'PrismRelayMsiSmoke'
$msiPath = $msis[0].FullName
try {
  Invoke-Installer 'msiexec.exe' "/i `"$msiPath`" /qn /norestart INSTALLDIR=`"$msiDirectory`"" 'MSI installation'
  Assert-AppStarts $msiDirectory
} finally {
  Invoke-Installer 'msiexec.exe' "/x `"$msiPath`" /qn /norestart" 'MSI uninstallation'
  Assert-AppRemoved $msiDirectory
}

$nsisDirectory = Join-Path $env:RUNNER_TEMP 'PrismRelayNsisSmoke'
try {
  Invoke-Installer $setups[0].FullName "/S /D=$nsisDirectory" 'Setup installation'
  Assert-AppStarts $nsisDirectory
} finally {
  $uninstaller = Join-Path $nsisDirectory 'uninstall.exe'
  if (Test-Path $uninstaller) {
    Invoke-Installer $uninstaller "/S _?=$nsisDirectory" 'Setup uninstallation'
    Assert-AppRemoved $nsisDirectory
  }
}
