param(
  [string]$Source = (Join-Path $PSScriptRoot '../src-tauri/windows/nova-installer-art.png')
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

function Save-InstallerBitmap {
  param(
    [System.Drawing.Image]$Image,
    [int]$Width,
    [int]$Height,
    [string]$Destination
  )

  $bitmap = New-Object System.Drawing.Bitmap $Width, $Height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $scale = [Math]::Max($Width / $Image.Width, $Height / $Image.Height)
    $drawWidth = [int][Math]::Ceiling($Image.Width * $scale)
    $drawHeight = [int][Math]::Ceiling($Image.Height * $scale)
    $left = [int](($Width - $drawWidth) / 2)
    $top = [int](($Height - $drawHeight) / 2)
    $graphics.DrawImage($Image, $left, $top, $drawWidth, $drawHeight)
    $bitmap.Save($Destination, [System.Drawing.Imaging.ImageFormat]::Bmp)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

$image = [System.Drawing.Image]::FromFile((Resolve-Path -LiteralPath $Source))
try {
  $windows = Join-Path $PSScriptRoot '../src-tauri/windows'
  Save-InstallerBitmap $image 164 314 (Join-Path $windows 'sidebar.bmp')
  Save-InstallerBitmap $image 150 57 (Join-Path $windows 'header.bmp')
} finally {
  $image.Dispose()
}
