from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import os

root = Path(__file__).resolve().parents[1]
output = root / 'docs/images'
scale = 2
fonts = Path(os.environ.get('WINDIR', 'C:/Windows')) / 'Fonts'

def font(size, bold=False):
    path = fonts / ('segoeuib.ttf' if bold else 'segoeui.ttf')
    if not path.exists():
        path = Path('/usr/share/fonts/truetype/dejavu') / ('DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf')
    return ImageFont.truetype(str(path), size * scale)

def text(draw, x, y, value, size, color, bold=False):
    draw.text((int(x * scale), int(y * scale)), value, font=font(size, bold), fill=color, anchor='lt')

def box(draw, coordinates, radius, fill, outline=None, width=1):
    draw.rounded_rectangle(tuple(int(v * scale) for v in coordinates), radius * scale, fill=fill, outline=outline, width=width * scale)

def line(draw, points, color, width=1):
    draw.line(tuple((int(x * scale), int(y * scale)) for x, y in points), fill=color, width=width * scale, joint='curve')

def save_gif(frames, durations, destination):
    samples = Image.new('RGB', (frames[0].width * 4, frames[0].height))
    for i, index in enumerate([0, len(frames)//4, len(frames)//2, len(frames)*3//4]):
        samples.paste(frames[index].convert('RGB'), (i * frames[0].width, 0))
    palette = samples.quantize(colors=256)
    frames = [frame.convert('RGB').quantize(palette=palette, dither=Image.Dither.NONE) for frame in frames]
    frames[0].save(destination, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True, disposal=2)

logo = Image.open(root / 'assets/aireluma.png').convert('RGBA')
base = Image.new('RGB', (1280 * scale, 360 * scale), '#101014')
draw = ImageDraw.Draw(base)
base.paste(logo.resize((76 * scale, 76 * scale), Image.Resampling.LANCZOS), (64 * scale, 72 * scale), logo.resize((76 * scale, 76 * scale), Image.Resampling.LANCZOS))
text(draw, 164, 62, 'Aireluma', 64, '#f5f3fc', True)
text(draw, 68, 166, 'Your iPhone. On your desktop.', 27, '#d0cadc')
text(draw, 70, 216, 'WINDOWS + LINUX   /   AIRPLAY   /   LOCAL NETWORK', 13, '#9f95b1')
box(draw, (68, 273, 246, 306), 8, '#242030')
text(draw, 81, 283, 'UP TO 4K  ·  60 FPS', 13, '#c7b8ef')
box(draw, (966, 65, 1200, 230), 14, '#1d1928', '#5e5473')
box(draw, (976, 75, 1190, 220), 8, '#282032')
line(draw, [(1083, 230), (1083, 260)], '#786a92', 2)
line(draw, [(1047, 260), (1119, 260)], '#786a92', 2)
base.paste(logo.resize((66 * scale, 66 * scale), Image.Resampling.LANCZOS), (1050 * scale, 115 * scale), logo.resize((66 * scale, 66 * scale), Image.Resampling.LANCZOS))
box(draw, (802, 84, 890, 264), 18, '#19151f', '#a498bd')
box(draw, (809, 92, 883, 256), 12, '#282032')
box(draw, (831, 97, 861, 104), 3, '#101014')
box(draw, (833, 246, 860, 249), 1, '#aea0c7')
base.paste(logo.resize((42 * scale, 42 * scale), Image.Resampling.LANCZOS), (825 * scale, 153 * scale), logo.resize((42 * scale, 42 * scale), Image.Resampling.LANCZOS))
line(draw, [(901, 173), (928, 173), (928, 148), (954, 148)], '#443a56', 2)
text(draw, 1038, 283, 'Windows · Linux', 14, '#9f95b1')
frames = []
for frame in range(42):
    image = base.copy()
    draw = ImageDraw.Draw(image)
    progress = (frame / 42) * 78
    if progress <= 27:
        x, y = 901 + progress, 173
    elif progress <= 52:
        x, y = 928, 173 - (progress - 27)
    else:
        x, y = 928 + progress - 52, 148
    radius = 3.0
    draw.ellipse(((x-radius)*scale, (y-radius)*scale, (x+radius)*scale, (y+radius)*scale), fill='#a18ae6')
    frames.append(image.resize((1280, 360), Image.Resampling.LANCZOS))
frames[12].save(output / 'banner.png', optimize=True)
save_gif(frames, [70] * len(frames), output / 'banner.gif')

source = Image.open(output / 'themes.png').convert('RGB')
panels = [source.crop((192,104,568,662)), source.crop((712,104,1088,662))]
theme_frames = []
durations = []
states = [(0.0, 1800)]
states += [(i / 12, 20) for i in range(1,13)]
states += [(1.0, 1800)]
states += [(1 - i / 12, 20) for i in range(1,13)]
for amount, duration in states:
    amount = amount * amount * (3 - 2 * amount)
    background = tuple(round(a + (b-a)*amount) for a,b in zip((238,237,244),(29,26,37)))
    foreground = tuple(round(a + (b-a)*amount) for a,b in zip((84,76,101),(203,193,221)))
    image = Image.new('RGB', (600 * scale, 700 * scale), background)
    draw = ImageDraw.Draw(image)
    title = 'Light  ↔  Dark'
    width = draw.textlength(title, font=font(17)) / scale
    text(draw, (600-width)/2, 28, title, 17, foreground)
    panel = Image.blend(panels[0], panels[1], amount).resize((376*scale,558*scale),Image.Resampling.LANCZOS)
    mask = Image.new('L',panel.size)
    ImageDraw.Draw(mask).rounded_rectangle((0,0,panel.width-1,panel.height-1),10*scale,fill=255)
    image.paste(panel,(112*scale,76*scale),mask)
    caption = 'Switch themes. Keep receiving.'
    width = draw.textlength(caption,font=font(14))/scale
    text(draw,(600-width)/2,661,caption,14,foreground)
    theme_frames.append(image.resize((600,700),Image.Resampling.LANCZOS))
    durations.append(duration)
save_gif(theme_frames,durations,output/'theme-transition.gif')
print('Rendered banner.png, banner.gif and theme-transition.gif')
