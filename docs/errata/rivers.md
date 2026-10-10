# River census and Natural Earth replacement (MAPS-X1)

## Authority and decisions

Scripture grounds river identities and boundary roles, never the precision of
modern vertices. The golden specifications are atlas C12's
`docs/superpowers/specs/2026-10-07-golden-map-{1446bc,1399bc}.md`.
The source is the already-vendored
`data/natural-earth/ne_10m_rivers_lake_centerlines.geojson`, SHA256
`bb854a900ecbd3b408df46d5e16e3e0f974ba55993f9d8b5c26e855273c0905a`.
It contains 1,455 features: 1,202 River and 253 Lake Centerline features.
The adapter preserves all features, multipart courses, identities, names and
vertices. Missing names remain unnamed. One source feature, Loire NE 178000,
has empty multipart geometry: it is retained as typed Unlocated and logged by
the compiler, never drawn. Malformed/incomplete nonempty paths are refused.

Natural Earth declares its vector data public domain in its
[terms](https://www.naturalearthdata.com/about/terms-of-use/).
Its [river documentation](https://www.naturalearthdata.com/downloads/10m-physical-vectors/10m-rivers-lake-centerlines/)
describes generalized modern drainages primarily derived from World Data Bank 2,
with adjustments to relief and optional lake centerlines. These are modern
courses, not evidence that an ancient branch, coastline or river reach existed
at either golden stop. Intermittent rivers are not comprehensively supplied.

DECISIONS:

- Remove the entire OSM river vendor, including the two buffered Jordan
  corridor polygons. ODbL is excluded by the owner's licence rule. The excluded
  drawing supplies no replacement geometry and no coordinate or topology input.
- Keep Natural Earth's connected Jordan drainage in the existing NE lake-admission
  Canaan window (29–34.6 N, 33.5–37.8 E), at its native source detail. Connect
  source parts by Natural Earth's river number, including lake centerlines for
  the connectivity check; clip water-interior portions against the existing NE
  sea/lake rings. No interpolation fills a missing river part.
- Remove the old 30 km network-length cutoff and the OSM-derived 5 km mouth
  allowance. Natural Earth supplies the drainage, so neither a styling length
  nor a proximity-derived endorheic classification selects its existence.
- Missing below means missing from this vendored source, never absent on earth
  or absent in Scripture. Such courses remain recorded and undrawn. X16 may
  later supply a permitted, justified course from its public-domain witnesses.
- Nile, Euphrates and Tigris are retained by the global source adapter but lie
  outside that Canaan admission window. X1 does not expand the frame or
  invent historical branches to make the world goldens complete.

## Every named golden river and river identification

A Course row means the vendored NE drainage exists. It does not assert ancient
course precision. A Missing row is the explicit unlocated-course record. The
last column retains the chosen identification, grounds and relevant alternatives.

| River | Outcome | Stop / grounds / source identification and alternatives |
|---|---|---|
| Jordan | Course | Both; Num 13:29; Deut 3:17; Josh 13:23, 27; 19:34. NE 229, 138 river vertices and 23 lake-centerline vertices in five source parts. Upper-Jordan reading primary at Josh 19:34; Havoth-jair and town-Judah readings remain golden-spec alternatives. |
| Nile | Course | Both, Egypt/Kush physical anchor; Exod 1–14; Gen 47:11. NE 4, with Rosetta 19 and Damietta 20. Modern branches do not locate the ancient Pelusiac branch. Outside the Canaan admission window. |
| Euphrates | Course | 1446, Egyptian campaign/upper Mesopotamia; golden 1446 §4. NE 62 (Euphrates), 65 (Firat), 69 (Al Furat). Outside the Canaan admission window. |
| Tigris | Course | 1446, Assyria/upper Mesopotamia; golden 1446 §4. NE 135 (Tigris), 120 (Dicle). Outside the Canaan admission window. |
| Orontes | Missing | Both, Kadesh/Asian sphere; golden 1446 §4, 1399 §4.8. No Orontes course in the vendored NE file. |
| Eleutheros | Missing | 1446, Asian sphere's changing northern reach; golden 1446 §4. Nahr el-Kabir identification; no NE course. This campaign limit never justifies a fixed state boundary. |
| Khabur | Missing | 1446, Mitanni's Khabur triangle; golden 1446 §4. No NE course. |
| Arnon | Missing | Both; Num 21:13, 26; Judg 11:18. Wadi Mujib identification. No NE course; the 1399 border remains a recorded geometry gap, not a straight chord. |
| Jabbok | Missing | Both; Num 21:24; Deut 3:16; Josh 12:2. Upper Wadi Zarqa identification. No NE course; Gad reaching into Ammon itself remains the golden-spec alternative. |
| Zered | Missing | Both; Num 21:11–12; Deut 2:8–14. Wadi el-Hasa identification; border role interpreted, Low. No NE course; Scripture does not call it Moab's boundary. |
| Kanah | Missing | 1399; Josh 16:8; 17:9–10. Wadi Qana identification for the brook, not the separate city Qana of Josh 19:28. No NE course. |
| Kishon | Missing | 1399; Josh 19:11's river before Jokneam is a tributary; Shihor-libnath chosen at its mouth, golden 1399 §5.4. No NE course. |
| Yarkon | Missing | 1399; Josh 19:46 Me-jarkon, golden 1399 §5.4. No NE course. |
| Yarmuk | Missing | 1399 identification discussion for Zaphon, golden 1399 §5.4; the PEF's conflation of Amathus and el-Hammeh is recorded, not a new chosen river boundary. No NE course. |
| Belus | Missing | 1399 alternative Shihor-libnath identification: PEF Nahr Na'amein (Handbook pp. 267–268), golden 1399 §5.4. Alternative remains recorded, capped below High; no NE course. |
| Shihor-libnath | Missing | 1399; Josh 19:26. Kishon mouth primary, Belus alternative (§5.4); no separately located NE course. A name match is not scriptural site derivation. |
| River of Egypt | Missing | 1399; Josh 15:4, 47. Wadi el-Arish primary; Pelusiac Nile/Sihor alternative (Josh 13:3; 1 Chr 13:5), golden §6. No NE Wadi el-Arish course. |
| Pelusiac Nile | Missing | 1399 River-of-Egypt alternative; golden §6. No permitted ancient branch course in the modern NE file; Rosetta/Damietta are not substitutions. |

## Kept, lost detail and removed

The previous OSM vendor had 1,752 LineString features in 125 network keys and
two Jordan corridor polygons. These counts describe the removed file, not the
compiled plate: the old 30 km and drainage filters selected a subset of it.
Its 563 compiled river paths reported by X0 are the pre-X1 checkpoint.

Kept with less detail: the Jordan main drainage. NE has five generalized parts
(three River, two Lake Centerline), 161 source vertices in total. OSM's explicit
Jordan name covered 38 line features / 452 vertices; their meanders and the two corridor areas
are not retained. NE connectivity is checked before water clipping; the final
noded/shoreline-clipped count is reported by the real compiler.

Removed detail with no replacement course: Jordan tributaries and headwaters
(Yarmuk/Yarmukh, Zarqa, Hasbani/Snir, Dan, Ruqqad); Arnon/Mujib, Zered/Hasa,
Kishon, Yarkon/Ayalon, Besor/Gaza/Beersheba, Sorek, Alexander, Shikma, Zin,
Paran, Arava, Wadi el-Arish; Lebanon/Syria's Orontes, Litani, Beirut,
Nahr el-Jaouz, Abou Aali, Awali, Damour, Zahrani, Berdawni, Chtoura,
Ghzayyel, Barada and Al Aawaj, and unnamed/other desert drainage details.
Every other old network is also removed: only the NE Jordan system intersects
this admission window. The NE feature named Litani is in South America
(about 54 W, 2–4 N), so it is explicitly not the Lebanese replacement.
Names and old source decisions remain in git history and this census;
excluded vertices and buffered polygons never become alternative drawn courses.

## Library survey

Reuse maintained [serde_json](https://docs.rs/serde_json/latest/serde_json/)
(MIT/Apache-2.0), already depended upon, to decode JSON. Read the narrow NE
properties/geometry schema into RiverNumber, RiverCourse and RiverSource at the
adapter door; its closed RiverError maps syntax and shape refusals.
[geojson](https://github.com/georust/geojson) is maintained, MIT/Apache-2.0,
and supplies broader geometry types; it would require an unassigned dependency
change without simplifying this two-course schema. GDAL is maintained and
MIT-style, suitable for conversion, but this source is already GeoJSON and no
conversion/reprojection is needed. No JSON tokenizer, generic format machinery,
new dependency, source conversion or coordinate tuning is introduced.

## Worker report and golden differences

Extra Codex carries Claude X1 on `lane/claude/MAPS-X1`, base X0 `9cb7e9a`,
detached worktree `~/w/CX2-w-x1`. Main compiler editing reservation belongs to X1.

Red observed before replacement: the no-OSM-input law failed on the compiler's
`data/osm` read; the named-golden-river law failed because the missing-course
census did not exist. The adapter fidelity law then failed with 0 admitted
features versus all 1,455 source features before implementing the adapter.

Expected golden differences, not approved pixel measurements: fewer river
lines/meanders, removal of the two Jordan corridor fills and their shoreline
interactions, generalized Jordan paths, and tributaries/boundary rivers missing
where NE has no course. These changes can affect both Canaan and hemisphere
probes. No renderer compensation or re-blessing is authorized or performed.

Gate outcomes and exact compiled/golden blockers will be appended here.

## Complete removed source-label inventory

These are source labels and line-feature counts, not asserted biblical
identifications or a list of rivers proven absent. Every row is removed as
an OSM drawing; only the Jordan drainage has a NE replacement in this frame.

| Removed OSM label | Line features | Replacement in frame |
|---|---:|---|
| (unnamed) | 775 | Missing; recorded, undrawn |
| Afifon | 1 | Missing; recorded, undrawn |
| Al Aawaj | 11 | Missing; recorded, undrawn |
| An Najili | 1 | Missing; recorded, undrawn |
| Ayalon River | 2 | Missing; recorded, undrawn |
| Barada River | 113 | Missing; recorded, undrawn |
| Beirut River | 12 | Missing; recorded, undrawn |
| Berdawni River | 37 | Missing; recorded, undrawn |
| Chtoura River | 10 | Missing; recorded, undrawn |
| Far Wadi al Abyad | 1 | Missing; recorded, undrawn |
| Ghzayyel River | 10 | Missing; recorded, undrawn |
| Hafir River | 10 | Missing; recorded, undrawn |
| Hala-Yahfoufa River | 19 | Missing; recorded, undrawn |
| Hasbani River | 46 | Missing; recorded, undrawn |
| JIḎAYʻAT H̱IDRIJ | 2 | Missing; recorded, undrawn |
| Jadhi Abu Hawaya | 1 | Missing; recorded, undrawn |
| Jaouz River | 1 | Missing; recorded, undrawn |
| Jashat al Adla | 1 | Missing; recorded, undrawn |
| Litani River | 58 | Missing; recorded, undrawn |
| Nahal Alexander | 1 | Missing; recorded, undrawn |
| Nahal Arava | 7 | Missing; recorded, undrawn |
| Nahal Beersheba | 11 | Missing; recorded, undrawn |
| Nahal Dan | 3 | Missing; recorded, undrawn |
| Nahal HaBsor | 9 | Missing; recorded, undrawn |
| Nahal Kishon | 23 | Missing; recorded, undrawn |
| Nahal Nekarot | 1 | Missing; recorded, undrawn |
| Nahal Paran | 2 | Missing; recorded, undrawn |
| Nahal Ramon | 1 | Missing; recorded, undrawn |
| Nahal Shikma | 9 | Missing; recorded, undrawn |
| Nahal Snir (Hatsbani) | 2 | Missing; recorded, undrawn |
| Nahal Sorek | 8 | Missing; recorded, undrawn |
| Nahal Yarkon | 7 | Missing; recorded, undrawn |
| Nahal Yarmukh | 6 | Missing; recorded, undrawn |
| Nahal Zin | 6 | Missing; recorded, undrawn |
| Nahal Zvira | 1 | Missing; recorded, undrawn |
| Nahr Abou Aali | 7 | Missing; recorded, undrawn |
| Nahr Aray | 1 | Missing; recorded, undrawn |
| Nahr El Awali | 2 | Missing; recorded, undrawn |
| Nahr El Barouk | 2 | Missing; recorded, undrawn |
| Nahr El Kebir | 1 | Missing; recorded, undrawn |
| Nahr El Safa | 4 | Missing; recorded, undrawn |
| Nahr Jaair | 12 | Missing; recorded, undrawn |
| Nahr ed Damour | 9 | Missing; recorded, undrawn |
| Nahr el Aouali | 2 | Missing; recorded, undrawn |
| Nahr el Ghaziri | 1 | Missing; recorded, undrawn |
| Nahr el Jaouz | 2 | Missing; recorded, undrawn |
| Nahr ez Zahrani | 16 | Missing; recorded, undrawn |
| Orontes | 5 | Missing; recorded, undrawn |
| Rijlat ash Sharari | 1 | Missing; recorded, undrawn |
| River Jordan | 38 | NE 229, generalized course |
| Ruqqad | 1 | Missing; recorded, undrawn |
| SHAGHĪYAT UMM QUṮAYFAH | 1 | Missing; recorded, undrawn |
| SHAʻĪB ABŪ NAJM | 1 | Missing; recorded, undrawn |
| SHAʻĪB ABŪ ṮURAYFĪYAH | 1 | Missing; recorded, undrawn |
| SHAʻĪB AL MIQYĀL | 1 | Missing; recorded, undrawn |
| SHAʻĪB DĀFINAH | 1 | Missing; recorded, undrawn |
| SHAʻĪB UMM AL LUQĀʼ | 1 | Missing; recorded, undrawn |
| Sayniq River | 4 | Missing; recorded, undrawn |
| TALʻAT UMM LABAN | 1 | Missing; recorded, undrawn |
| TALʻAT ZĀYID | 1 | Missing; recorded, undrawn |
| Valley of Gaza (Wadi Ghazza) | 10 | Missing; recorded, undrawn |
| WĀDī ABŪ GHAḎĀ’ | 1 | Missing; recorded, undrawn |
| WĀDī ABŪ H̱AWĀYAH | 1 | Missing; recorded, undrawn |
| WĀDī ABŪ ṮURAYFīYAH | 2 | Missing; recorded, undrawn |
| WĀDī ABŪ ‘ALDĀ | 3 | Missing; recorded, undrawn |
| WĀDī ABŪ ‘AWĀDHIR | 1 | Missing; recorded, undrawn |
| WĀDī AḎ ḎĀH̱IKīYAH | 1 | Missing; recorded, undrawn |
| WĀDī AL ASMAR | 1 | Missing; recorded, undrawn |
| WĀDī AL BUWAYB | 1 | Missing; recorded, undrawn |
| WĀDī AL FīHAH | 1 | Missing; recorded, undrawn |
| WĀDī AL H̱IMĀRAH | 2 | Missing; recorded, undrawn |
| WĀDī AL MALĀWīH̱ | 1 | Missing; recorded, undrawn |
| WĀDī AL QĀSIMAH | 1 | Missing; recorded, undrawn |
| WĀDī AL ‘ARAY‘ARAYYĀT | 1 | Missing; recorded, undrawn |
| WĀDī ASH SHAHAYBĀ' | 1 | Missing; recorded, undrawn |
| WĀDī DĀBIS | 1 | Missing; recorded, undrawn |
| WĀDī FAKK | 2 | Missing; recorded, undrawn |
| WĀDī GHUḎAYY | 1 | Missing; recorded, undrawn |
| WĀDī H̱US̱AYYIDAH UMM NAKHLAH | 3 | Missing; recorded, undrawn |
| WĀDī QARĀQIR | 2 | Missing; recorded, undrawn |
| WĀDī SI‘D AL BARS̱Ā’ | 1 | Missing; recorded, undrawn |
| WĀDī SI‘D AL H̱AMRĀ’ | 5 | Missing; recorded, undrawn |
| WIDYĀN ASH SHUṮṮīYĀT | 1 | Missing; recorded, undrawn |
| Wadi Abu Buthayran | 3 | Missing; recorded, undrawn |
| Wadi Abu Gharisah | 2 | Missing; recorded, undrawn |
| Wadi Abu Handal | 2 | Missing; recorded, undrawn |
| Wadi Abu Mutamir | 1 | Missing; recorded, undrawn |
| Wadi Abu Muzrayqat | 5 | Missing; recorded, undrawn |
| Wadi Abu Qarazif | 4 | Missing; recorded, undrawn |
| Wadi Abu Sharib | 1 | Missing; recorded, undrawn |
| Wadi Abu Sulaylat | 1 | Missing; recorded, undrawn |
| Wadi Abu Tarfa | 1 | Missing; recorded, undrawn |
| Wadi Abu Treifia | 10 | Missing; recorded, undrawn |
| Wadi Aheimar | 1 | Missing; recorded, undrawn |
| Wadi Al 'Aliya | 1 | Missing; recorded, undrawn |
| Wadi Al-Seeq | 1 | Missing; recorded, undrawn |
| Wadi Arada | 1 | Missing; recorded, undrawn |
| Wadi Ba'ir | 8 | Missing; recorded, undrawn |
| Wadi Dhulayl | 1 | Missing; recorded, undrawn |
| Wadi Fakk Abu Dureisa | 2 | Missing; recorded, undrawn |
| Wadi Fakk Abu Thiran | 1 | Missing; recorded, undrawn |
| Wadi Garf al Diyuf | 1 | Missing; recorded, undrawn |
| Wadi Ghada | 3 | Missing; recorded, undrawn |
| Wadi Ghudayy | 1 | Missing; recorded, undrawn |
| Wadi Gira | 6 | Missing; recorded, undrawn |
| Wadi Hamra | 1 | Missing; recorded, undrawn |
| Wadi Husaydat Umm Ghurubat | 1 | Missing; recorded, undrawn |
| Wadi Jais | 1 | Missing; recorded, undrawn |
| Wadi Kulwah | 2 | Missing; recorded, undrawn |
| Wadi Mardi | 1 | Missing; recorded, undrawn |
| Wadi Marzi | 1 | Missing; recorded, undrawn |
| Wadi Mudaysis | 13 | Missing; recorded, undrawn |
| Wadi Mujib | 3 | Missing; recorded, undrawn |
| Wadi Qatara | 1 | Missing; recorded, undrawn |
| Wadi Rihab | 1 | Missing; recorded, undrawn |
| Wadi Ruqqad | 1 | Missing; recorded, undrawn |
| Wadi Safra | 1 | Missing; recorded, undrawn |
| Wadi Salayta | 1 | Missing; recorded, undrawn |
| Wadi Suada al Barsa | 1 | Missing; recorded, undrawn |
| Wadi Umm Jufayn | 1 | Missing; recorded, undrawn |
| Wadi Umm Laban | 1 | Missing; recorded, undrawn |
| Wadi Umm Qubur | 3 | Missing; recorded, undrawn |
| Wadi Umm Rujm | 1 | Missing; recorded, undrawn |
| Wadi Umm Sayyid | 10 | Missing; recorded, undrawn |
| Wadi Umm Urtah | 3 | Missing; recorded, undrawn |
| Wadi Umm Zulaykhah | 1 | Missing; recorded, undrawn |
| Wadi Uqabah | 3 | Missing; recorded, undrawn |
| Wadi Zerka Ma'in | 5 | Missing; recorded, undrawn |
| Wadi al Aradah | 1 | Missing; recorded, undrawn |
| Wadi al Bayda | 1 | Missing; recorded, undrawn |
| Wadi al Buruk | 4 | Missing; recorded, undrawn |
| Wadi al Demtha | 1 | Missing; recorded, undrawn |
| Wadi al Gaifi | 5 | Missing; recorded, undrawn |
| Wadi al Ghinah | 1 | Missing; recorded, undrawn |
| Wadi al Ghudaywiyat | 1 | Missing; recorded, undrawn |
| Wadi al Girafi | 2 | Missing; recorded, undrawn |
| Wadi al Hasa | 6 | Missing; recorded, undrawn |
| Wadi al Hasah | 2 | Missing; recorded, undrawn |
| Wadi al Hasana | 1 | Missing; recorded, undrawn |
| Wadi al Hasha | 5 | Missing; recorded, undrawn |
| Wadi al Jayfi | 1 | Missing; recorded, undrawn |
| Wadi al Juhfah | 1 | Missing; recorded, undrawn |
| Wadi al Kuwaykabah | 1 | Missing; recorded, undrawn |
| Wadi al Masak | 3 | Missing; recorded, undrawn |
| Wadi al Mindassah | 1 | Missing; recorded, undrawn |
| Wadi al Mushawwah | 1 | Missing; recorded, undrawn |
| Wadi al Qariya | 3 | Missing; recorded, undrawn |
| Wadi al Qasimah | 1 | Missing; recorded, undrawn |
| Wadi al Qureis | 4 | Missing; recorded, undrawn |
| Wadi al Ruaq | 1 | Missing; recorded, undrawn |
| Wadi al Rutami | 1 | Missing; recorded, undrawn |
| Wadi al Sarah | 1 | Missing; recorded, undrawn |
| Wadi al Uitm | 3 | Missing; recorded, undrawn |
| Wadi al-Arish | 5 | Missing; recorded, undrawn |
| Wādī aş Şawt | 1 | Missing; recorded, undrawn |
| Yarmuk River | 5 | Missing; recorded, undrawn |
| Yarmukh | 1 | Missing; recorded, undrawn |
| Zarqa River | 12 | Missing; recorded, undrawn |
| ] | 1 | Missing; recorded, undrawn |
| الضاحكية | 6 | Missing; recorded, undrawn |
| تلعة زايد | 2 | Missing; recorded, undrawn |
| شعبان الناعم | 6 | Missing; recorded, undrawn |
| شعبان برقة الدودة | 1 | Missing; recorded, undrawn |
| شعيب أم أرطى الغينة | 1 | Missing; recorded, undrawn |
| شعيب الجضيعة | 5 | Missing; recorded, undrawn |
| شعيب الدمنة | 4 | Missing; recorded, undrawn |
| شعيب العقيب | 1 | Missing; recorded, undrawn |
| شعيب المطيريد | 1 | Missing; recorded, undrawn |
| شعيب الناعم | 1 | Missing; recorded, undrawn |
| شعيب سيالة | 1 | Missing; recorded, undrawn |
| شعيب مديسيس | 2 | Missing; recorded, undrawn |
| نهر العاصي | 9 | Missing; recorded, undrawn |
| نهر اليرموك | 1 | Missing; recorded, undrawn |
| وادي أبو طرفاء | 1 | Missing; recorded, undrawn |
| وادي أم طليحة | 1 | Missing; recorded, undrawn |
| وادي ابا الحار | 1 | Missing; recorded, undrawn |
| وادي ابو سيلا | 2 | Missing; recorded, undrawn |
| وادي ابو طريفه | 1 | Missing; recorded, undrawn |
| وادي ابو غضى | 1 | Missing; recorded, undrawn |
| وادي الأبيض | 1 | Missing; recorded, undrawn |
| وادي الاحواء | 1 | Missing; recorded, undrawn |
| وادي الاسمر | 1 | Missing; recorded, undrawn |
| وادي البسة | 1 | Missing; recorded, undrawn |
| وادي الجوعليات | 1 | Missing; recorded, undrawn |
| وادي الدحيل | 1 | Missing; recorded, undrawn |
| وادي السرحان | 3 | Missing; recorded, undrawn |
| وادي الشايب | 1 | Missing; recorded, undrawn |
| وادي الشبيكي | 3 | Missing; recorded, undrawn |
| وادي الشعره | 1 | Missing; recorded, undrawn |
| وادي الضبيعاني | 2 | Missing; recorded, undrawn |
| وادي العمق | 13 | Missing; recorded, undrawn |
| وادي العنيق | 1 | Missing; recorded, undrawn |
| وادي الغراء | 4 | Missing; recorded, undrawn |
| وادي الغضيا | 9 | Missing; recorded, undrawn |
| وادي الغينة | 2 | Missing; recorded, undrawn |
| وادي القراحي | 1 | Missing; recorded, undrawn |
| وادي القويصرة | 3 | Missing; recorded, undrawn |
| وادي اللجفة | 1 | Missing; recorded, undrawn |
| وادي اللحاوي | 2 | Missing; recorded, undrawn |
| وادي المترملة | 1 | Missing; recorded, undrawn |
| وادي المحاش | 1 | Missing; recorded, undrawn |
| وادي المعقر | 1 | Missing; recorded, undrawn |
| وادي ام جرفين | 13 | Missing; recorded, undrawn |
| وادي ام لجوج | 3 | Missing; recorded, undrawn |
| وادي ثرف | 5 | Missing; recorded, undrawn |
| وادي ثميد ربيعة | 1 | Missing; recorded, undrawn |
| وادي حدرج | 6 | Missing; recorded, undrawn |
| وادي حصيدة أم طلحة | 8 | Missing; recorded, undrawn |
| وادي حصيدة أم غدران | 2 | Missing; recorded, undrawn |
| وادي حصيدة أم قلات | 1 | Missing; recorded, undrawn |
| وادي حصيدة الفصيا | 12 | Missing; recorded, undrawn |
| وادي خرم عليان | 1 | Missing; recorded, undrawn |
| وادي دبر | 1 | Missing; recorded, undrawn |
| وادي دبل | 1 | Missing; recorded, undrawn |
| وادي دغداش | 1 | Missing; recorded, undrawn |
| وادي رتامه | 1 | Missing; recorded, undrawn |
| وادي سرمداء | 9 | Missing; recorded, undrawn |
| وادي سلادح | 1 | Missing; recorded, undrawn |
| وادي سمرمدة | 1 | Missing; recorded, undrawn |
| وادي سهب الأبيض | 1 | Missing; recorded, undrawn |
| وادي سهب الأسمر | 1 | Missing; recorded, undrawn |
| وادي صبح | 1 | Missing; recorded, undrawn |
| وادي صدار | 1 | Missing; recorded, undrawn |
| وادي صفاة | 1 | Missing; recorded, undrawn |
| وادي عسرات | 3 | Missing; recorded, undrawn |
| وادي عطارة الغباشة | 1 | Missing; recorded, undrawn |
| وادي عفال | 1 | Missing; recorded, undrawn |
| وادي عميق | 2 | Missing; recorded, undrawn |
| وادي عيياد | 1 | Missing; recorded, undrawn |
| وادي مبرك | 7 | Missing; recorded, undrawn |
| وادي واسط | 3 | Missing; recorded, undrawn |

## Gate checkpoint (2026-10-09)

Safe contract subgates: **381 examples / 0 failures**, both contract
totality checks and both vocabulary checks green, **6 semver tests green**.
After sharing the existing lake/river frame predicate, the focused physical
partition law passed once more. Full workspace/make and actual golden gates
are not included in those counts.

Rust source/test LOC delta: **+289** (source +51; tests +238).
The removed OSM vendor is not counted as application code.
Diff checks are clean; no new application or test comments were added.

- Final scoped Cargo gate: **43 passed, 0 failed** (adapters 21, compiler 18,
  X0 integration laws 4). The first aggregate run's stale six-family provenance
  result came from the shared target. Touching this worktree's map-canon library
  timestamp forced its exact three-family source to rebuild; lineage 3/0 and
  the final aggregate 43/0 then passed. No other worker's outputs were cleaned.
- `cargo build -p map-compile -j 2` passes. This packet adds no warning;
  inherited unused Area/PolityRow imports in compile.rs and PolityRow in its
  old tests are Minor, outside X1's files. The now-unused local provenance
  closure in the owned bridge was deleted.
- The real `map-compile build --out /tmp/maps-x1-canon`, audited with
  `strace -f -e openat`, reads no `data/osm` path. It builds **314 faces / 5
  river paths / zero spherical-area residual**, compared with X0's
  **329 / 563 / zero**. It logs the unlocated Loire record. It then refuses
  the existing stale `partition:canaan` canonical registry declaration, as X0
  already reported to X4. Only a scratch reconciliation report was written;
  no committed canon or fixture moved.
- Exact `node crates/map-viewer/tests/golden.js --check` fails before browser
  launch because this worktree lacks `playwright-core`. Port 8090 is free, and
  fresh canon is blocked by X4's registry declaration. The harness additionally
  pins Windows Chrome. Actual colour/golden differences remain unmeasured;
  expected and exact partition-census changes above are disclosed separately.
- Three heavy-lock attempts were refused by C12's live workspace landing gate.
  This worker started no workspace/make gate without the lock and deleted no
  foreign lock. Full `cargo test --workspace` and `make contract-gates` remain
  pending the shared lock; the safe contract subgates are reported separately.

**Important coordination gap:** `map-canon::Witness::PARTITION_INPUTS` still
contains Osm, and its own terms test plus X0's exact-inventory law require it.
Those files are outside X1's assigned paths and now overlap reserved X0/X4/X5
work. The worker requested narrow scope approval asynchronously and has not
received an answer. River lines themselves now carry NaturalEarth provenance,
but composite partition credits still conservatively name ODbL until the
source-inventory owner removes it and updates both tests. This packet is not
a complete permissive-lineage or golden acceptance claim.

**Handoff:** review the source replacement; consume X0's forthcoming F-283/F-284
checkpoint if its branch moves, reconcile composite source credits through the
reserved owner, and consume X4's stale-Canaan repair before fresh canon/goldens.
Do not re-bless. No type mismatch, graph-types/map-types change, raw-data copy,
port/process from another worker, secret or published history was touched.

Final checkpoint: X0 remains `9cb7e9a`; no forward merge is available yet.
All owned test/build/trace processes have ended, no host was started, and
no lock is held by this worker. X1 main.rs reservation is released on this
published source packet; integrate later shared-file changes serially.
