param(
    [string]$Root = 'target/entity-matrix-isolated',
    [string]$Atlas = 'target/entity-atlas',
    [string]$Version = 'AC1021',
    [ValidateSet('ACAD2027','BCAD')][string]$Engine = 'ACAD2027',
    [ValidateSet('dwg','dxf_ascii','dxf_binary')][string]$Format = 'dwg',
    [string]$Case = '*',
    [int]$TimeoutSeconds = 15
)
$ErrorActionPreference = 'Stop'
& cargo build --example entity_atlas --quiet
if ($LASTEXITCODE -ne 0) { throw 'Failed to build entity atlas' }
$catalog = @(Get-Content (Join-Path $Atlas 'manifest.json') -Raw | ConvertFrom-Json)
$drawing = $catalog | Where-Object version -EQ $Version | Select-Object -First 1
if (-not $drawing) { throw "No atlas for $Version" }
foreach ($entity in $drawing.cases | Where-Object { $_.included -and $_.name -like $Case }) {
    $destination = Join-Path $Root $entity.name
    & target/debug/examples/entity_atlas.exe $destination "--case=$($entity.name)" "--version=$Version" --exact-case | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate $($entity.name)" }
    Write-Output "ENTITY $($entity.name)"
    & (Join-Path $PSScriptRoot 'validate_entity_atlas.ps1') -Root $destination -Engine $Engine -Filter "*_$Format" -TimeoutSeconds $TimeoutSeconds
}
