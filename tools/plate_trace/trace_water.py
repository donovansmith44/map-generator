# Trace the plate's water: blue mask -> components -> contours ->
# lat/lon rings. Dilated a stroke-width so the water laps slightly
# over the region borders; water paints on top, so the visible seam
# is exactly flush.
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
blue = ndi.binary_closing(blue, structure=np.ones((13, 13)))   # bridge label gaps
blue = cv2.dilate(blue.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (13, 13))) > 0
blue = ndi.binary_fill_holes(blue)

lab, n = ndi.label(blue)
sizes = ndi.sum(blue, lab, index=range(1, n + 1))
order = np.argsort(sizes)[::-1]
coef = np.load(f'{TMP}/affine.npy')

def contour_lonlat(mask, eps=3.0):
    cnts, _ = cv2.findContours(mask.astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2).astype(float)  # (x, y)
    ap = cv2.approxPolyDP(cnt.astype(np.float32).reshape(-1, 1, 2), eps, True).reshape(-1, 2)
    P = np.c_[ap, np.ones(len(ap))]
    return P @ coef   # (lon, lat)

parts = []
for k in order:
    sz = sizes[k]
    if sz < 2500:
        break
    comp = lab == (k + 1)
    ll = contour_lonlat(comp)
    parts.append((int(sz), ll))
    ys, xs = np.nonzero(comp)
    print(f"component {k+1}: {int(sz)} px, {len(ll)} pts, "
          f"lat {np.min(ll[:,1]):.2f}..{np.max(ll[:,1]):.2f} lon {np.min(ll[:,0]):.2f}..{np.max(ll[:,0]):.2f}")

np.savez(f'{TMP}/plate_water.npz', **{f'part{i}': ll for i, (_, ll) in enumerate(parts)})
print("saved", len(parts), "parts")
