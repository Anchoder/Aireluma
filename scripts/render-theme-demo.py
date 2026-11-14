from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import os

root = Path(__file__).resolve().parents[1]
output = root / 'docs/images'
scale = 2
fonts = Path(os.environ.get('WINDIR', 'C:/Windows')) / 'Fonts'
logo = Image.open(root / 'assets/aireluma.png').convert('RGBA')
light = {'bg':'#f7f7fa','card':'#ffffff','border':'#dddde6','input':'#f7f7fa','input_border':'#d4d4df','step':'#eaecf9','text':'#242933','secondary':'#5c606a','muted':'#666b75','accent':'#6753dd','toggle':'#e6e8ef'}
dark = {'bg':'#101014','card':'#18181e','border':'#2b2b31','input':'#101016','input_border':'#34343b','step':'#20202b','text':'#f3f3f6','secondary':'#c2c1c9','muted':'#9f9da7','accent':'#7764ed','toggle':'#282833'}

def font(size, semibold=False):
    path = fonts / ('seguisb.ttf' if semibold else 'segoeui.ttf')
    if not path.exists():
        path = Path('/usr/share/fonts/truetype/dejavu') / ('DejaVuSans-Bold.ttf' if semibold else 'DejaVuSans.ttf')
    return ImageFont.truetype(str(path), size * scale)

def rgb(value):
    return tuple(int(value[i:i+2],16) for i in (1,3,5))

def mix(a,b,t):
    return tuple(round(x+(y-x)*t) for x,y in zip(rgb(a) if isinstance(a,str) else a,rgb(b) if isinstance(b,str) else b))

def eased(t):
    return t*t*(3-2*t)

def rect(draw, bounds, radius, fill, outline=None):
    draw.rounded_rectangle(tuple(round(v*scale) for v in bounds),radius*scale,fill=fill,outline=outline,width=scale)

def text(draw,x,y,value,size,color,semibold=False):
    draw.text((round(x*scale),round(y*scale)),value,font=font(size,semibold),fill=color,anchor='lt')

def centered(draw,bounds,value,size,color,semibold=False,offset=0):
    face=font(size,semibold)
    box=draw.textbbox((0,0),value,font=face)
    left,top,right,bottom=[v*scale for v in bounds]
    width,height=box[2]-box[0],box[3]-box[1]
    assert width <= right-left-8*scale, value
    assert height <= bottom-top-4*scale, value
    x=(left+right-width)/2-box[0]
    y=(top+bottom-height)/2-box[1]+offset*scale
    draw.text((round(x),round(y)),value,font=face,fill=color)

