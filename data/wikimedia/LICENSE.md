# Wikimedia Commons data

`tribes12.geojson` is derived from
["12 Tribes of Israel Map.svg"](https://commons.wikimedia.org/wiki/File:12_Tribes_of_Israel_Map.svg)
on Wikimedia Commons (itself a translation of
["12 tribus de Israel.svg"](https://commons.wikimedia.org/wiki/File:12_tribus_de_Israel.svg)),
licensed under the
[Creative Commons Attribution-ShareAlike 3.0](https://creativecommons.org/licenses/by-sa/3.0/deed.en)
license. The derivation (tools/plate_trace/vendor_tribes12.py):

- each tribe's visible region was read from a painter's-order raster of
  the SVG and identified by the fill color under its label;
- the SVG plane was georeferenced by fitting an affine transform on the
  map's own city dot markers against the known coordinates of 19
  identified tells (mean residual 2.3 km, max 6.3 km);
- Manasseh, drawn across the Jordan, was split mechanically at the
  OSM-derived Jordan corridor;
- borders the SVG draws against its own sea and lakes were snapped to
  the real shorelines (Natural Earth) by adjacency;
- shared borders between tribes were spliced to a single polyline.

This derived file is likewise available under CC BY-SA 3.0.
