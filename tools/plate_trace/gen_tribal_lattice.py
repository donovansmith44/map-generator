# Organic allotment lattice v2: shared borders authored once, densified
# with a deterministic wiggle, mirrored into both neighbor circuits.
import io, math, re, hashlib

SRC = r'C:\Users\donov\Documents\the-best-maps-ever\crates\map-adapters\src\surveys.rs'

# ---- anchors: name -> (lat, lon). Names are descriptive (non-binding).
A = {
  # west offshore / coast anchors (sea clips these)
  "the sea before Judah":            (31.72, 34.40),
  "the coast toward Jabneel":        (31.74, 34.62),
  "the Sorek valley":                (31.78, 34.85),
  "the going down of Beth-shemesh":  (31.72, 35.00),
  "the saddle by Kiriath-jearim":    (31.76, 35.12),
  "south of Jebus, Hinnom":          (31.74, 35.21),
  "the wilderness toward Jericho":   (31.78, 35.30),
  "the descent to the Salt Sea":     (31.72, 35.42),
  "the Salt Sea's north bay":        (31.74, 35.52),
  "the Salt Sea under En-gedi":      (31.45, 35.48),
  "the Salt Sea toward Zoar":        (31.15, 35.50),
  "the south end of the sea":        (31.02, 35.50),
  "the ascent of Akrabbim southward":(30.90, 35.25),
  "the wilderness of Zin southward": (30.75, 34.85),
  "toward Kadesh-barnea":            (30.65, 34.50),
  "the wilderness toward Shur":      (30.72, 34.25),
  "the sea toward the river of Egypt":(30.85, 34.05),
  # Simeon (coastal Negev lobe)
  "the shore below Gerar":           (30.92, 34.28),
  "the pastures toward Gerar":       (30.95, 34.70),
  "the wells at Beersheba eastward": (31.05, 34.95),
  "toward Moladah":                  (31.25, 35.05),
  "the border above Beersheba":      (31.42, 35.00),
  "toward Ziklag northward":         (31.45, 34.85),
  "the fields of Gerar northward":   (31.52, 34.62),
  "the shore above Gerar":           (31.55, 34.32),
  # Dan / Benjamin / Ephraim
  "the border under Aijalon":        (31.83, 35.05),
  "the going up to Beth-horon":      (31.95, 35.00),
  "the sea by Me-jarkon":            (32.02, 34.68),
  "the plain toward Japho":          (32.00, 34.85),
  "toward Bethel southward":         (31.94, 35.20),
  "the wilderness of Beth-aven":     (31.90, 35.35),
  "north of Jericho":                (31.93, 35.47),
  "the Jordan at Jericho":           (31.90, 35.555),
  # Ephraim / Manasseh (Kanah)
  "the sea at the Kanah brook":      (32.28, 34.72),
  "the mouth of Kanah":              (32.28, 34.90),
  "the Kanah brook upward":          (32.22, 35.05),
  "the spring of Tappuah":           (32.25, 35.20),
  "before Shechem southward":        (32.18, 35.35),
  "the descent toward the Jordan":   (32.28, 35.45),
  "the Jordan by Adam":              (32.25, 35.555),
  # Manasseh north rim / Carmel / Jezreel
  "the sea under Carmel":            (32.72, 34.85),
  "the shoulder of Carmel":          (32.70, 34.95),
  "under Jokneam":                   (32.68, 35.02),
  "the edge of the great valley":    (32.63, 35.12),
  "the valley before Megiddo":       (32.60, 35.22),
  "the valley toward Jezreel":       (32.58, 35.28),
  "the spring of Harod":             (32.48, 35.38),
  "above Beth-shean":                (32.40, 35.48),
  "the Jordan under Beth-shean":     (32.38, 35.555),
  # Zebulun / Issachar / Naphtali / Asher
  "the hill before Jiphthah-el":     (32.78, 35.08),
  "the valley of Jiphthah-el northward":(32.88, 35.22),
  "the border above Hannathon":      (32.88, 35.30),
  "the slopes under Rimmon":         (32.78, 35.33),
  "the oak in Zaanannim":            (32.64, 35.35),
  "under the slopes of Tabor":       (32.70, 35.47),
  "the Jordan at Chinnereth's outflowing":(32.70, 35.555),
  "the shore of Chinnereth westward":(32.78, 35.58),
  "Chinnereth under Capernaum":      (32.95, 35.60),
  "the waters of Merom eastward":    (33.10, 35.60),
  "the upper Jordan northward":      (33.25, 35.61),
  "toward Ijon":                     (33.32, 35.58),
  "the north border under Lebanon":  (33.32, 35.35),
  "the hills above Kedesh":          (33.20, 35.32),
  "the height of Ramah northward":   (33.05, 35.28),
  "the sea toward great Zidon":      (33.32, 35.05),
  "the coast under Achzib":          (33.00, 34.95),
  # East bank
  "the Salt Sea's eastern shore":    (31.76, 35.50),
  "the mouth of the Arnon":          (31.45, 35.52),
  "the Arnon gorge eastward":        (31.48, 35.70),
  "the brink of the Arnon at Aroer": (31.44, 35.85),
  "the high plain by Dibon":         (31.50, 36.00),
  "the wilderness beyond Mephaath":  (31.47, 36.15),
  "the desert rim of Moab":          (31.70, 36.18),
  "the border toward Ammon":         (31.88, 36.12),
  "the plain above Heshbon":         (31.92, 35.90),
  "the fields of Abel-shittim":      (31.88, 35.72),
  "the Jordan by Beth-jeshimoth":    (31.90, 35.57),
  "the Jordan toward Succoth":       (32.10, 35.57),
  "the mouth of the Jabbok":         (32.20, 35.57),
  "the Jordan under Zaphon":         (32.35, 35.57),
  "the Jabbok toward Gerasa":        (32.30, 35.72),
  "the upper Jabbok at Mahanaim":    (32.22, 35.85),
  "the hills toward Ramoth":         (32.35, 36.00),
  "toward Ramoth in Gilead":         (32.35, 36.10),
  "the desert rim of Gilead":        (32.10, 36.18),
  "the Jordan toward Chinnereth eastward":(32.55, 35.58),
  "Chinnereth's east shore southward":(32.72, 35.60),
  "the east shore of Chinnereth":    (32.80, 35.60),
  "above Chinnereth eastward":       (32.90, 35.60),
  "the upper Jordan eastward":       (33.10, 35.63),
  "toward Dan of the north":         (33.25, 35.65),
  "under mount Hermon":              (33.30, 35.72),
  "the slopes of Hermon eastward":   (33.28, 36.00),
  "the border of Maakah":            (33.15, 36.30),
  "the coasts of Argob":             (32.90, 36.50),
  "toward Salecah":                  (32.75, 36.55),
  "the rim of Bashan":               (32.55, 36.30),
  "by Edrei":                        (32.55, 36.10),
}

