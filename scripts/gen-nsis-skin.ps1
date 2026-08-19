# 生成 AlpeHuez NSIS 安装包皮肤切图（Liquid Glass 规范）
# sidebarImage: 164x314 BMP — 顶部纯白 #FFFFFF → 底部微蓝 #E8F4FD 垂直渐变 + 中央 3D 蓝色 A logo（带投影）
# headerImage:  150x57  BMP — 纯白底 + 右侧横版 logo（蓝 A + 深灰 AlpeHuez 文字）
Add-Type -AssemblyName System.Drawing

$iconsDir = Join-Path $PSScriptRoot '..\src-tauri\icons'
$iconsDir = [System.IO.Path]::GetFullPath($iconsDir)
New-Item -ItemType Directory -Force -Path $iconsDir | Out-Null

$blue = [System.Drawing.Color]::FromArgb(255, 0, 135, 235)      # #0087EB
$deep = [System.Drawing.Color]::FromArgb(255, 51, 51, 51)        # #333333 深灰
$white = [System.Drawing.Color]::White

# ---------- sidebarImage 164x314 ----------
$w = 164; $h = 314
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

# 垂直渐变：顶部纯白 #FFFFFF → 底部微蓝 #E8F4FD（与右侧纯白向导区无缝融合）
$rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    $white,
    [System.Drawing.Color]::FromArgb(255, 232, 244, 253),
    90.0)
$g.FillRectangle($grad, $rect)

# 玻璃质感圆（半透明叠加，模拟 3D 玻璃厚度，集中在底部蓝色区）
function Draw-GlassCircle($g, $cx, $cy, $r, $alpha, $color) {
    $c = [System.Drawing.Color]::FromArgb($alpha, $color)
    $b = New-Object System.Drawing.SolidBrush($c)
    $g.FillEllipse($b, ($cx - $r), ($cy - $r), (2 * $r), (2 * $r))
    $b.Dispose()
    $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(120, 255, 255, 255), 2)
    $g.DrawArc($pen, ($cx - $r + 3), ($cy - $r + 3), (2 * $r - 6), (2 * $r - 6), 200, 120)
    $pen.Dispose()
}
Draw-GlassCircle $g 40 250 46 70 $white
Draw-GlassCircle $g 118 268 34 55 $blue
Draw-GlassCircle $g 78 292 26 90 $white

# 中央：3D 蓝色 A logo（先画偏移投影，再画主体，形成轻微立体感）
$font = New-Object System.Drawing.Font('Segoe UI', 64, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$sf = New-Object System.Drawing.StringFormat
$sf.Alignment = [System.Drawing.StringAlignment]::Center
$sf.LineAlignment = [System.Drawing.StringAlignment]::Center
$logoRect = New-Object System.Drawing.RectangleF(0, 100, $w, 120)
$shadowBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(70, 0, 0, 0))
$shadowRect = New-Object System.Drawing.RectangleF(2, 103, $w, 120)
$g.DrawString('A', $font, $shadowBrush, $shadowRect, $sf)
$sb = New-Object System.Drawing.SolidBrush($blue)
$g.DrawString('A', $font, $sb, $logoRect, $sf)
$sb.Dispose(); $shadowBrush.Dispose(); $font.Dispose(); $sf.Dispose()

# 底部细字
$font2 = New-Object System.Drawing.Font('Segoe UI', 9, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$sb2 = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(120, 0, 135, 235))
$sf2 = New-Object System.Drawing.StringFormat
$sf2.Alignment = [System.Drawing.StringAlignment]::Center
$g.DrawString('AlpeHuez', $font2, $sb2, (New-Object System.Drawing.RectangleF(0, 150, $w, 20)), $sf2)
$sb2.Dispose(); $font2.Dispose(); $sf2.Dispose()

$g.Dispose()
$bmp.Save((Join-Path $iconsDir 'nsis-sidebar.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$bmp.Dispose()
Write-Host 'Wrote nsis-sidebar.bmp (164x314)'

# ---------- headerImage 150x57 ----------
$w2 = 150; $h2 = 57
$bmp2 = New-Object System.Drawing.Bitmap($w2, $h2)
$g2 = [System.Drawing.Graphics]::FromImage($bmp2)
$g2.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g2.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$g2.Clear($white)

# 右侧横版 logo：蓝 A + 深灰 AlpeHuez 文字，整体靠右对齐
$fontA = New-Object System.Drawing.Font('Segoe UI', 24, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$sbA = New-Object System.Drawing.SolidBrush($blue)
$g2.DrawString('A', $fontA, $sbA, 78, 10)
$sbA.Dispose(); $fontA.Dispose()

$fontT = New-Object System.Drawing.Font('Segoe UI', 12, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$sbT = New-Object System.Drawing.SolidBrush($deep)
$g2.DrawString('AlpeHuez', $fontT, $sbT, 100, 19)
$sbT.Dispose(); $fontT.Dispose()

$g2.Dispose()
$bmp2.Save((Join-Path $iconsDir 'nsis-header.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$bmp2.Dispose()
Write-Host 'Wrote nsis-header.bmp (150x57)'
