$InstallDir = "$env:LOCALAPPDATA\VibeCody"
Unregister-ScheduledTask -TaskName "VibeCody" -Confirm:$false -ErrorAction SilentlyContinue
$Path = [Environment]::GetEnvironmentVariable("PATH", "User")
[Environment]::SetEnvironmentVariable("PATH", ($Path -replace [regex]::Escape("$InstallDir;"), ""), "User")
Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "VibeCody uninstalled." -ForegroundColor Green
