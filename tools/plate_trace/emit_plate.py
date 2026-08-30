# Emit the traced Canaan contour as a circuit constant + SurveySpec.
import io
import numpy as np

SRC = r'C:\Users\donov\Documents\the-best-maps-ever\crates\map-adapters\src\surveys.rs'
LL = np.load(r'C:/Users/donov/.claude/jobs/c6946bce/tmp/green_lonlat.npy')  # (lon, lat)

lines = ["""// ------------------------- the traced plate contour (calibration proof)
//
// One region of the owner's reference plate, georeferenced and traced:
// the pixel->position function is an affine fit over 12 detected city
// dots (mean residual 1.6 km, max 2.8 km), the border is the region's
// color mask contour (~75 m/px), Douglas-Peucker simplified at ~300 m.
// Every waypoint is an interpolation marker of that tracing, not a
// place. This is the precision reference the tribal circuits converge
// to; the method spreads region by region.

const PLATE_CANAAN_CONTOUR: &[Waypoint] = &["""]
for i, (lon, lat) in enumerate(LL):
    lines.append(f'    Waypoint {{ name: "canaan contour {i:03d}", lat: {lat:.5f}, lon: {lon:.5f} }},')
lines.append("];\n")
block = "\n".join(lines)

t = io.open(SRC, encoding='utf8').read()
anchor = "// --------------------------------- the table of nations"
marker = "// ------------------------- the traced plate contour"
if marker in t:
    i = t.index(marker)
    j = t.index("];", t.index("const PLATE_CANAAN_CONTOUR")) + 2
    t = t[:i] + block.rstrip() + t[j:]
else:
    t = t.replace(anchor, block + chr(10) + anchor, 1)

spec_anchor = """const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "NUM34","""
spec_new = """const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "PLATE-CANAAN",
        label: "Canaan (traced contour)",
        note: "Georeferenced tracing of the reference plate's Canaan region \\
               (affine calibration over 12 city dots, mean residual 1.6 km); \\
               waypoints are tracing markers, not places.",
        book: 4, chapter: 34, verse_from: 1, verse_to: 12,
        year: -2200,
        grade: Grade::CityDerived,
        circuit: PLATE_CANAAN_CONTOUR,
    },
    SurveySpec {
        tag: "NUM34","""
if 'tag: "PLATE-CANAAN"' not in t:
    t = t.replace(spec_anchor, spec_new, 1)
io.open(SRC, 'w', encoding='utf8', newline='').write(t)
print("emitted", len(LL), "waypoints + spec")
