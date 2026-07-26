$candidates = @(
    'C:\Program Files (x86)\NSIS\makensis.exe',
    'C:\Program Files\NSIS\makensis.exe'
)
$makensis = $null
foreach ($p in $candidates) {
    if (Test-Path $p) { $makensis = $p; Write-Host ('FOUND: ' + $p); break }
}
if (-not $makensis) {
    $w = Get-Command makensis -ErrorAction SilentlyContinue
    if ($w) { $makensis = $w.Source; Write-Host ('PATH: ' + $w.Source) }
}
if (-not $makensis) { Write-Host 'MAKENSIS NOT FOUND'; exit 1 }

$nsi = 'C:\Users\YLW\Documents\PJ\WinSLA\installer\winsla-installer.nsi'
Write-Host 'Compiling installer...'
& $makensis $nsi
Write-Host ('makensis exit: ' + $LASTEXITCODE)