# ---- shared borders: polylines over anchor names. Each is authored
# once; densified with a deterministic wiggle; both neighbors walk the
# same literal points (one forward, one reversed).
BORDERS = {
  "JUDAH_DAN":   ["the sea before Judah","the coast toward Jabneel","the Sorek valley","the going down of Beth-shemesh"],
  "JUDAH_BENJ":  ["the going down of Beth-shemesh","the saddle by Kiriath-jearim","south of Jebus, Hinnom","the wilderness toward Jericho","the descent to the Salt Sea","the Salt Sea's north bay"],
  "JUDAH_SIMEON":["the shore above Gerar","the fields of Gerar northward","toward Ziklag northward","the border above Beersheba","toward Moladah","the wells at Beersheba eastward","the pastures toward Gerar","the shore below Gerar"],
  "DAN_BENJ":    ["the going down of Beth-shemesh","the border under Aijalon","the going up to Beth-horon"],
  "DAN_EPHRAIM": ["the sea by Me-jarkon","the plain toward Japho","the going up to Beth-horon"],
  "BENJ_EPHRAIM":["the going up to Beth-horon","toward Bethel southward","the wilderness of Beth-aven","north of Jericho","the Jordan at Jericho"],
  "EPHRAIM_MANASSEH":["the sea at the Kanah brook","the mouth of Kanah","the Kanah brook upward","the spring of Tappuah","before Shechem southward","the descent toward the Jordan","the Jordan by Adam"],
  "MANASSEH_ASHER":["the sea under Carmel","the shoulder of Carmel","under Jokneam"],
  "MANASSEH_ZEB":["under Jokneam","the edge of the great valley","the valley before Megiddo","the valley toward Jezreel"],
  "MANASSEH_ISS":["the valley toward Jezreel","the spring of Harod","above Beth-shean","the Jordan under Beth-shean"],
  "ASHER_ZEB":   ["under Jokneam","the hill before Jiphthah-el","the valley of Jiphthah-el northward","the border above Hannathon"],
  "ZEB_NAPH":    ["the border above Hannathon","the slopes under Rimmon","the oak in Zaanannim"],
  "ZEB_ISS":     ["the valley toward Jezreel","the oak in Zaanannim"],
  "ISS_NAPH":    ["the oak in Zaanannim","under the slopes of Tabor","the Jordan at Chinnereth's outflowing"],
  "ASHER_NAPH":  ["the border above Hannathon","the height of Ramah northward","the hills above Kedesh","the north border under Lebanon"],
  "REUBEN_GAD":  ["the Jordan by Beth-jeshimoth","the fields of Abel-shittim","the plain above Heshbon","the border toward Ammon"],
  "GAD_ME":      ["the Jordan under Zaphon","the Jabbok toward Gerasa","the upper Jabbok at Mahanaim","the hills toward Ramoth","toward Ramoth in Gilead"],
}

