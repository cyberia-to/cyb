#!/usr/bin/env python3
"""Build shell/assets/icon.icns from shell/assets/icon-source.png.

The source is the cyb eye on transparency. The app icon composites it over
pure black inside a Big Sur squircle: the robot is white, and any lighter
tile washes it out. Run from the repo root:

    python3 scripts/mkicon.py

Needs `sips` and `iconutil` (macOS). No third-party packages — PNG encode
and decode are inline so the icon can be rebuilt on a bare checkout.
"""
import os, shutil, subprocess, sys
import zlib, struct

def load_rgba(p):
    d=open(p,'rb').read(); i=8; idat=b''; w=h=None
    while i<len(d):
        ln=struct.unpack('>I',d[i:i+4])[0]; t=d[i+4:i+8]; data=d[i+8:i+8+ln]
        if t==b'IHDR': w,h,bd,ct,_,_,_=struct.unpack('>IIBBBBB',data)
        elif t==b'IDAT': idat+=data
        i+=8+ln+4
    raw=zlib.decompress(idat); bpp=4; stride=w*bpp
    out=bytearray(); prev=bytearray(stride); pos=0
    for _ in range(h):
        f=raw[pos]; pos+=1
        line=bytearray(raw[pos:pos+stride]); pos+=stride
        if f==1:
            for x in range(bpp,stride): line[x]=(line[x]+line[x-bpp])&255
        elif f==2:
            for x in range(stride): line[x]=(line[x]+prev[x])&255
        elif f==3:
            for x in range(stride):
                a=line[x-bpp] if x>=bpp else 0
                line[x]=(line[x]+((a+prev[x])>>1))&255
        elif f==4:
            for x in range(stride):
                a=line[x-bpp] if x>=bpp else 0
                b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
                pp=a+b-c; pa=abs(pp-a); pb=abs(pp-b); pc=abs(pp-c)
                pr=a if (pa<=pb and pa<=pc) else (b if pb<=pc else c)
                line[x]=(line[x]+pr)&255
        out+=line; prev=line
    return w,h,bytearray(out)

def write_png(path,w,h,px):
    rows=b''.join(b'\x00'+bytes(px[y*w*4:(y+1)*w*4]) for y in range(h))
    def ck(t,d):
        c=t+d; return struct.pack('>I',len(d))+c+struct.pack('>I',zlib.crc32(c)&0xffffffff)
    open(path,'wb').write(b'\x89PNG\r\n\x1a\n'
        +ck(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))
        +ck(b'IDAT',zlib.compress(bytes(rows),9))+ck(b'IEND',b''))

N = 1024
# macOS Big Sur icon grid: the squircle covers 824 of 1024, centred.
A = 824/2.0
CX = CY = N/2.0
NEXP = 5.0            # superellipse exponent — Apple's squircle
SS = 3                # coverage samples per axis for the squircle edge
EYE_SCALE = 0.66      # eye width relative to the full source canvas

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC  = os.path.join(ROOT, 'shell/assets/icon-source.png')
ICNS = os.path.join(ROOT, 'shell/assets/icon.icns')

sw, sh, src = load_rgba(SRC)

def squircle_alpha(x,y):
    hits=0
    for sy in range(SS):
        for sx in range(SS):
            u=(x+(sx+0.5)/SS-CX)/A
            v=(y+(sy+0.5)/SS-CY)/A
            if abs(u)**NEXP+abs(v)**NEXP<=1.0: hits+=1
    return hits*255//(SS*SS)

def sample(fx,fy):
    """Bilinear sample of the source, premultiplied-safe on alpha."""
    if fx<0 or fy<0 or fx>sw-1 or fy>sh-1: return (0,0,0,0)
    x0=int(fx); y0=int(fy); x1=min(x0+1,sw-1); y1=min(y0+1,sh-1)
    tx=fx-x0; ty=fy-y0
    out=[]
    for c in range(4):
        p00=src[(y0*sw+x0)*4+c]; p10=src[(y0*sw+x1)*4+c]
        p01=src[(y1*sw+x0)*4+c]; p11=src[(y1*sw+x1)*4+c]
        top=p00+(p10-p00)*tx; bot=p01+(p11-p01)*tx
        out.append(int(round(top+(bot-top)*ty)))
    return tuple(out)

