# Plate water v2, in PIXEL space (the chart derives the sphere):
#  - wide water (sea, lakes) -> area contours
#  - thin water (rivers)     -> skeleton centerlines -> smooth ribbons
import numpy as np
from PIL import Image
import scipy.ndimage as ndi
import cv2

IMG = r'C:/Users/donov/Documents/the-best-maps-ever/10 Canaan Before the Conquest of Joshua.png'
TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
im = np.asarray(Image.open(IMG).convert('RGB')).astype(np.int16)
H, W, _ = im.shape
r, g, b = im[..., 0], im[..., 1], im[..., 2]

blue = (b > r + 25) & (b > g + 18) & (b > 130)
blue = ndi.binary_closing(blue, structure=np.ones((5, 5)))
blue8 = blue.astype(np.uint8)

# wide water: core deeper than 7 px, grown back within blue (bounded
# reconstruction so it can't crawl up the rivers)
dist = cv2.distanceTransform(blue8, cv2.DIST_L2, 5)
wide = (dist > 7).astype(np.uint8)
k3 = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
for _ in range(10):
    wide = cv2.dilate(wide, k3) & blue8
wide = wide > 0

coef = np.load(f'{TMP}/affine.npy')
def to_lonlat(pts_xy):
    P = np.c_[pts_xy, np.ones(len(pts_xy))]
    return P @ coef

def name_of(cx, cy):
    lon, lat = to_lonlat(np.array([[cx, cy]]))[0]
    if lat > 32.4: return "chinnereth"
    if lat > 30.8: return "salt-sea"
    return f"water-{cx:.0f}-{cy:.0f}"

# area rings (pixel space), dilated half a border stroke for the lap
lab, n = ndi.label(wide)
sizes = ndi.sum(wide, lab, index=range(1, n + 1))
areas = []
k9 = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9))
for k in np.argsort(sizes)[::-1]:
    if sizes[k] < 2500:
        break
    comp = cv2.dilate((lab == k + 1).astype(np.uint8), k9)
    cnts, _ = cv2.findContours(comp, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2).astype(np.float32)
    ap = cv2.approxPolyDP(cnt.reshape(-1, 1, 2), 2.0, True).reshape(-1, 2).astype(float)
    ys, xs = np.nonzero(lab == k + 1)
    nm = "great-sea" if len(areas) == 0 and sizes[k] > 3e6 else name_of(xs.mean(), ys.mean())
    areas.append((nm, ap))
    print(f"area {nm}: {int(sizes[k])} px -> {len(ap)} pts")

# thin water -> skeleton (Zhang-Suen, vectorized on the bbox).
# Rivers only: saturated blue (region borders and frame strokes are
# grayer), healed hard across label breaks, frame margin excluded.
tight = ndi.binary_closing(blue, structure=np.ones((17, 17)))
thin = tight & ~wide
thin[:40, :] = False; thin[-40:, :] = False; thin[:, :40] = False; thin[:, -40:] = False
thin = ndi.binary_opening(thin, structure=np.ones((2, 2)))
lab2, n2 = ndi.label(thin)
sz2 = ndi.sum(thin, lab2, index=range(1, n2 + 1))
keep = np.isin(lab2, 1 + np.nonzero(sz2 >= 120)[0])
ys, xs = np.nonzero(keep)
y0, y1, x0, x1 = ys.min()-2, ys.max()+3, xs.min()-2, xs.max()+3
M = keep[y0:y1, x0:x1].astype(np.uint8)

