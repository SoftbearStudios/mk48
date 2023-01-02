use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

fn main() {
    let json = include_str!("../../entities-raw.json");

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Entity {
        label: String,
        link: Option<String>,
        kind: String,
        subkind: String,
        #[serde(default)]
        position_forward: f32,
        #[serde(default)]
        position_side: f32,
        level: Option<usize>,
        reload: Option<usize>,
        length: f32,
        width: f32,
        draft: Option<f32>,
        depth: Option<f32>,
        speed: Option<f32>,
        range: Option<f32>,
        lifespan: Option<usize>,
        stealth: Option<f32>,
        damage: Option<f32>,
        ram_damage: Option<f32>,
        torpedo_resistance: Option<f32>,
        #[serde(default)]
        sensors: HashMap<String, Sensor>,
        #[serde(default)]
        armaments: Vec<Armament>,
        #[serde(default)]
        turrets: Vec<Turret>,
        #[serde(default)]
        exhausts: Vec<Exhaust>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sensor {
        #[serde(default)]
        range: f32,
    }

    fn one() -> usize {
        1
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Armament {
        #[serde(rename = "type")]
        _type: String,
        #[serde(default)]
        position_forward: f32,
        #[serde(default)]
        position_side: f32,
        angle: Option<f32>,
        #[serde(default)]
        symmetrical: bool,
        turret: Option<usize>,
        #[serde(default = "one")]
        count: usize,
        #[serde(default)]
        hidden: bool,
        #[serde(default)]
        external: bool,
        #[serde(default)]
        vertical: bool,

        #[serde(rename = "kind", default)]
        _kind: String,
        #[serde(rename = "subkind", default)]
        _sub_kind: String,
        #[serde(rename = "width", default)]
        _width: f32,
        #[serde(rename = "length", default)]
        _length: f32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Turret {
        #[serde(rename = "type")]
        _type: Option<String>,
        #[serde(default)]
        position_forward: f32,
        #[serde(default)]
        position_side: f32,
        speed: String,
        angle: Option<f32>,
        azimuth: Option<f32>,
        azimuth_b: Option<f32>,
        #[serde(rename = "azimuthBR")]
        azimuth_br: Option<f32>,
        #[serde(rename = "azimuthBL")]
        azimuth_bl: Option<f32>,
        azimuth_f: Option<f32>,
        #[serde(rename = "azimuthFR")]
        azimuth_fr: Option<f32>,
        #[serde(rename = "azimuthFL")]
        azimuth_fl: Option<f32>,
        #[serde(default)]
        symmetrical: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Exhaust {
        #[serde(default)]
        position_forward: f32,
        #[serde(default)]
        position_side: f32,
        #[serde(default)]
        symmetrical: bool,
    }

    let parsed: Result<HashMap<String, Entity>, _> = serde_json::from_str(json);

    let data = match parsed {
        Ok(data) => {
            //println!("{data:?}");
            //let pretty = serde_json::to_string_pretty(&data).unwrap();
            //println!("{pretty}");
            data
        }
        Err(e) => {
            panic!("parse error: {e}");
        }
    };

    let mut kvs = data.into_iter().collect::<Vec<_>>();
    kvs.sort_by_key(|(k, v)| format!("{}-{}", &v.kind, k));

    fn opt<D: Display>(name: &str, opt: Option<D>) -> Option<String> {
        opt.map(|f| format!("{name} = {f}"))
    }

    println!("pub enum EntityType {{");
    for (name, entity) in kvs {
        println!(
            "    #[info(label = \"{}\"{})]",
            entity.label,
            entity
                .link
                .map(|link| format!(", link = \"{link}\""))
                .unwrap_or_default()
        );
        println!(
            "    #[entity({}, {}{})]",
            name_to_ident(entity.kind),
            name_to_ident(entity.subkind),
            entity
                .level
                .map(|l| format!(", level = {l}"))
                .unwrap_or_default()
        );
        println!(
            "    #[size(length = {}, width = {}{})]",
            entity.length,
            entity.width,
            entity
                .draft
                .map(|d| format!(", draft = {d}"))
                .unwrap_or_default()
        );
        let offsets = opt(
            "forward",
            (entity.position_forward != 0.0).then_some(entity.position_forward),
        )
        .into_iter()
        .chain(opt(
            "side",
            (entity.position_side != 0.0).then_some(entity.position_side),
        ))
        .collect::<Vec<String>>()
        .join(", ");
        if !offsets.is_empty() {
            println!("    #[offset({offsets})]");
        }
        let props = opt("speed", entity.speed)
            .into_iter()
            .chain(opt("range", entity.range))
            .chain(opt("depth", entity.depth))
            .chain(opt("reload", entity.reload))
            .chain(opt("lifespan", entity.lifespan))
            .chain(opt("stealth", entity.stealth))
            .chain(opt("damage", entity.damage))
            .chain(opt("ram_damage", entity.ram_damage))
            .chain(opt("torpedo_resistance", entity.torpedo_resistance))
            .collect::<Vec<String>>()
            .join(", ");
        if !props.is_empty() {
            println!("    #[props({props})]");
        }
        if !entity.sensors.is_empty() {
            let mut sensors = entity.sensors.into_iter().collect::<Vec<_>>();
            sensors.sort_by_key(|(k, _)| k.to_owned());

            print!("    #[sensors(");
            for (i, (sensor, _)) in sensors.into_iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{sensor}");
            }
            println!(")]");
        }
        for armament in entity.armaments {
            let a = std::iter::once(format!("{}", name_to_ident(armament._type)))
                .chain(opt(
                    "forward",
                    (armament.position_forward != 0.0).then_some(armament.position_forward),
                ))
                .chain(opt(
                    "side",
                    (armament.position_side != 0.0).then_some(armament.position_side),
                ))
                .chain(opt("angle", armament.angle))
                .chain(opt("turret", armament.turret))
                .chain(opt(
                    "count",
                    (armament.count != 1).then_some(armament.count),
                ))
                .chain(armament.symmetrical.then_some(String::from("symmetrical")))
                .chain(armament.hidden.then_some(String::from("hidden")))
                .chain(armament.external.then_some(String::from("external")))
                .chain(armament.vertical.then_some(String::from("vertical")))
                .collect::<Vec<String>>()
                .join(", ");
            println!("    #[armament({a})]");
        }
        for turret in entity.turrets {
            let t = turret._type.clone().map(name_to_ident)
                .into_iter()
                .chain(opt("forward", Some(turret.position_forward)))
                .chain(opt(
                    "side",
                    (turret.position_side != 0.0).then_some(turret.position_side),
                ))
                .chain(opt("angle", turret.angle))
                .chain(std::iter::once(format!("{}", turret.speed)))
                .chain(turret.symmetrical.then_some(String::from("symmetrical")))
                .chain(opt("azimuth", turret.azimuth))
                .chain(opt("azimuth_f", turret.azimuth_f))
                .chain(opt("azimuth_fl", turret.azimuth_fl))
                .chain(opt("azimuth_fr", turret.azimuth_fr))
                .chain(opt("azimuth_b", turret.azimuth_b))
                .chain(opt("azimuth_bl", turret.azimuth_bl))
                .chain(opt("azimuth_br", turret.azimuth_br))
                .collect::<Vec<String>>()
                .join(", ");
            println!("    #[turret({t})]");
        }
        for exhaust in entity.exhausts {
            let e = opt("forward", Some(exhaust.position_forward))
                .into_iter()
                .chain(opt(
                    "side",
                    (exhaust.position_side != 0.0).then_some(exhaust.position_side),
                ))
                .chain(exhaust.symmetrical.then_some(String::from("symmetrical")))
                .collect::<Vec<String>>()
                .join(", ");
            println!("    #[exhaust({e})]");
        }
        println!("    {},", name_to_ident(name));
    }
    println!("}}");
}

fn name_to_ident(mut name: String) -> String {
    use convert_case::Casing;
    name = name.to_case(convert_case::Case::UpperCamel);
    if name.chars().next().unwrap().is_digit(10) {
        name = format!("_{name}");
    }
    name
}
