---
triggers: ["PowerShell", "pwsh", "PSScript", "cmdlet", "PowerShell module", "PowerShell automation", "Windows automation", "Azure PowerShell", "PowerShell Core"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["pwsh"]
category: powershell
---

# PowerShell

When writing PowerShell scripts and automation:

1. Use PowerShell 7+ (cross-platform): `pwsh` on Linux/macOS/Windows — use `$PSVersionTable.PSVersion` to check version; avoid Windows PowerShell 5.1 for new projects unless required for legacy module compatibility.
2. Follow Verb-Noun naming convention: `Get-Process`, `Set-Content`, `New-Item`, `Remove-Service` — use approved verbs (`Get-Verb` to list them); PascalCase for function names; use `$camelCase` for variables.
3. Use the pipeline for data transformation: `Get-Process | Where-Object { $_.CPU -gt 100 } | Sort-Object CPU -Descending | Select-Object -First 10 Name, CPU` — each cmdlet passes objects (not text) to the next.
4. Write advanced functions with `[CmdletBinding()]`: `function Get-SystemInfo { [CmdletBinding()] param([Parameter(Mandatory)][string]$ComputerName) ... }` — enables `-Verbose`, `-Debug`, `-ErrorAction`, `-WhatIf` support automatically.
5. Handle errors with `try/catch/finally`: `try { Get-Content $path -ErrorAction Stop } catch [System.IO.FileNotFoundException] { Write-Warning "File not found: $path" } catch { Write-Error $_.Exception.Message }` — use `-ErrorAction Stop` to make non-terminating errors catchable.
6. Use `ForEach-Object -Parallel` for concurrent operations (PowerShell 7+): `1..100 | ForEach-Object -Parallel { Invoke-WebRequest "https://api.example.com/$_" } -ThrottleLimit 10` — controls concurrency with `-ThrottleLimit`.
7. Manage remote systems: `Invoke-Command -ComputerName Server01 -ScriptBlock { Get-Service }` — use `Enter-PSSession` for interactive remoting; configure with `Enable-PSRemoting`; use SSH transport on Linux.
8. Use modules for reusable code: `New-ModuleManifest -Path MyModule.psd1`; export functions with `Export-ModuleMember`; publish to PowerShell Gallery with `Publish-Module`; install with `Install-Module ModuleName`.
9. Work with structured data: `$json = Get-Content data.json | ConvertFrom-Json`; `$csv = Import-Csv data.csv`; `$xml = [xml](Get-Content config.xml)` — PowerShell natively handles JSON, CSV, XML with object conversion.
10. Use Pester for testing: `Describe 'Get-SystemInfo' { It 'returns hostname' { Get-SystemInfo | Should -Be $env:COMPUTERNAME } }` — run with `Invoke-Pester`; use `Mock` for stubbing cmdlets; `BeforeAll`/`AfterAll` for setup/teardown.
11. For Azure automation: `Connect-AzAccount`; `Get-AzVM | Where-Object {$_.PowerState -eq 'VM running'} | Stop-AzVM -Force` — use Az module for resource management; use Azure Automation runbooks for scheduled tasks.
12. Script security: use `Set-ExecutionPolicy RemoteSigned` (minimum); sign scripts with `Set-AuthenticodeSignature`; use `SecureString` for passwords: `$cred = Get-Credential`; never store passwords in plain text.