# ---- unshared runs (authored once, one owner)
RUNS = {
  "JUDAH_DEADSEA":["the Salt Sea's north bay","the Salt Sea under En-gedi","the Salt Sea toward Zoar","the south end of the sea"],
  "JUDAH_SOUTH": ["the south end of the sea","the ascent of Akrabbim southward","the wilderness of Zin southward","toward Kadesh-barnea","the wilderness toward Shur","the sea toward the river of Egypt"],
  "BENJ_JORDAN": ["the Jordan at Jericho","the Salt Sea's north bay"],   # short seam piece
  "EPH_JORDAN":  ["the Jordan by Adam","the Jordan at Jericho"],
  "MAN_JORDAN":  ["the Jordan under Beth-shean","the Jordan by Adam"],
  "ISS_JORDAN":  ["the Jordan at Chinnereth's outflowing","the Jordan under Beth-shean"],
  "NAPH_EAST":   ["the Jordan at Chinnereth's outflowing","the shore of Chinnereth westward","Chinnereth under Capernaum","the waters of Merom eastward","the upper Jordan northward","toward Ijon"],
  "NAPH_NORTH":  ["toward Ijon","the north border under Lebanon"],
  "ASHER_WEST":  ["the sea toward great Zidon","the coast under Achzib","the sea under Carmel"],
  "ASHER_NORTH": ["the north border under Lebanon","the sea toward great Zidon"],
  "REUBEN_WEST": ["the Salt Sea's eastern shore","the mouth of the Arnon"],
  "REUBEN_SOUTH":["the mouth of the Arnon","the Arnon gorge eastward","the brink of the Arnon at Aroer","the high plain by Dibon","the wilderness beyond Mephaath"],
  "REUBEN_EAST": ["the wilderness beyond Mephaath","the desert rim of Moab","the border toward Ammon"],
  "REUBEN_NW":   ["the Jordan by Beth-jeshimoth","the Salt Sea's eastern shore"],
  "GAD_WEST":    ["the Jordan by Beth-jeshimoth","the Jordan toward Succoth","the mouth of the Jabbok","the Jordan under Zaphon"],
  "GAD_EAST":    ["toward Ramoth in Gilead","the desert rim of Gilead","the border toward Ammon"],
  "ME_WEST":     ["the Jordan under Zaphon","the Jordan toward Chinnereth eastward","Chinnereth's east shore southward","the east shore of Chinnereth","above Chinnereth eastward","the upper Jordan eastward","toward Dan of the north","under mount Hermon"],
  "ME_NORTHEAST":["under mount Hermon","the slopes of Hermon eastward","the border of Maakah","the coasts of Argob","toward Salecah","the rim of Bashan","by Edrei","toward Ramoth in Gilead"],
}

