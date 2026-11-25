# Prueba del paquete MSIX ya instalado: lanza `iureocr.exe --mcp` por su alias (con la
# identidad del paquete, como lo haría Claude Desktop) y le pide, por MCP, la lista de
# idiomas de Tesseract y el OCR de una imagen generada aquí. Comprueba así que Tesseract
# y sus modelos van dentro del paquete y que la app los encuentra.
#
#   pwsh scripts/probar-msix.ps1 -Alias "$env:LOCALAPPDATA\Microsoft\WindowsApps\iureocr.exe"
param(
  [Parameter(Mandatory = $true)][string]$Alias,
  [string]$Dir = (Join-Path ([System.IO.Path]::GetTempPath()) 'iureocr-msix-prueba')
)
$ErrorActionPreference = 'Stop'

foreach ($p in @("$env:ProgramFiles\Tesseract-OCR", "${env:ProgramFiles(x86)}\Tesseract-OCR")) {
  if (Test-Path $p) { Write-Output "::warning::Hay un Tesseract del sistema en $p; la prueba podría no usar el del paquete." }
}

# Imagen de prueba: texto negro sobre blanco, a buen tamaño.
New-Item -ItemType Directory -Force $Dir | Out-Null
$png = Join-Path $Dir 'prueba.png'
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap 1400, 300
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::White)
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
$font = New-Object System.Drawing.Font 'Arial', 64
$g.DrawString('Hola mundo 12345', $font, [System.Drawing.Brushes]::Black, 40, 90)
$g.Dispose()
$bmp.SetResolution(300, 300)
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "Imagen de prueba: $png"

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Alias
$psi.Arguments = '--mcp'
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
$proc = [System.Diagnostics.Process]::Start($psi)
$errTask = $proc.StandardError.ReadToEndAsync()

function Send($obj) {
  $line = $obj | ConvertTo-Json -Depth 10 -Compress
  $proc.StandardInput.WriteLine($line)
  $proc.StandardInput.Flush()
}

# Lee respuestas hasta la del id pedido (las notificaciones de progreso se ignoran).
function Receive([int]$id, [int]$seconds) {
  $limit = (Get-Date).AddSeconds($seconds)
  while ((Get-Date) -lt $limit) {
    $task = $proc.StandardOutput.ReadLineAsync()
    $left = [int]($limit - (Get-Date)).TotalMilliseconds
    if ($left -le 0 -or -not $task.Wait($left)) { break }
    $line = $task.Result
    if ($null -eq $line) { throw "El servidor MCP terminó sin responder al id $id" }
    $msg = $line | ConvertFrom-Json
    if ($msg.id -eq $id) { return $msg }
  }
  throw "Sin respuesta del servidor MCP al id $id en $seconds s"
}

function ToolText($msg) {
  if ($msg.error) { throw "Error MCP: $($msg.error | ConvertTo-Json -Compress)" }
  if ($msg.result.isError) { throw "La herramienta falló: $($msg.result.content | ConvertTo-Json -Compress -Depth 5)" }
  ($msg.result.content | Where-Object { $_.type -eq 'text' } | ForEach-Object { $_.text }) -join "`n"
}

try {
  Send @{ jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{ protocolVersion = '2025-06-18'; capabilities = @{}; clientInfo = @{ name = 'ci-msix'; version = '1' } } }
  $init = Receive 1 60
  Write-Output "initialize: $($init.result.serverInfo | ConvertTo-Json -Compress)"
  Send @{ jsonrpc = '2.0'; method = 'notifications/initialized' }

  Send @{ jsonrpc = '2.0'; id = 2; method = 'tools/call'; params = @{ name = 'ocr_languages'; arguments = @{} } }
  $langs = ToolText (Receive 2 60)
  Write-Output "ocr_languages: $langs"
  if ($langs -notmatch '"spa"' -or $langs -notmatch '"eng"') { throw "Faltan los modelos spa/eng dentro del paquete" }

  Send @{ jsonrpc = '2.0'; id = 3; method = 'tools/call'; params = @{ name = 'ocr_file'; arguments = @{ path = $png; output_dir = $Dir; languages = 'spa+eng'; force = $true } } }
  $ocr = ToolText (Receive 3 300)
  Write-Output "ocr_file: $ocr"
  if ($ocr -notmatch 'mundo' -or $ocr -notmatch '12345') { throw "El OCR no reconoció el texto de prueba" }
  Write-Output 'OCR dentro del paquete MSIX: correcto'
} finally {
  try { $proc.StandardInput.Close() } catch {}
  if (-not $proc.WaitForExit(20000)) { $proc.Kill() }
  $stderr = $errTask.Result
  if ($stderr) { Write-Output "--- stderr del servidor MCP ---`n$stderr" }
}
