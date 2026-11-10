# Builds llama-server for Windows as a single static exe with the Vulkan backend and puts it where
# Tauri's `bundle.externalBin` expects the translation engine's sidecar (doc-1 4.1, 4.2). The
# Windows counterpart of build-llama-server.sh.
#
# Built from source rather than taken from the GitHub release: the release's win-vulkan zip splits
# llama-server into llama.dll and several ggml-*.dll, and externalBin ships exactly one file.
#
# ASCII only: Windows PowerShell 5.1 reads a script without a BOM in the ANSI code page.
#
# Needs Visual Studio (or its Build Tools) with the C++ workload, which also brings CMake, and the
# Vulkan SDK for glslc and the Vulkan headers. Runs under Windows PowerShell 5.1 and PowerShell 7.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# The same tag and commit as build-llama-server.sh, so both platforms run the engine TASK-1 measured.
$tag = 'b12110'
$commit = 'cd959bae9'

function Fail([string]$message) {
    Write-Error "build-llama-server.ps1: $message" -ErrorAction Continue
    exit 1
}

# $ErrorActionPreference does not cover a native program's exit code.
function Invoke-Native([string]$program, [string[]]$arguments) {
    & $program @arguments
    if ($LASTEXITCODE -ne 0) {
        Fail "$program exited with $LASTEXITCODE"
    }
}

$root = Split-Path -Parent $PSScriptRoot
$triple = (rustc -vV | Select-String -Pattern '^host: (.+)$').Matches[0].Groups[1].Value
if ($triple -ne 'x86_64-pc-windows-msvc') {
    Fail "no build defined for $triple"
}
$out = Join-Path $root "src-tauri\binaries\llama-server-$triple.exe"
# Outside the repository, as in build-llama-server.sh: llama.cpp carries its own Svelte UI, which
# svelte-check and other workspace-wide tools would otherwise find and try to load.
$cacheHome = if ($env:XDG_CACHE_HOME) { $env:XDG_CACHE_HOME } else { Join-Path $HOME '.cache' }
$work = Join-Path $cacheHome "tiny-translator\llama.cpp-$tag"

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) {
    Fail 'Visual Studio is not installed (vswhere.exe not found)'
}
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) {
    Fail 'no Visual Studio with the C++ x64 tools found'
}

$onPath = Get-Command cmake -ErrorAction SilentlyContinue
if ($onPath) {
    $cmake = $onPath.Source
} else {
    $cmake = Join-Path $vs 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'
    if (-not (Test-Path $cmake)) {
        Fail 'CMake not found on PATH or in Visual Studio'
    }
}

# The SDK installer sets VULKAN_SDK machine-wide, which a shell opened before the install lacks.
if (-not $env:VULKAN_SDK) {
    $env:VULKAN_SDK = [Environment]::GetEnvironmentVariable('VULKAN_SDK', 'Machine')
}
if (-not $env:VULKAN_SDK -or -not (Test-Path (Join-Path $env:VULKAN_SDK 'Bin\glslc.exe'))) {
    Fail 'the Vulkan SDK is not installed (winget install KhronosGroup.VulkanSDK)'
}

if (-not (Test-Path (Join-Path $work '.git'))) {
    Invoke-Native git @('-c', 'advice.detachedHead=false', 'clone', '--quiet', '--depth', '1', '--branch', $tag, 'https://github.com/ggml-org/llama.cpp.git', $work)
}
$actual = git -C $work rev-parse --short=9 HEAD
if ($actual -ne $commit) {
    Fail "$tag is $actual, expected $commit"
}

# Built without llama.cpp's Web UI, as in build-llama-server.sh: the app passes --no-webui, and an
# embedded UI would add npm dependencies to the shipped licence notice. llama.cpp embeds whatever
# an earlier build left in the build tree's dist/, so that is removed first.
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $work 'build\tools\ui\dist'), (Join-Path $work 'build\tools\ui\.ui-stamp')

# GGML_NATIVE=OFF limits the CPU code to AVX2: a native build on a Zen 4 machine (the Ryzen Z1) would
# use AVX-512, which Zen 3 and older lack; the model runs on the GPU, so the CPU path's speed hardly
# matters. GGML_OPENMP=OFF and the static MSVC runtime (MultiThreaded) keep vcomp140.dll and the
# VC++ runtime DLLs out, which a machine without the redistributable lacks. LLAMA_CURL=OFF and
# LLAMA_OPENSSL=OFF: the engine reads a local GGUF and must not reach the network (R1).
Invoke-Native $cmake @(
    '-S', $work, '-B', (Join-Path $work 'build'), '-A', 'x64',
    '-DCMAKE_BUILD_TYPE=Release',
    '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded',
    '-DBUILD_SHARED_LIBS=OFF',
    '-DGGML_VULKAN=ON',
    '-DGGML_NATIVE=OFF',
    '-DGGML_OPENMP=OFF',
    '-DGGML_RPC=OFF',
    '-DLLAMA_CURL=OFF',
    '-DLLAMA_OPENSSL=OFF',
    '-DLLAMA_BUILD_TESTS=OFF',
    '-DLLAMA_BUILD_EXAMPLES=OFF',
    '-DLLAMA_BUILD_UI=OFF',
    '-DLLAMA_USE_PREBUILT_UI=OFF'
)
Invoke-Native $cmake @('--build', (Join-Path $work 'build'), '--config', 'Release', '--target', 'llama-server', '--parallel')
$built = Join-Path $work 'build\bin\Release\llama-server.exe'

if (Select-String -Quiet -SimpleMatch 'asset_0' (Join-Path $work 'build\tools\ui\ui.cpp')) {
    Fail 'llama-server embeds a Web UI; it must be built without one'
}

# The sidecar ships alone, so it may load only what every Windows machine has: DLLs of the OS
# itself, and the Vulkan loader, which every GPU driver installs. A DLL in System32 is not enough:
# the VC++ redistributable puts its DLLs there too, so an OS DLL is told by its product name.
$dumpbin = Get-ChildItem (Join-Path $vs 'VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe') |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $dumpbin) {
    Fail 'dumpbin.exe not found in Visual Studio'
}
$listing = & $dumpbin.FullName /nologo /dependents $built
if ($LASTEXITCODE -ne 0) {
    Fail "dumpbin exited with $LASTEXITCODE"
}
$dlls = $listing | Where-Object { $_ -match '^\s+\S+\.dll\s*$' } | ForEach-Object { $_.Trim() }
if (-not $dlls) {
    Fail 'dumpbin listed no DLLs; its output format may have changed'
}
$system32 = Join-Path $env:SystemRoot 'System32'
$foreign = @($dlls | Where-Object {
        $dll = $_
        if ($dll -ieq 'vulkan-1.dll' -or $dll -like 'api-ms-win-*') {
            return $false
        }
        $path = Join-Path $system32 $dll
        -not ((Test-Path $path) -and (Get-Item $path).VersionInfo.ProductName -match 'Windows')
    })
if ($foreign.Count -gt 0) {
    Fail "llama-server loads DLLs outside the OS: $($foreign -join ', ')"
}
Write-Host "llama-server loads: $($dlls -join ', ')"

New-Item -ItemType Directory -Force (Split-Path -Parent $out) | Out-Null
Copy-Item $built $out -Force
Invoke-Native $out @('--version')
Write-Host "built $out"
