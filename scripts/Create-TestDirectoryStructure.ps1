# Create-TestDirectoryStructure.ps1
# Kes/kopyala/yapıştır testi için örnek dizin yapısı oluşturur.
# Kullanım: .\Create-TestDirectoryStructure.ps1
#          .\Create-TestDirectoryStructure.ps1 -Root "C:\TestFM"
#          .\Create-TestDirectoryStructure.ps1 -Root "C:\TestFM" -Clean  # Önce siler, sonra oluşturur

param(
    [string]$Root = (Join-Path $env:USERPROFILE "Desktop\FM-Test-Root"),
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

if ($Clean -and (Test-Path $Root)) {
    Write-Host "Removing existing: $Root"
    Remove-Item -Path $Root -Recurse -Force
}

New-Item -ItemType Directory -Path $Root -Force | Out-Null
Write-Host "Root: $Root"

# --- Klasörler ---
$folders = @(
    "Projeler",
    "Projeler\2024",
    "Projeler\2024\Q1",
    "Projeler\2024\Q2",
    "Projeler\2025",
    "Dökümanlar",
    "Dökümanlar\Faturalar",
    "Dökümanlar\Sözleşmeler",
    "Medya",
    "Medya\Müzik",
    "Medya\Görseller",
    "Yedek",
    "Boş_Klasör"
)

foreach ($f in $folders) {
    $path = Join-Path $Root $f
    New-Item -ItemType Directory -Path $path -Force | Out-Null
    Write-Host "  + $f"
}

# --- Dosyalar (içerikle) ---
$files = @(
    @{ Path = "README.txt"; Content = "FM kes/kopyala/yapıştır test kök dizini.`nOluşturulma: $(Get-Date -Format 'yyyy-MM-dd HH:mm')" },
    @{ Path = "Projeler\özet.txt"; Content = "2024 ve 2025 proje özeti" },
    @{ Path = "Projeler\2024\Q1\rapor.txt"; Content = "Q1 rapor içeriği" },
    @{ Path = "Projeler\2024\Q2\rapor.txt"; Content = "Q2 rapor içeriği" },
    @{ Path = "Projeler\2025\plan.txt"; Content = "2025 plan" },
    @{ Path = "Dökümanlar\notlar.txt"; Content = "Genel notlar" },
    @{ Path = "Dökümanlar\Faturalar\liste.txt"; Content = "Fatura listesi" },
    @{ Path = "Medya\liste.txt"; Content = "Medya indeksi" },
    @{ Path = "Yedek\son_yedek.txt"; Content = "Son yedek tarihi" }
)

foreach ($f in $files) {
    $path = Join-Path $Root $f.Path
    $dir = Split-Path $path -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    Set-Content -Path $path -Value $f.Content -Encoding UTF8
    Write-Host "  + $($f.Path)"
}

# --- Boş dosyalar (farklı uzantılar) ---
$empty = @(
    "Projeler\boş.json",
    "Projeler\config.ini",
    "Dökümanlar\taslak.docx.txt",
    "Medya\playlist.m3u.txt",
    "Yedek\flag.txt"
)
foreach ($rel in $empty) {
    $path = Join-Path $Root $rel
    $dir = Split-Path $path -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    New-Item -ItemType File -Path $path -Force | Out-Null
    Write-Host "  + $rel (empty)"
}

# --- Alt klasörde çoklu öğe (tek seçim / çoklu seçim testi) ---
$batch = @(
    "Yedek\a.txt", "Yedek\b.txt", "Yedek\c.txt",
    "Yedek\alt\1.txt", "Yedek\alt\2.txt"
)
foreach ($rel in $batch) {
    $path = Join-Path $Root $rel
    $dir = Split-Path $path -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    Set-Content -Path $path -Value "Dosya: $(Split-Path $rel -Leaf)" -Encoding UTF8
    Write-Host "  + $rel"
}

Write-Host ""
Write-Host "Done. Open in FM: $Root"
Write-Host "Try: copy/paste between Projeler, Dökümanlar, Medya, Yedek; cut from one folder and paste into another."