def zhang_suen(img):
    img = img.copy()
    def nbrs(a):
        p2 = np.roll(a, -1, 0); p3 = np.roll(np.roll(a, -1, 0), 1, 1)
        p4 = np.roll(a, 1, 1);  p5 = np.roll(np.roll(a, 1, 0), 1, 1)
        p6 = np.roll(a, 1, 0);  p7 = np.roll(np.roll(a, 1, 0), -1, 1)
        p8 = np.roll(a, -1, 1); p9 = np.roll(np.roll(a, -1, 0), -1, 1)
        return p2, p3, p4, p5, p6, p7, p8, p9
    while True:
        changed = False
        for phase in (0, 1):
            p2, p3, p4, p5, p6, p7, p8, p9 = nbrs(img)
            B = p2+p3+p4+p5+p6+p7+p8+p9
            seq = [p2, p3, p4, p5, p6, p7, p8, p9, p2]
            A = sum(((seq[i] == 0) & (seq[i+1] == 1)).astype(np.uint8) for i in range(8))
            if phase == 0:
                cond = (img == 1) & (B >= 2) & (B <= 6) & (A == 1) & (p2*p4*p6 == 0) & (p4*p6*p8 == 0)
            else:
                cond = (img == 1) & (B >= 2) & (B <= 6) & (A == 1) & (p2*p4*p8 == 0) & (p2*p6*p8 == 0)
            if cond.any():
                img[cond] = 0
                changed = True
        if not changed:
            return img

skel = zhang_suen(M)
print("skeleton px:", int(skel.sum()))

# skeleton -> chains between endpoints/junctions
nb = ndi.convolve(skel.astype(np.int16), np.ones((3, 3), np.int16), mode='constant') - skel
deg = np.where(skel > 0, nb, 0)
special = (skel > 0) & (deg != 2)
sy, sx = np.nonzero(skel)
skset = set(zip(sy.tolist(), sx.tolist()))
spset = set(zip(*[a.tolist() for a in np.nonzero(special)]))
visited = set()
chains = []
D8 = [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)]
def walk(start, first):
    ch = [start, first]
    visited.add((start, first)); visited.add((first, start))
    cur, prev = first, start
    while cur not in spset:
        nxt = None
        for dy, dx in D8:
            q = (cur[0]+dy, cur[1]+dx)
            if q in skset and q != prev and (cur, q) not in visited:
                nxt = q; break
        if nxt is None: break
        visited.add((cur, nxt)); visited.add((nxt, cur))
        ch.append(nxt); prev, cur = cur, nxt
    return ch
for p in sorted(spset):
    for dy, dx in D8:
        q = (p[0]+dy, p[1]+dx)
        if q in skset and (p, q) not in visited:
            chains.append(walk(p, q))
print("chains:", len(chains))

# side classification: a river runs inside ONE fill; a border stroke
# separates two. Chroma (r-g, g-b) cancels the relief shading.
REF = {
  'green': (-12, 19), 'gray': (0, -13), 'orange': (82, 55), 'tan': (36, 36),
  'red': (68, 2), 'yellow': (48, 82), 'cream': (10, 24), 'blue': (-28, -49),
}
cr = (im[..., 0] - im[..., 1]).astype(np.float64)
cg = (im[..., 1] - im[..., 2]).astype(np.float64)
def classify(x, y):
    xi, yi = int(round(x)), int(round(y))
    if not (0 <= yi < H and 0 <= xi < W):
        return 'off'
    a, bb = cr[yi, xi], cg[yi, xi]
    return min(REF, key=lambda k: (REF[k][0]-a)**2 + (REF[k][1]-bb)**2)
def side_of(ap, sign):
    votes = {}
    d = np.diff(ap, axis=0)
    L = np.hypot(d[:, 0], d[:, 1]); L[L == 0] = 1
    nrm = np.c_[-d[:, 1]/L, d[:, 0]/L]
    idx = np.linspace(0, len(d) - 1, min(9, len(d))).astype(int)
    for i in idx:
        mid = (ap[i] + ap[i+1]) / 2
        for off in (8.0, 13.0):
            c = classify(mid[0] + sign*nrm[i][0]*off, mid[1] + sign*nrm[i][1]*off)
            if c != 'blue':
                votes[c] = votes.get(c, 0) + 1
                break
    return max(votes, key=votes.get) if votes else 'off'

