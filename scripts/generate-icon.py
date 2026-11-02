"""Regenerate the checked-in Windows icon/tray raster from its simple vector geometry.
Development only: requires Pillow. The application has no Python dependency.
"""
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1]
scale = 8
image = Image.new('RGBA', (64 * scale, 64 * scale))
draw = ImageDraw.Draw(image)
def box(values): return tuple(round(v * scale) for v in values)
draw.rounded_rectangle(box((3, 3, 61, 61)), radius=14*scale, fill='#2e3440')
draw.rounded_rectangle(box((11, 14, 53, 50)), radius=6*scale,
                       fill='#3b4252', outline='#88c0d0', width=round(2.5*scale))
draw.line(box((21, 25, 29, 32, 21, 39)), fill='#e5e9f0', width=round(3.5*scale), joint='curve')
draw.line(box((35, 39, 44, 39)), fill='#e5e9f0', width=round(3.5*scale))
for x,y in ((21,25),(21,39),(35,39),(44,39)):
    draw.ellipse(box((x-1.75,y-1.75,x+1.75,y+1.75)), fill='#e5e9f0')
image.save(root/'assets/app.ico', sizes=[(n,n) for n in (16,20,24,32,40,48,64,128,256)])
(root/'assets/app-icon-32.rgba').write_bytes(image.resize((32,32), Image.Resampling.LANCZOS).tobytes())