def window(amount,target_dark,hover=False,pressed=False):
    p={name:mix(light[name],dark[name],amount) for name in light}
    image=Image.new('RGBA',(376*scale,556*scale),p['bg'])
    draw=ImageDraw.Draw(image)
    image.alpha_composite(logo.resize((14*scale,14*scale),Image.Resampling.LANCZOS),(9*scale,7*scale))
    text(draw,29,8,'Aireluma',11,p['text'])
    draw.line((294*scale,14*scale,304*scale,14*scale),fill=p['secondary'],width=scale)
    draw.rectangle((326*scale,9*scale,334*scale,17*scale),outline=p['muted'],width=scale)
    for a,b in [((356,9),(364,17)),((364,9),(356,17))]:
        draw.line((a[0]*scale,a[1]*scale,b[0]*scale,b[1]*scale),fill=p['secondary'],width=scale)
    def t(x,y,value,size,color,semibold=False):text(draw,x,y+28,value,size,color,semibold)
    def box(bounds,radius,fill,outline=None):rect(draw,(bounds[0],bounds[1]+28,bounds[2],bounds[3]+28),radius,fill,outline)
    image.alpha_composite(logo.resize((48*scale,48*scale),Image.Resampling.LANCZOS),(24*scale,52*scale))
    t(88,24,'Aireluma',27,p['text'],True)
    t(89,60,'Повтор экрана с iOS',13,p['muted'])
    toggle=p['toggle']
    if hover:toggle=mix(toggle,p['accent'],0.12)
    if pressed:toggle=mix(toggle,p['accent'],0.25)
    box((254,30+int(pressed),352,58-int(pressed)),6,toggle)
    centered(draw,(254,58,352,86),'Светлая тема' if target_dark else 'Тёмная тема',13,p['text'],offset=int(pressed))
    box((24,100,352,292),10,p['card'],p['border'])
    t(40,120,'Имя устройства в AirPlay',13,p['secondary'])
    box((40,140,336,180),6,p['input'],p['input_border'])
    t(52,154,'Aireluma',14,p['text'])
    draw.ellipse((44*scale,230*scale,51*scale,237*scale),fill=p['muted'])
    t(60,198,'Приём выключен',14,p['secondary'])
    box((40,232,336,272),6,p['accent'])
    centered(draw,(40,260,336,300),'Включить AirPlay',14,'#ffffff',True)
    t(24,320,'Как подключиться',16,p['text'],True)
    for number,y,title,note in [('1',358,'Откройте «Повтор экрана»','В Пункте управления iPhone'),('2',416,'Выберите устройство','Имя указано в поле выше')]:
        box((24,y,52,y+28),5,p['step'])
        centered(draw,(24,y+28,52,y+56),number,13,p['accent'])
        t(68,y-4,title,14,p['secondary'])
        t(68,y+21,note,13,p['muted'])
    draw.line((24*scale,502*scale,352*scale,502*scale),fill=p['border'],width=scale)
    labels=['Экран и звук','Локально','до 60 FPS']
    widths=[round(draw.textlength(value,font=font(13))/scale)+1 for value in labels]
    spare=328-sum(widths)-72
    gaps=[spare//2,spare-spare//2]
    x=24
    for i,(name,value) in enumerate(zip(['screen','wifi','clock'],labels)):
        symbol=Image.open(root/f'assets/footer-{name}.ico').convert('RGBA').resize((16*scale,16*scale),Image.Resampling.LANCZOS)
        solid=Image.new('RGBA',symbol.size,p['secondary'])
        solid.putalpha(symbol.getchannel('A'))
        image.alpha_composite(solid,(x*scale,520*scale))
        text(draw,x+24,522,value,13,p['secondary'])
        x+=24+widths[i]+(gaps[i] if i<2 else 0)
    mask=Image.new('L',image.size)
    ImageDraw.Draw(mask).rounded_rectangle((0,0,image.width-1,image.height-1),10*scale,fill=255)
    image.putalpha(mask)
    return image

def cursor(draw,x,y,pressed=False):
    points=[(0,0),(0,21),(5.7,15.8),(10.4,25),(14.3,23),(9.9,14),(18.4,14)]
    factor=.94 if pressed else 1
    polygon=[(round((x+u*factor)*scale),round((y+v*factor)*scale)) for u,v in points]
    draw.polygon(polygon,fill='#ffffff')
    draw.line(polygon+[polygon[0]],fill='#17141d',width=round(1.5*scale),joint='curve')

def frame(amount,target_dark,point=None,hover=False,pressed=False,ripple=None):
    image=Image.new('RGBA',(440*scale,608*scale),mix('#eeedf4','#1d1a25',amount))
    image.alpha_composite(window(amount,target_dark,hover,pressed),(32*scale,24*scale))
    if ripple is not None:
        layer=Image.new('RGBA',image.size)
        draw=ImageDraw.Draw(layer)
        radius=8+19*ripple
        alpha=round(190*(1-ripple))
        draw.ellipse(((370-radius)*scale,(96-radius)*scale,(370+radius)*scale,(96+radius)*scale),outline=(132,103,225,alpha),width=2*scale)
        image.alpha_composite(layer)
    if point is not None:cursor(ImageDraw.Draw(image),*point,pressed)
    return image.convert('RGB')

frames=[]
durations=[]
def add(amount,target_dark,duration,point=None,hover=False,pressed=False,ripple=None):
    frames.append(frame(amount,target_dark,point,hover,pressed,ripple))
    durations.append(duration)

add(0,False,1000)
checkpoints={'light':0}
for origin,target in [(0,1),(1,0)]:
    for i in range(1,13):
        t=eased(i/12)
        add(origin,bool(origin),40,(410+(370-410)*t,205+(96-205)*t),hover=i>=11)
    add(origin,bool(origin),240,(370,96),True)
    for i in range(3):add(origin,bool(origin),60,(370,96),True,True,i*.16)
    checkpoints['press-dark' if origin else 'press-light']=len(frames)-2
    add(origin,bool(target),60,(370,96),True,ripple=.5)
    for i in range(1,11):
        t=eased(i/10)
        add(origin+(target-origin)*t,bool(target),30,(370,96),True,ripple=.5+.5*i/10)
    for i in range(1,9):
        t=eased(i/8)
        add(target,bool(target),35,(370+(410-370)*t,96+(205-96)*t))
    add(target,bool(target),1400)
    if target:checkpoints['dark']=len(frames)-1

samples=Image.new('RGB',(frames[0].width*6,frames[0].height))
indices=[0,checkpoints['press-light'],checkpoints['dark'],checkpoints['press-dark'],20,60]
for i,index in enumerate(indices):samples.paste(frames[index],(i*frames[0].width,0))
palette=samples.quantize(colors=256)
encoded=[image.quantize(palette=palette,dither=Image.Dither.NONE) for image in frames]
encoded[0].save(output/'theme-transition.gif',save_all=True,append_images=encoded[1:],duration=durations,loop=0,optimize=True,disposal=2)

still=Image.new('RGBA',(800*scale,588*scale),'#eeedf4')
for x,amount in [(16,0),(408,1)]:still.alpha_composite(window(amount,bool(amount)),(x*scale,16*scale))
still.convert('RGB').save(output/'themes.png',optimize=True)

checkdir=root/'test-ui'
checkdir.mkdir(exist_ok=True)
sheet=Image.new('RGB',(440*2,608*2))
for i,name in enumerate(['light','press-light','dark','press-dark']):
    sheet.paste(frames[checkpoints[name]].resize((440,608),Image.Resampling.LANCZOS),((i%2)*440,(i//2)*608))
sheet.save(checkdir/'theme-click-check.png')
print(f'Rendered {len(frames)} frames, {sum(durations)/1000:.2f}s, {frames[0].width}x{frames[0].height}')