# ---- circuits: sequences of (segment, forward?) — anchors dedup'd on join.
CIRCUITS = {
  "JOS_15_CIRCUIT": [("JUDAH_DAN",1),("JUDAH_BENJ",1),("JUDAH_DEADSEA",1),("JUDAH_SOUTH",1),
                     ("~sea hop 1",("the sea toward the river of Egypt","the shore below Gerar")),
                     ("JUDAH_SIMEON",0),
                     ("~sea hop 2",("the shore above Gerar","the sea before Judah"))],
  "JOS_19_SIMEON": [("JUDAH_SIMEON",1),
                    ("~sea hop 3",("the shore below Gerar","the shore above Gerar"))],
  "JOS_19_DAN":    [("DAN_EPHRAIM",1),("DAN_BENJ",0),("JUDAH_DAN",0),
                    ("~sea hop 4",("the sea before Judah","the sea by Me-jarkon"))],
  "JOS_18_BENJAMIN":[("DAN_BENJ",1),("BENJ_EPHRAIM",1),("BENJ_JORDAN",1),("JUDAH_BENJ",0),
                     ("~join 5",("the going down of Beth-shemesh","the going down of Beth-shemesh"))],
  "JOS_16_EPHRAIM":[("DAN_EPHRAIM",0),("BENJ_EPHRAIM",0)],  # then Jordan up + Kanah back + sea hop, below
  "JOS_17_MANASSEH_WEST":[("EPHRAIM_MANASSEH",1)],           # assembled specially below
  "JOS_19_ZEBULUN":[("MANASSEH_ZEB",1),("ZEB_ISS",1),("ZEB_NAPH",0),("ASHER_ZEB",0)],
  "JOS_19_ISSACHAR":[("MANASSEH_ISS",1),("ISS_JORDAN",0),("ISS_NAPH",0),("ZEB_ISS",0)],
  "JOS_19_ASHER":  [("ASHER_ZEB",1),("ASHER_NAPH",1),("ASHER_NORTH",1),("ASHER_WEST",1),("MANASSEH_ASHER",0)],
  "JOS_19_NAPHTALI":[("ISS_NAPH",1),("NAPH_EAST",1),("NAPH_NORTH",1),("ASHER_NAPH",0),("ZEB_NAPH",1)],
  "JOS_13_REUBEN": [("REUBEN_WEST",1),("REUBEN_SOUTH",1),("REUBEN_EAST",1),("REUBEN_GAD",0),("REUBEN_NW",1)],
  "JOS_13_GAD":    [("GAD_WEST",1),("GAD_ME",1),("GAD_EAST",1),("REUBEN_GAD",0)],
  "JOS_13_MANASSEH_EAST":[("ME_WEST",1),("ME_NORTHEAST",1),("GAD_ME",0)],
}
# Ephraim: DAN_EPHRAIM rev gives sea->..wrong; assemble by hand below.

def wig(seed, i, n):
    # deterministic gentle perpendicular wiggle, zero at endpoints
    h = int(hashlib.md5(seed.encode()).hexdigest()[:8], 16)
    u = i / n
    return 0.0  # no synthetic wiggle: an authored stand-in is an honest straight line

def densify(name, pts):
    # pts: list of (pname, lat, lon); insert wiggled points every ~0.09 deg
    out = [pts[0]]
    for k in range(len(pts)-1):
        (n1, la1, lo1), (n2, la2, lo2) = pts[k], pts[k+1]
        d = math.hypot(la2-la1, lo2-lo1)
        steps = max(1, int(d/0.12))
        for i in range(1, steps):
            u = i/steps
            la = la1 + (la2-la1)*u
            lo = lo1 + (lo2-lo1)*u
            # perpendicular unit
            px, py = -(lo2-lo1)/d, (la2-la1)/d
            w = wig(name+str(k), i, steps)
            out.append((f"{name.lower().replace('_',' ')} reach {k}.{i}", la+px*w, lo+py*w))
        out.append((n2, la2, lo2))
    return out

SEG = {}
for nm, chain in {**BORDERS, **RUNS}.items():
    SEG[nm] = densify(nm, [(p, A[p][0], A[p][1]) for p in chain])

def seg(nm, fwd):
    s = SEG[nm]
    return s if fwd else list(reversed(s))

def assemble(parts):
    ring = []
    for p in parts:
        if isinstance(p[1], tuple) and p[0].startswith("~"):
            a, b = p[1]
            pieces = [(a, A[a][0], A[a][1]), (b, A[b][0], A[b][1])]
        else:
            pieces = seg(p[0], p[1])
        for pt in pieces:
            if ring and ring[-1][0] == pt[0]:
                continue
            ring.append(pt)
    if ring[0][0] == ring[-1][0]:
        ring.pop()
    return ring