dst=bytearray(N*N*4)
inv = 1.0/EYE_SCALE
for y in range(N):
    for x in range(N):
        sa = squircle_alpha(x,y)
        o=(y*N+x)*4
        if sa==0:
            continue
        # Eye pixel, sampled from the source scaled about the centre.
        fx = (x-CX)*inv + sw/2.0
        fy = (y-CY)*inv + sh/2.0
        r,g,b,a = sample(fx,fy)
        # Composite eye over pure black, then mask by the squircle.
        af = a/255.0
        dst[o+0]=int(round(r*af))
        dst[o+1]=int(round(g*af))
        dst[o+2]=int(round(b*af))
        dst[o+3]=sa
build = os.path.join(ROOT, 'target', 'icon-build')
shutil.rmtree(build, ignore_errors=True)
iconset = os.path.join(build, 'cyb.iconset')
os.makedirs(iconset)
master = os.path.join(build, 'icon_1024.png')
write_png(master, N, N, dst)

for side in (16, 32, 128, 256, 512):
    for suffix, px in ((f'{side}x{side}', side), (f'{side}x{side}@2x', side * 2)):
        subprocess.run(['sips', '-Z', str(px), master,
                        '--out', os.path.join(iconset, f'icon_{suffix}.png')],
                       check=True, stdout=subprocess.DEVNULL)

subprocess.run(['iconutil', '-c', 'icns', iconset, '-o', ICNS], check=True)
print(f'wrote {ICNS}', file=sys.stderr)

# ── Android adaptive icon ────────────────────────────────────────────────────
# A launcher icon is 108dp with the outer 25% reserved for the launcher's own
# mask and parallax: only the centre 72dp is guaranteed visible. Shipping a
# pre-masked square (what cyb did) makes every launcher shrink it again and
# leaves its own background showing at the corners — the small icon on a
# not-quite-black tile. An adaptive icon hands over the two layers instead:
# solid black background, the eye alone on the foreground inside the safe zone.
RES = os.path.join(ROOT, 'shell/gen/android/app/src/main/res')
SAFE = 0.62          # eye width as a fraction of the 108dp canvas
DENSITIES = {'mdpi': 108, 'hdpi': 162, 'xhdpi': 216, 'xxhdpi': 324, 'xxxhdpi': 432}

fg = bytearray(N * N * 4)
inv_fg = 1.0 / SAFE
for y in range(N):
    for x in range(N):
        r, g, b, a = sample((x - CX) * inv_fg + sw / 2.0, (y - CY) * inv_fg + sh / 2.0)
        o = (y * N + x) * 4
        af = a / 255.0
        dst_px = (int(round(r * af)), int(round(g * af)), int(round(b * af)), a)
        fg[o:o + 4] = bytes(dst_px)

fg_master = os.path.join(build, 'ic_launcher_foreground.png')
write_png(fg_master, N, N, fg)

for density, px in DENSITIES.items():
    out_dir = os.path.join(RES, f'mipmap-{density}')
    os.makedirs(out_dir, exist_ok=True)
    subprocess.run(['sips', '-Z', str(px), fg_master,
                    '--out', os.path.join(out_dir, 'ic_launcher_foreground.png')],
                   check=True, stdout=subprocess.DEVNULL)
    # Legacy square for API < 26: the pre-masked squircle still applies there.
    subprocess.run(['sips', '-Z', str(px), master,
                    '--out', os.path.join(out_dir, 'ic_launcher.png')],
                   check=True, stdout=subprocess.DEVNULL)

anydpi = os.path.join(RES, 'mipmap-anydpi-v26')
os.makedirs(anydpi, exist_ok=True)
for name in ('ic_launcher', 'ic_launcher_round'):
    with open(os.path.join(anydpi, f'{name}.xml'), 'w') as f:
        f.write('<?xml version="1.0" encoding="utf-8"?>\n'
                '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
                '    <background android:drawable="@color/icon_background" />\n'
                '    <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n'
                '    <monochrome android:drawable="@mipmap/ic_launcher_foreground" />\n'
                '</adaptive-icon>\n')

values = os.path.join(RES, 'values')
os.makedirs(values, exist_ok=True)
with open(os.path.join(values, 'colors.xml'), 'w') as f:
    f.write('<?xml version="1.0" encoding="utf-8"?>\n'
            '<resources>\n'
            '    <!-- One black, the same one every cyb surface uses. -->\n'
            '    <color name="icon_background">#FF000000</color>\n'
            '</resources>\n')

shutil.rmtree(build, ignore_errors=True)
print(f'wrote adaptive icon layers under {RES}', file=sys.stderr)
