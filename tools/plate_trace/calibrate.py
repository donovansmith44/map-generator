# Georeference the plate: find city dots near guessed positions, fit
# pixel -> (lon, lat) affine by least squares, report residuals in km.
import numpy as np
from PIL import Image

IMG = r'C:/Users/donov/Documents/the-best-maps-ever/10 Canaan Before the Conquest of Joshua.png'
im = np.asarray(Image.open(IMG).convert('RGB')).astype(np.int16)
H, W, _ = im.shape

# name: (guess_x, guess_y at 1500x2000 display scale), (lat, lon)
CTRL = {
  "Sidon":      ((834, 229),  (33.562, 35.369)),
  "Tyre":       ((759, 381),  (33.270, 35.194)),
  "Hazor":      ((918, 519),  (33.017, 35.568)),
  "Megiddo":    ((750, 741),  (32.585, 35.183)),
  "Shechem":    ((800, 931),  (32.213, 35.282)),
  "Bethel":     ((779, 1083), (31.930, 35.221)),
  "Jericho":    ((884, 1108), (31.871, 35.444)),
  "Jerusalem":  ((765, 1160), (31.778, 35.229)),
  "Ashkelon":   ((479, 1216), (31.669, 34.546)),
  "Gaza":       ((435, 1302), (31.503, 34.446)),
  "Hebron":     ((708, 1283), (31.532, 35.096)),
  "Beersheba":  ((578, 1430), (31.245, 34.792)),
  "Heshbon":    ((1046, 1132),(31.802, 35.809)),
  "Rabbah":     ((1080, 1089),(31.955, 35.934)),
  "Edrei":      ((1160, 716), (32.618, 36.102)),
}

def find_dot(gx, gy, win=60):
    # search window around guess (original scale) for a small dark
    # roundish blob; return its centroid or None.
    x0, x1 = max(0, gx-win), min(W, gx+win)
    y0, y1 = max(0, gy-win), min(H, gy+win)
    tile = im[y0:y1, x0:x1]
    dark = (tile.sum(axis=2) < 250)  # near-black
    if not dark.any():
        return None
    # connected components (4-neigh, tiny BFS)
    lab = np.zeros(dark.shape, np.int32)
    comps = []
    nxt = 0
    from collections import deque
    for yy, xx in zip(*np.nonzero(dark)):
        if lab[yy, xx]:
            continue
        nxt += 1
        q = deque([(yy, xx)]); lab[yy, xx] = nxt; pix = []
        while q:
            cy, cx = q.popleft(); pix.append((cy, cx))
            for dy, dx in ((1,0),(-1,0),(0,1),(0,-1)):
                ny, nx2 = cy+dy, cx+dx
                if 0 <= ny < dark.shape[0] and 0 <= nx2 < dark.shape[1] and dark[ny, nx2] and not lab[ny, nx2]:
                    lab[ny, nx2] = nxt; q.append((ny, nx2))
        pix = np.array(pix)
        h = int(np.ptp(pix[:,0]))+1; w = int(np.ptp(pix[:,1]))+1
        area = len(pix)
        # a dot: 20..200 px, roundish, filled
        if 60 <= area <= 700 and max(h,w) <= 45 and abs(h-w) <= max(h,w)*0.4 and area >= 0.55*h*w:
            cy, cx = pix[:,0].mean(), pix[:,1].mean()
            d = ((cy-(gy-y0))**2 + (cx-(gx-x0))**2)**0.5
            comps.append((d, x0+cx, y0+cy, area))
    if not comps:
        return None
    comps.sort()
    return comps[0][1], comps[0][2]

pix, lonlat, names = [], [], []
for name, ((dx, dy), (lat, lon)) in CTRL.items():
    got = find_dot(dx*3, dy*3)
    if got is None:
        print(f"  MISS {name}")
        continue
    pix.append(got); lonlat.append((lon, lat)); names.append(name)

P = np.array(pix); Q = np.array(lonlat)
Amat = np.c_[P, np.ones(len(P))]
coef, *_ = np.linalg.lstsq(Amat, Q, rcond=None)   # (3,2): [x y 1] -> [lon lat]
pred = Amat @ coef
res = Q - pred
km = np.c_[res[:,0]*111.0*np.cos(np.radians(Q[:,1])), res[:,1]*111.0]
d = np.hypot(km[:,0], km[:,1])
for n, dd, (px_, py_) in sorted(zip(names, d, pix), key=lambda t: -t[1]):
    print(f"  {n:12s} residual {dd:5.2f} km   dot=({px_:.0f},{py_:.0f})")
print("mean %.2f km, max %.2f km, n=%d" % (d.mean(), d.max(), len(d)))
np.save(r'C:/Users/donov/.claude/jobs/c6946bce/tmp/affine.npy', coef)
print("affine saved. lon/lat = [x y 1] @ coef")
