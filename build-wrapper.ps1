$ErrorActionPreference = "Stop"

$root = $PSScriptRoot
$project = Join-Path $root "Wrapper\Wrapper.csproj"
$output = Join-Path $root "dist"

# Garante que vswhere está no PATH para o Native AOT linker
$vswherePath = "C:\Program Files (x86)\Microsoft Visual Studio\Installer"
if (Test-Path $vswherePath) {
    $env:PATH = "$vswherePath;$env:PATH"
}

Write-Host "Building Wrapper (Native AOT)..."
dotnet publish $project -c Release -o $output

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed."
    exit 1
}

Write-Host "Done. Output: $output\L2Editor.exe"