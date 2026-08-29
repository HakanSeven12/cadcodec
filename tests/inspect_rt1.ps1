$src = "tests\issue51_rt_issues\civil_roundtrip1.dxf"
$text = [System.IO.File]::ReadAllText($src)
# Insert a reactors group into the WDFLT record (owner C = NOD)
$patched = $text.Replace(
    "0`r`nACDBDICTIONARYWDFLT`r`n  5`r`nE`r`n330`r`nC`r`n100`r`nAcDbDictionary",
    "0`r`nACDBDICTIONARYWDFLT`r`n  5`r`nE`r`n102`r`n{ACAD_REACTORS`r`n330`r`nC`r`n102`r`n}`r`n330`r`nC`r`n100`r`nAcDbDictionary"
)
"patched: $($text -ne $patched)"
[System.IO.File]::WriteAllText("tests\issue51_rt_issues\x3_reactors.dxf", $patched)
$scr = @"
_.LOGFILEON
_.FILEDIA 0
_.RECOVER
D:\GitHub\acadrust\tests\issue51_rt_issues\x3_reactors.dxf
_.LOGFILEOFF
_.QUIT
"@
Set-Content -Path "$env:TEMP\opencode\x3.scr" -Value $scr -Encoding Ascii
"script written"
