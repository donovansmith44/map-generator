use super::super::{
    CitedRelation, Direction, Extent, Lot, Part, Position, Relation, Subject, Target, Travel,
};

pub(in super::super) fn outcomes() -> Vec<CitedRelation> {
    use Direction::*;
    use Lot::*;
    use Travel::*;
    let groups = vec![
        cited(
            PromisedLand,
            "NUM.34.3",
            vec![travel(From, Unstated, site("NUM.34.3:sea-end"))],
        ),
        cited(
            PromisedLand,
            "NUM.34.4",
            vec![
                travel(Unto, Unstated, site("ascent of Akrabbim")),
                travel(Along, Unstated, site("Zin")),
                position(Subject::Border, Position::SouthOf, site("Kadesh-barnea")),
                travel(Unto, Unstated, site("Hazar-addar")),
                travel(Unto, Unstated, site("Azmon")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.5",
            vec![
                travel(Unto, Unstated, site("river of Egypt")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.6",
            vec![Relation::Boundary {
                target: site("Great Sea"),
            }],
        ),
        cited(
            PromisedLand,
            "NUM.34.7",
            vec![
                travel(From, Unstated, site("Great Sea")),
                travel(Unto, Unstated, site("northern mount Hor")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.8",
            vec![
                travel(Unto, Unstated, site("entrance of Hamath")),
                travel(Outgoing, Unstated, site("Zedad")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.9",
            vec![
                travel(Unto, Unstated, site("Ziphron")),
                travel(Outgoing, Unstated, site("Hazar-enan")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.10",
            vec![
                travel(From, Unstated, site("Hazar-enan")),
                travel(Unto, Unstated, site("Shepham")),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.11",
            vec![
                travel(Unto, Unstated, site("Riblah")),
                position(
                    reference("Riblah", Part::Site),
                    Position::EastOf,
                    site("Ain"),
                ),
                mention("Ain"),
                travel(Unto, Unstated, part("sea of Chinnereth", Part::EastSide)),
            ],
        ),
        cited(
            PromisedLand,
            "NUM.34.12",
            vec![
                travel(Along, Unstated, site("Jordan")),
                travel(Outgoing, Unstated, site("Salt Sea")),
            ],
        ),
        cited(
            Reuben,
            "JOS.13.16",
            vec![
                extent(Extent::From, site("Aroer (JOS.13.16)")),
                position(
                    reference("Aroer (JOS.13.16)", Part::Site),
                    Position::At,
                    part("Arnon", Part::Bank),
                ),
                mention("Arnon"),
                extent(Extent::Includes, site("JOS.13.16:city")),
                position(
                    reference("JOS.13.16:city", Part::Site),
                    Position::Within,
                    site("Arnon"),
                ),
                listed("Medeba"),
            ],
        ),
        cited(
            Reuben,
            "JOS.13.17",
            cities(&["Heshbon", "Dibon", "Bamoth-baal", "Beth-baal-meon"]),
        ),
        cited(
            Reuben,
            "JOS.13.18",
            cities(&["Jahaz", "Kedemoth", "Mephaath"]),
        ),
        cited(
            Reuben,
            "JOS.13.19",
            cities(&["Kirjathaim", "Sibmah", "Zareth-shahar"]),
        ),
        cited(
            Reuben,
            "JOS.13.20",
            cities(&["Beth-peor", "Ashdoth-pisgah", "Beth-jeshimoth"]),
        ),
        cited(
            Reuben,
            "JOS.13.23",
            vec![Relation::Boundary {
                target: site("Jordan"),
            }],
        ),
        cited(
            Gad,
            "JOS.13.25",
            vec![
                extent(Extent::Includes, site("Jazer")),
                extent(Extent::Includes, part("Gilead", Part::Cities)),
                extent(Extent::Half, site("land of Ammon")),
                travel(Unto, Unstated, site("Aroer (JOS.13.25)")),
                position(
                    reference("Aroer (JOS.13.25)", Part::Site),
                    Position::Before,
                    site("Rabbah"),
                ),
                mention("Rabbah"),
            ],
        ),
        cited(
            Gad,
            "JOS.13.26",
            vec![
                travel(From, Unstated, site("Heshbon")),
                travel(Unto, Unstated, site("Ramath-mizpeh")),
                travel(Unto, Unstated, site("Betonim")),
                travel(From, Unstated, site("Mahanaim")),
                travel(Unto, Unstated, part("Debir (JOS.13.26)", Part::Border)),
            ],
        ),
        cited(
            Gad,
            "JOS.13.27",
            vec![
                listed("Beth-aram"),
                listed("Beth-nimrah"),
                listed("Succoth"),
                listed("Zaphon"),
                Relation::Boundary {
                    target: site("Jordan"),
                },
                travel(Unto, Unstated, part("sea of Chinnereth", Part::Edge)),
                position(Subject::Border, Position::EastOf, site("Jordan")),
            ],
        ),
        cited(
            ManassehEast,
            "JOS.13.30",
            vec![
                extent(Extent::From, site("Mahanaim")),
                extent(Extent::Includes, site("Bashan")),
                extent(Extent::Includes, site("towns of Jair")),
                position(
                    reference("towns of Jair", Part::Site),
                    Position::Within,
                    site("Bashan"),
                ),
            ],
        ),
        cited(
            ManassehEast,
            "JOS.13.31",
            vec![
                extent(Extent::Half, site("Gilead")),
                listed("Ashtaroth"),
                listed("Edrei"),
            ],
        ),
        cited(
            Judah,
            "JOS.15.2",
            vec![travel(From, Unstated, site("JOS.15.2:sea-bay"))],
        ),
        cited(
            Judah,
            "JOS.15.3",
            vec![
                travel(Unto, Unstated, part("Maaleh-acrabbim", Part::SouthSide)),
                travel(Along, Unstated, site("Zin")),
                travel(Unto, Unstated, part("Kadesh-barnea", Part::SouthSide)),
                travel(Unto, Unstated, site("Hezron")),
                travel(Unto, Unstated, site("Adar")),
                travel(Unto, Unstated, site("Karkaa")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.4",
            vec![
                travel(Unto, Unstated, site("Azmon")),
                travel(Unto, Unstated, site("river of Egypt")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.5",
            vec![
                Relation::Boundary {
                    target: site("Salt Sea"),
                },
                travel(Unto, Unstated, site("Jordan terminus")),
                travel(From, Unstated, site("JOS.15.5:sea-bay")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.6",
            vec![
                travel(Unto, Unstated, site("Beth-hoglah")),
                position(Subject::Border, Position::NorthOf, site("Beth-arabah")),
                travel(Unto, Unstated, site("Bohan stone")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.7",
            vec![
                travel(Unto, Unstated, site("Debir (JOS.15.7)")),
                travel(From, Unstated, site("valley of Achor")),
                travel(Toward, North, site("Gilgal")),
                position(
                    reference("Adummim", Part::Ascent),
                    Position::SouthOf,
                    site("JOS.15.7:river"),
                ),
                position(
                    reference("Gilgal", Part::Site),
                    Position::Before,
                    part("Adummim", Part::Ascent),
                ),
                mention("JOS.15.7:river"),
                travel(Unto, Unstated, site("En-shemesh")),
                travel(Outgoing, Unstated, site("En-rogel")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.8",
            vec![
                travel(Along, Unstated, site("valley of Hinnom")),
                position(Subject::Border, Position::SouthOf, site("Jerusalem")),
                travel(Unto, Unstated, part("JOS.15.8:mountain", Part::Top)),
                position(
                    reference("JOS.15.8:mountain", Part::Site),
                    Position::Before,
                    site("valley of Hinnom"),
                ),
                position(
                    reference("JOS.15.8:mountain", Part::Site),
                    Position::WestOf,
                    site("valley of Hinnom"),
                ),
                position(
                    reference("JOS.15.8:mountain", Part::Site),
                    Position::At,
                    part("Rephaim valley", Part::NorthEnd),
                ),
                mention("Rephaim valley"),
            ],
        ),
        cited(
            Judah,
            "JOS.15.9",
            vec![
                travel(Unto, Unstated, site("Nephtoah")),
                travel(Unto, Unstated, part("mount Ephron", Part::Cities)),
                travel(Unto, Unstated, site("Baalah/Kirjath-jearim")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.10",
            vec![
                travel(Unto, West, site("mount Seir")),
                travel(Unto, Unstated, part("mount Jearim", Part::NorthSide)),
                position(
                    reference("Chesalon", Part::Site),
                    Position::SameAs,
                    site("mount Jearim"),
                ),
                travel(Unto, Unstated, site("Beth-shemesh")),
                travel(Unto, Unstated, site("Timnah")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.11",
            vec![
                position(Subject::Border, Position::NorthOf, site("Ekron")),
                travel(Unto, Unstated, site("Shicron")),
                travel(Unto, Unstated, site("mount Baalah")),
                travel(Unto, Unstated, site("Jabneel")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            Judah,
            "JOS.15.12",
            vec![Relation::Boundary {
                target: site("Great Sea"),
            }],
        ),
        cited(
            Ephraim,
            "JOS.16.1",
            vec![
                travel(From, Unstated, site("Jordan (JOS.16.1)")),
                travel(Unto, East, site("Jericho water")),
                travel(Along, Unstated, site("JOS.16.1:wilderness")),
                travel(Unto, Unstated, site("mount Bethel")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.2",
            vec![
                travel(From, Unstated, site("Bethel")),
                travel(Unto, Unstated, site("Luz")),
                travel(Along, Unstated, part("Archi", Part::Border)),
                travel(Unto, Unstated, site("Ataroth")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.3",
            vec![
                travel(Unto, West, part("Japhleti", Part::Border)),
                travel(Unto, Unstated, part("nether Beth-horon", Part::Border)),
                travel(Unto, Unstated, site("Gezer")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.5",
            vec![
                travel(At, Unstated, site("Ataroth-addar")),
                travel(Unto, Unstated, site("upper Beth-horon")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.6",
            vec![
                travel(Unto, SeaWard, part("Michmethah", Part::NorthSide)),
                travel(Unto, East, site("Taanath-shiloh")),
                travel(Unto, Unstated, site("Janohah")),
                position(Subject::Border, Position::EastOf, site("Taanath-shiloh")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.7",
            vec![
                travel(From, Unstated, site("Janohah")),
                travel(Unto, Unstated, site("Ataroth")),
                travel(Unto, Unstated, site("Naarath")),
                travel(Unto, Unstated, site("Jericho")),
                travel(Outgoing, Unstated, site("Jordan")),
            ],
        ),
        cited(
            Ephraim,
            "JOS.16.8",
            vec![
                travel(From, Unstated, site("Tappuah")),
                travel(Unto, West, site("river Kanah")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            ManassehWest,
            "JOS.17.7",
            vec![
                travel(From, Unstated, site("Asher (JOS.17.7)")),
                travel(Unto, Unstated, site("Michmethah")),
                position(
                    reference("Michmethah", Part::Site),
                    Position::Before,
                    site("Shechem"),
                ),
                mention("Shechem"),
                travel(Unto, RightHand, site("En-tappuah")),
            ],
        ),
        cited(
            ManassehWest,
            "JOS.17.9",
            vec![
                travel(Unto, Unstated, site("river Kanah")),
                position(Subject::Border, Position::NorthOf, site("river Kanah")),
                travel(Outgoing, Unstated, site("Great Sea")),
            ],
        ),
        cited(
            ManassehWest,
            "JOS.17.10",
            vec![
                neighbour(ManassehWest, super::super::Side::North, "Asher"),
                neighbour(ManassehWest, super::super::Side::East, "Issachar"),
            ],
        ),
        cited(
            ManassehWest,
            "JOS.17.11",
            cities(&["Beth-shean", "Ibleam", "Dor", "Endor", "Taanach", "Megiddo"]),
        ),
        cited(
            Benjamin,
            "JOS.18.12",
            vec![
                travel(From, Unstated, site("Jordan")),
                position(Subject::Border, Position::NorthOf, site("Jericho")),
                travel(Along, West, site("JOS.18.12:mountains")),
                travel(Outgoing, Unstated, site("Beth-aven wilderness")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.13",
            vec![
                position(Subject::Border, Position::SouthOf, site("Luz/Bethel")),
                travel(Unto, Unstated, site("Ataroth-adar")),
                position(
                    reference("Ataroth-adar", Part::Site),
                    Position::Near,
                    site("JOS.18.13:hill"),
                ),
                position(
                    reference("JOS.18.13:hill", Part::Site),
                    Position::SouthOf,
                    site("nether Beth-horon"),
                ),
                mention("nether Beth-horon"),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.14",
            vec![
                travel(From, Unstated, site("JOS.18.14:hill")),
                position(
                    reference("JOS.18.14:hill", Part::Site),
                    Position::Before,
                    site("nether Beth-horon"),
                ),
                position(
                    reference("JOS.18.14:hill", Part::Site),
                    Position::SouthOf,
                    site("nether Beth-horon"),
                ),
                mention("nether Beth-horon"),
                travel(Outgoing, Unstated, site("Kirjath-baal/Kirjath-jearim")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.15",
            vec![
                travel(From, Unstated, part("Kirjath-jearim", Part::End)),
                travel(Unto, West, site("Nephtoah waters")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.16",
            vec![
                travel(Unto, Unstated, part("JOS.18.16:mountain", Part::End)),
                position(
                    reference("JOS.18.16:mountain", Part::Site),
                    Position::Before,
                    site("valley of Hinnom"),
                ),
                position(
                    reference("JOS.18.16:mountain", Part::Site),
                    Position::Within,
                    part("Rephaim valley", Part::NorthSide),
                ),
                mention("Rephaim valley"),
                travel(Along, Unstated, site("valley of Hinnom")),
                position(Subject::Border, Position::SouthOf, site("Jerusalem")),
                travel(Unto, Unstated, site("En-rogel")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.17",
            vec![
                travel(Unto, Unstated, site("En-shemesh")),
                travel(Toward, Unstated, site("Geliloth")),
                position(
                    reference("Geliloth", Part::Site),
                    Position::OverAgainst,
                    part("Adummim", Part::Ascent),
                ),
                mention("Adummim"),
                travel(Unto, Unstated, site("Bohan stone")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.18",
            vec![
                position(
                    Subject::Border,
                    Position::OverAgainst,
                    part("Arabah", Part::NorthSide),
                ),
                travel(Unto, Unstated, site("Arabah")),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.19",
            vec![
                position(Subject::Border, Position::NorthOf, site("Beth-hoglah")),
                travel(Outgoing, Unstated, site("JOS.18.19:sea-bay")),
                position(
                    reference("JOS.18.19:sea-bay", Part::Site),
                    Position::At,
                    part("JOS.18.19:Jordan-end", Part::End),
                ),
                mention("JOS.18.19:Jordan-end"),
            ],
        ),
        cited(
            Benjamin,
            "JOS.18.20",
            vec![Relation::Boundary {
                target: site("Jordan"),
            }],
        ),
        cited(
            Simeon,
            "JOS.19.2",
            cities(&["Beer-sheba", "Sheba", "Moladah"]),
        ),
        cited(
            Simeon,
            "JOS.19.3",
            cities(&["Hazar-shual", "Balah", "Azem"]),
        ),
        cited(Simeon, "JOS.19.4", cities(&["Eltolad", "Bethul", "Hormah"])),
        cited(
            Simeon,
            "JOS.19.5",
            cities(&["Ziklag", "Beth-marcaboth", "Hazar-susah"]),
        ),
        cited(Simeon, "JOS.19.6", cities(&["Beth-lebaoth", "Sharuhen"])),
        cited(
            Simeon,
            "JOS.19.7",
            cities(&["Ain", "Remmon", "Ether", "Ashan"]),
        ),
        cited(
            Simeon,
            "JOS.19.8",
            cities(&["Baalath-beer", "Ramath of the south"]),
        ),
        cited(
            Zebulun,
            "JOS.19.10",
            vec![travel(Unto, Unstated, site("Sarid"))],
        ),
        cited(
            Zebulun,
            "JOS.19.11",
            vec![
                travel(Unto, SeaWard, site("Maralah")),
                travel(Unto, Unstated, site("Dabbasheth")),
                travel(Unto, Unstated, site("JOS.19.11:river")),
                position(
                    reference("JOS.19.11:river", Part::Site),
                    Position::Before,
                    site("Jokneam"),
                ),
                mention("Jokneam"),
            ],
        ),
        cited(
            Zebulun,
            "JOS.19.12",
            vec![
                travel(From, Unstated, site("Sarid")),
                travel(Unto, East, part("Chisloth-tabor", Part::Border)),
                travel(Unto, Unstated, site("Daberath")),
                travel(Unto, Unstated, site("Japhia")),
            ],
        ),
        cited(
            Zebulun,
            "JOS.19.13",
            vec![
                travel(Unto, East, site("Gittah-hepher")),
                travel(Unto, Unstated, site("Ittah-kazin")),
                travel(Unto, Unstated, site("Remmon-methoar")),
                travel(Unto, Unstated, site("Neah")),
            ],
        ),
        cited(
            Zebulun,
            "JOS.19.14",
            vec![
                travel(Unto, North, site("Hannathon")),
                travel(Outgoing, Unstated, site("valley of Jiphthah-el")),
            ],
        ),
        cited(
            Zebulun,
            "JOS.19.15",
            cities(&["Kattath", "Nahallal", "Shimron", "Idalah", "Beth-lehem"]),
        ),
        cited(
            Issachar,
            "JOS.19.18",
            cities(&["Jezreel", "Chesulloth", "Shunem"]),
        ),
        cited(
            Issachar,
            "JOS.19.19",
            cities(&["Hapharaim", "Shion", "Anaharath"]),
        ),
        cited(
            Issachar,
            "JOS.19.20",
            cities(&["Rabbith", "Kishion", "Abez"]),
        ),
        cited(
            Issachar,
            "JOS.19.21",
            cities(&["Remeth", "En-gannim", "En-haddah", "Beth-pazzez"]),
        ),
        cited(
            Issachar,
            "JOS.19.22",
            vec![
                travel(Unto, Unstated, site("Tabor")),
                travel(Unto, Unstated, site("Shahazimah")),
                travel(Unto, Unstated, site("Beth-shemesh")),
                travel(Outgoing, Unstated, site("Jordan")),
            ],
        ),
        cited(
            Asher,
            "JOS.19.25",
            cities(&["Helkath", "Hali", "Beten", "Achshaph"]),
        ),
        cited(
            Asher,
            "JOS.19.26",
            vec![
                listed("Alammelech"),
                listed("Amad"),
                listed("Misheal"),
                travel(Unto, West, site("Carmel")),
                travel(Unto, Unstated, site("Shihor-libnath")),
            ],
        ),
        cited(
            Asher,
            "JOS.19.27",
            vec![
                travel(Unto, East, site("Beth-dagon")),
                travel(Unto, Unstated, site("Zebulun")),
                travel(Unto, Unstated, site("valley of Jiphthah-el")),
                travel(Unto, Unstated, part("Beth-emek", Part::NorthSide)),
                travel(Unto, Unstated, site("Neiel")),
                travel(Outgoing, LeftHand, site("Cabul")),
            ],
        ),
        cited(
            Asher,
            "JOS.19.28",
            vec![
                listed("Hebron Ebron"),
                listed("Rehob"),
                listed("Hammon"),
                listed("Kanah"),
                travel(Unto, Unstated, site("great Zidon")),
            ],
        ),
        cited(
            Asher,
            "JOS.19.29",
            vec![
                travel(Unto, Unstated, site("Ramah")),
                travel(Unto, Unstated, site("Tyre")),
                travel(Unto, Unstated, site("Hosah")),
                travel(Outgoing, Unstated, site("Great Sea")),
                travel(From, Unstated, part("Achzib", Part::Border)),
            ],
        ),
        cited(Asher, "JOS.19.30", cities(&["Ummah", "Aphek", "Rehob"])),
        cited(
            Naphtali,
            "JOS.19.33",
            vec![
                travel(From, Unstated, site("Heleph")),
                travel(From, Unstated, site("Allon")),
                travel(Unto, Unstated, site("Zaanannim")),
                travel(Unto, Unstated, site("Adami")),
                travel(Unto, Unstated, site("Nekeb")),
                travel(Unto, Unstated, site("Jabneel")),
                travel(Unto, Unstated, site("Lakum")),
                travel(Outgoing, Unstated, site("Jordan")),
            ],
        ),
        cited(
            Naphtali,
            "JOS.19.34",
            vec![
                travel(Unto, West, site("Aznoth-tabor")),
                travel(Unto, Unstated, site("Hukkok")),
                neighbour(Naphtali, super::super::Side::South, "Zebulun"),
                neighbour(Naphtali, super::super::Side::West, "Asher"),
                neighbour(Naphtali, super::super::Side::East, "Judah/Jordan"),
            ],
        ),
        cited(
            Naphtali,
            "JOS.19.35",
            cities(&["Ziddim", "Zer", "Hammath", "Rakkath", "Chinnereth"]),
        ),
        cited(Naphtali, "JOS.19.36", cities(&["Adamah", "Ramah", "Hazor"])),
        cited(
            Naphtali,
            "JOS.19.37",
            cities(&["Kedesh", "Edrei", "En-hazor"]),
        ),
        cited(
            Naphtali,
            "JOS.19.38",
            cities(&["Iron", "Migdal-el", "Horem", "Beth-anath", "Beth-shemesh"]),
        ),
        cited(
            Dan,
            "JOS.19.41",
            cities(&["Zorah", "Eshtaol", "Ir-shemesh"]),
        ),
        cited(
            Dan,
            "JOS.19.42",
            cities(&["Shaalabbin", "Ajalon", "Jethlah"]),
        ),
        cited(Dan, "JOS.19.43", cities(&["Elon", "Thimnathah", "Ekron"])),
        cited(
            Dan,
            "JOS.19.44",
            cities(&["Eltekeh", "Gibbethon", "Baalath"]),
        ),
        cited(
            Dan,
            "JOS.19.45",
            cities(&["Jehud", "Bene-berak", "Gath-rimmon"]),
        ),
        cited(
            Dan,
            "JOS.19.46",
            vec![
                listed("Me-jarkon"),
                listed("Rakkon"),
                position(Subject::Border, Position::Before, site("Japho")),
            ],
        ),
    ];
    let mut outcomes: Vec<_> = groups.into_iter().flatten().collect();
    outcomes.sort();
    outcomes
}

fn cited(lot: Lot, verse: &str, relations: Vec<Relation>) -> Vec<CitedRelation> {
    relations
        .into_iter()
        .map(|relation| CitedRelation {
            lot: lot.clone(),
            verse: verse.to_owned(),
            relation,
        })
        .collect()
}

fn travel(verb: Travel, direction: Direction, target: Target) -> Relation {
    Relation::Travel {
        verb,
        direction,
        target,
    }
}

fn position(subject: Subject, predicate: Position, target: Target) -> Relation {
    Relation::Position {
        subject,
        predicate,
        target,
    }
}

fn reference(name: &str, part: Part) -> Subject {
    Subject::Reference {
        reference: name.to_owned(),
        part,
    }
}

fn extent(verb: Extent, target: Target) -> Relation {
    Relation::Extent { verb, target }
}

fn neighbour(subject: Lot, side: super::super::Side, name: &str) -> Relation {
    Relation::Neighbour {
        subject,
        side,
        target: site(name),
    }
}

fn cities(names: &[&str]) -> Vec<Relation> {
    names.iter().map(|name| listed(name)).collect()
}

fn listed(name: &str) -> Relation {
    Relation::Listed { target: site(name) }
}

fn mention(name: &str) -> Relation {
    Relation::Reference { target: site(name) }
}

fn site(name: &str) -> Target {
    part(name, Part::Site)
}

fn part(name: &str, part: Part) -> Target {
    Target {
        reference: name.to_owned(),
        part,
    }
}
