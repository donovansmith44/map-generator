# OpenBible.info data

`regions.geojson` is derived from
[Bible-Geocoding-Data](https://github.com/openbibleinfo/Bible-Geocoding-Data)
by OpenBible.info (Stephen Smith), licensed under
[Creative Commons Attribution 4.0](https://creativecommons.org/licenses/by/4.0/).

The derivation (tools/plate_trace/vendor_openbible.py): for each of
Philistia, Phoenicia, Geshur, Ammon, Moab, and Edom, the 50% confidence
isoband of the region's geometry was taken as its outline; borders
facing a tribal ring (data/wikimedia/), the real Mediterranean, or a
Natural Earth lake were snapped and spliced onto those rings so shared
borders exist once. The snap budget is measured per region: its own
maximum isoband spacing plus the tribal map's maximum georeference
error (6.3 km).