# THE JORDAN, by construction: a fixed-width band hugging the green
# region's edge wherever the gray region lies across the valley —
# continuous past the labels, kept off the lakes (they are areas).
green_m = (g >= r - 6) & (g - b > 14) & (b < 190) & (g > 90) & (r < 210)
green_m = ndi.binary_closing(green_m, structure=np.ones((25, 25)))
glab, gn = ndi.label(green_m)
gsz = ndi.sum(green_m, glab, index=range(1, gn + 1))
green_m = ndi.binary_fill_holes(glab == 1 + int(np.argmax(gsz)))
gray_m = (abs(r - g) < 14) & (b > g + 4) & (b - g < 26) & (r > 110) & (r < 185)
gray_m = ndi.binary_closing(gray_m, structure=np.ones((25, 25)))
ylab, yn = ndi.label(gray_m)
ysz = ndi.sum(gray_m, ylab, index=range(1, yn + 1))
gray_m = ndi.binary_fill_holes(ylab == 1 + int(np.argmax(ysz)))
def disk(rr):
    return cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2*rr+1, 2*rr+1))
band = (cv2.dilate(green_m.astype(np.uint8), disk(18)) > 0)      & (cv2.dilate(gray_m.astype(np.uint8), disk(90)) > 0)      & ~green_m & ~gray_m      & ~(cv2.dilate(wide.astype(np.uint8), disk(10)) > 0)
band = ndi.binary_opening(band, structure=np.ones((3, 3)))
band = ndi.binary_closing(band, structure=np.ones((11, 11)))
jordan_ribbons = []
blab, bn = ndi.label(band)
bsz = ndi.sum(band, blab, index=range(1, bn + 1))
for k in np.argsort(bsz)[::-1]:
    if bsz[k] < 250:
        break
    comp = (blab == k + 1).astype(np.uint8)
    cnts, _ = cv2.findContours(comp, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2).astype(np.float32)
    ap = cv2.approxPolyDP(cnt.reshape(-1, 1, 2), 2.0, True).reshape(-1, 2).astype(float)
    if len(ap) >= 4:
        jordan_ribbons.append(ap)
print("jordan ribbons:", len(jordan_ribbons))

rivers = []
dropped = 0
for ch in chains:
    if len(ch) < 80:
        continue
    pts = np.array([(x + x0, y + y0) for y, x in ch], float)   # (x, y)
    ap = cv2.approxPolyDP(pts.astype(np.float32).reshape(-1, 1, 2), 2.5, False).reshape(-1, 2).astype(float)
    if len(ap) < 2:
        continue
    ls, rs = side_of(ap, +1), side_of(ap, -1)
    pair = {ls, rs}
    # the ink itself is the last word: real rivers are saturated blue,
    # border strokes are grayer — a desert river (cream sides) survives
    # and border ink dies regardless of neighbors.
    idxs = np.linspace(0, len(ap) - 1, min(15, len(ap))).astype(int)
    ink = []
    for i2 in idxs:
        xi, yi = int(round(ap[i2][0])), int(round(ap[i2][1]))
        if 0 <= yi < H and 0 <= xi < W:
            ink.append(int(im[yi, xi, 2]) - int(im[yi, xi, 0]))
    saturated = len(ink) > 0 and float(np.median(ink)) > 55
    if 'off' in pair:
        dropped += 1
        continue
    if ('orange' in pair or 'cream' in pair) and not saturated:
        dropped += 1
        continue
    if pair == {'green', 'gray'}:
        continue  # the corridor already owns the Jordan
    # ribbon: offset +-4 px with clamped miter joins
    d = np.diff(ap, axis=0)
    L = np.hypot(d[:, 0], d[:, 1]); L[L == 0] = 1
    nseg = np.c_[-d[:, 1]/L, d[:, 0]/L]
    nv = np.vstack([nseg[0], nseg[:-1] + nseg[1:], nseg[-1]])
    nn = np.hypot(nv[:, 0], nv[:, 1]); nn[nn < 0.3] = 0.3
    nv = nv / nn[:, None]
    wpx = 4.0
    left = ap + nv*wpx; right = ap - nv*wpx
    ring = np.vstack([left, right[::-1]])
    rivers.append(ring)
print("rivers kept:", len(rivers), "jordan ribbons:", len(jordan_ribbons), "dropped border ink:", dropped)

np.savez(f'{TMP}/plate_water2.npz',
         **{f'area_{nm}_{i}': ap for i, (nm, ap) in enumerate(areas)},
         **{f'jordan_{i}': rg for i, rg in enumerate(jordan_ribbons)},
         **{f'river_{i}': rg for i, rg in enumerate(rivers)})
print("saved")
