$scr = @"
_.FILEDIA 0
_.OPEN
D:\GitHub\acadrust\tests\issue51_rt_issues\civil_roundtrip1.dxf
_.QUIT
_N
"@
Set-Content -Path "$env:TEMP\opencode\open_test.scr" -Value $scr -Encoding Ascii
$p = Start-Process -FilePath "D:\Bricsys\BricsCAD\bricscad.exe" -ArgumentList '/nologo', '/b', "$env:TEMP\opencode\open_test.scr" -PassThru
$exited = $p.WaitForExit(75000)
if (-not $exited) { "RESULT: HUNG (modal present)"; $p.Kill(); Get-Process -Name bricscad -ErrorAction SilentlyContinue | Stop-Process -Force } else { "RESULT: EXITED CLEANLY" }