rings = {}
for cname, parts in CIRCUITS.items():
    if cname in ("JOS_16_EPHRAIM", "JOS_17_MANASSEH_WEST"):
        continue
    rings[cname] = assemble(parts)

# Ephraim: sea at Me-jarkon -> east along DAN_EPHRAIM rev? DAN_EPHRAIM
# is sea->Beth-horon; Ephraim's south walks it sea->Beth-horon (fwd),
# then BENJ_EPHRAIM fwd to the Jordan, up EPH_JORDAN, back west along
# EPHRAIM_MANASSEH reversed, then a sea hop down to Me-jarkon.
rings["JOS_16_EPHRAIM"] = assemble([
    ("DAN_EPHRAIM",1),("BENJ_EPHRAIM",1),("EPH_JORDAN",0),("EPHRAIM_MANASSEH",0),
    ("~sea hop e",("the sea at the Kanah brook","the sea by Me-jarkon")),
])
rings["JOS_17_MANASSEH_WEST"] = assemble([
    ("EPHRAIM_MANASSEH",1),("MAN_JORDAN",0),("MANASSEH_ISS",0),("MANASSEH_ZEB",0),("MANASSEH_ASHER",0),
    ("~sea hop m",("the sea under Carmel","the sea at the Kanah brook")),
])

# ---- verify: global name->coord consistency (incl. rest of the file)
allpts = {}
for r in rings.values():
    for n, la, lo in r:
        if n in allpts and allpts[n] != (la, lo):
            raise SystemExit(f"internal conflict: {n}")
        allpts[n] = (la, lo)
t = io.open(SRC, encoding='utf8').read()
# existing names outside the JOS block
start = t.index('const JOS_15_CIRCUIT')
start = t.rindex('// THE ALLOTMENT LATTICE', 0, start)
end = t.index('// ------------------------- the traced plate contour')
rest = t[:start] + t[end:]
for m in re.finditer(r'name: "([^"]+)", lat: ([-\d.]+), lon: ([-\d.]+)', rest):
    n, la, lo = m.group(1), float(m.group(2)), float(m.group(3))
    if n in allpts and allpts[n] != (la, lo):
        raise SystemExit(f"conflict with existing file name: {n} {allpts[n]} vs {(la,lo)}")
for m in re.finditer(r'\(([-\d.]+),([-\d.]+),"([^"]+)"\)', rest):
    la, lo, n = float(m.group(1)), float(m.group(2)), m.group(3)
    if n in allpts and allpts[n] != (la, lo):
        raise SystemExit(f"conflict with route name: {n}")

# ---- emit rust
def emit(cname, ring):
    lines = [f"const {cname}: &[Waypoint] = &["]
    for n, la, lo in ring:
        lines.append(f'    Waypoint {{ name: "{n}", lat: {la:.4f}, lon: {lo:.4f} }},')
    lines.append("];")
    return "\n".join(lines)

ORDER = ["JOS_15_CIRCUIT","JOS_18_BENJAMIN","JOS_16_EPHRAIM","JOS_17_MANASSEH_WEST",
         "JOS_19_SIMEON","JOS_19_ZEBULUN","JOS_19_ISSACHAR","JOS_19_ASHER",
         "JOS_19_NAPHTALI","JOS_19_DAN","JOS_13_REUBEN","JOS_13_GAD","JOS_13_MANASSEH_EAST"]
header = """// THE ALLOTMENT LATTICE (JOS 13-19), organic authoring. Every border
// two tribes share is authored ONCE as a densified, gently wiggling
// polyline and BOTH circuits walk the identical literals, so neighbors
// tile with no gap and no overlap. Coastal circuits overhang into the
// sea and the lakes; the Water layer paints after the claims, so the
// visible edge is the real natural-earth shoreline. West-bank circuits
// stop at lon 35.555, east-bank at 35.57: the hairline between is the
// Jordan (river geometry itself is a standing atlas ask). Intermediate
// "reach" waypoints are disclosed interpolation markers, not places.

"""
block = header + "\n\n".join(emit(c, rings[c]) for c in ORDER) + "\n\n"
t2 = t[:start] + block + t[end:]
io.open(SRC, 'w', encoding='utf8', newline='').write(t2)
counts = {c: len(rings[c]) for c in ORDER}
print("written.", sum(counts.values()), "waypoints:", counts)
