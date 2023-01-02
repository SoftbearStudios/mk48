use common_util::angle::Angle;
use common_util::range::map_ranges;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use std::ops::Mul;
use std::str::FromStr;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Lit, Meta, MetaNameValue, NestedMeta};

pub(crate) fn derive_entity_type(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    assert_eq!(ident.to_string(), "EntityType");

    let Data::Enum(DataEnum { variants, .. }) = data else {
        panic!("expected an enum");
    };

    let ordered_entity_names = variants
        .iter()
        .map(|variant| variant.ident.to_string())
        .collect::<Vec<_>>();

    let mut entities = variants
        .into_iter()
        .map(|variant| {
            let mut entity = Entity::default();

            for attr in variant.attrs
            /* TODO filter */
            {
                let meta = attr.parse_meta().expect("couldn't parse as meta");
                let list = match meta {
                    Meta::List(list) => list,
                    Meta::Path(_) => panic!("unexpected top-level path"),
                    Meta::NameValue(_) => panic!("unexpected top-level name-value pair"),
                };

                fn set_string(string: &mut Option<String>, meta: Meta) {
                    let path = meta.path().get_ident().unwrap().to_string();
                    let Meta::NameValue(MetaNameValue{ lit, .. }) = meta else {
                    panic!("expected name value for {path}")
                };
                    let Lit::Str(str_lit) = lit else {
                    panic!("expected string literal {path}");
                };
                    if string.is_some() {
                        panic!("duplicate key for {path}");
                    }
                    *string = Some(str_lit.value());
                }

                fn set_usize(int: &mut Option<usize>, meta: Meta) {
                    let path = meta.path().get_ident().unwrap().to_string();
                    let Meta::NameValue(MetaNameValue{ lit, .. }) = meta else {
                    panic!("expected name value for {path}")
                };
                    let Lit::Int(int_lit) = lit else {
                    panic!("expected int literal {path}");
                };
                    if int.is_some() {
                        panic!("duplicate key for {path}");
                    }
                    *int = Some(
                        usize::from_str(int_lit.base10_digits())
                            .expect(&format!("invalid usize {}", int_lit.base10_digits())),
                    );
                }

                fn set_f32(float: &mut Option<f32>, meta: Meta) {
                    let path = meta.path().get_ident().unwrap().to_string();
                    let Meta::NameValue(MetaNameValue{ lit, .. }) = meta else {
                        panic!("expected name value for {path}")
                    };
                    let value = match lit {
                        Lit::Int(int_lit) => i64::from_str(int_lit.base10_digits())
                            .expect(&format!("invalid i64 {}", int_lit.base10_digits()))
                            as f32,
                        Lit::Float(float_lit) => {
                            float_lit.base10_parse::<f32>().expect("expected valid f32")
                        }
                        _ => panic!("expected numeric literal {path}"),
                    };
                    if float.is_some() {
                        panic!("duplicate key for {path}");
                    }
                    *float = Some(value);
                }

                fn set_angle(angle: &mut Option<Angle>, meta: Meta) {
                    let path = meta.path().get_ident().unwrap().to_string();
                    let Meta::NameValue(MetaNameValue{ lit, .. }) = meta else {
                        panic!("expected name value for {path}")
                    };
                    let value = match lit {
                        Lit::Int(int_lit) => i64::from_str(int_lit.base10_digits())
                            .expect(&format!("invalid i64 {}", int_lit.base10_digits()))
                            as f32,
                        Lit::Float(float_lit) => {
                            float_lit.base10_parse::<f32>().expect("expected valid f32")
                        }
                        _ => panic!("expected numeric literal {path}"),
                    };
                    if angle.is_some() {
                        panic!("duplicate key for {path}");
                    }
                    *angle = Some(Angle::from_degrees(value));
                }

                fn set_bool(boolean: &mut bool, meta: Meta) {
                    let path = meta.path().get_ident().unwrap().to_string();
                    let Meta::Path(_) = meta else {
                    panic!("expected simple path for {path}")
                };
                    if *boolean {
                        panic!("duplicate key for {path}");
                    }
                    *boolean = true;
                }

                let path = list.path.get_ident().unwrap().to_string();
                match path.as_str() {
                    "info" => {
                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                                panic!("expected nested meta, found {:?}", nested);
                            };

                            let path = nested.path().get_ident().unwrap().to_string();

                            set_string(
                                match path.as_str() {
                                    "name" => &mut entity.name,
                                    "label" => &mut entity.label,
                                    "link" => &mut entity.link,
                                    _ => panic!("unexpected info path: {path}"),
                                },
                                nested,
                            );
                        }
                    }
                    "entity" => {
                        for (i, nested) in list.nested.into_iter().enumerate() {
                            let NestedMeta::Meta(nested) = nested else {
                                panic!("expected nested meta");
                            };

                            let path = nested.path().get_ident().unwrap().to_string();

                            match i {
                                0 => {
                                    entity.kind = Some(path);
                                }
                                1 => {
                                    entity.sub_kind = Some(path);
                                }
                                _ => match path.as_str() {
                                    "level" => {
                                        set_usize(&mut entity.level, nested);
                                    }
                                    _ => panic!("unexpected entity path: {path}"),
                                },
                            }
                        }
                    }
                    "size" => {
                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta, found {:?}", nested);
                        };

                            let path = nested.path().get_ident().unwrap().to_string();

                            set_f32(
                                match path.as_str() {
                                    "length" => &mut entity.length,
                                    "width" => &mut entity.width,
                                    "draft" => &mut entity.draft,
                                    "mast" => &mut entity.mast,
                                    _ => panic!("unexpected size path: {path}"),
                                },
                                nested,
                            );
                        }
                    }
                    "offset" => {
                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta");
                        };

                            let path = nested.path().get_ident().unwrap().to_string();

                            set_f32(
                                match path.as_str() {
                                    "forward" => &mut entity.position_forward,
                                    "side" => &mut entity.position_side,
                                    _ => panic!("unexpected offset path: {path}"),
                                },
                                nested,
                            );
                        }
                    }
                    "props" => {
                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta");
                        };

                            let path = nested.path().get_ident().unwrap().to_string();

                            match path.as_str() {
                                "reload" => {
                                    set_f32(&mut entity.reload, nested);
                                }
                                "depth" => {
                                    set_f32(&mut entity.depth, nested);
                                }
                                "speed" => {
                                    set_f32(&mut entity.speed, nested);
                                }
                                "range" => {
                                    set_f32(&mut entity.range, nested);
                                }
                                "lifespan" => {
                                    set_f32(&mut entity.lifespan, nested);
                                }
                                "stealth" => {
                                    set_f32(&mut entity.stealth, nested);
                                }
                                "damage" => {
                                    set_f32(&mut entity.damage, nested);
                                }
                                "ram_damage" => {
                                    set_f32(&mut entity.ram_damage, nested);
                                }
                                "torpedo_resistance" => {
                                    set_f32(&mut entity.torpedo_resistance, nested);
                                }
                                _ => panic!("unexpected props path: {path}"),
                            }
                        }
                    }
                    "sensors" => {
                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta");
                        };

                            let mut sensor = Sensor::default();

                            let path = match nested.clone() {
                                Meta::Path(path) => path,
                                Meta::NameValue(MetaNameValue { path, .. }) => {
                                    set_f32(&mut sensor.range, nested);
                                    path
                                }
                                Meta::List(_) => panic!("unexpected sensors list"),
                            }
                            .get_ident()
                            .unwrap()
                            .to_string();

                            assert!(
                                entity.sensors.insert(path.clone(), sensor).is_none(),
                                "duplicate sensor {path}"
                            );
                        }
                    }
                    "armament" => {
                        let mut armament = Armament::default();

                        for (i, nested) in list.nested.into_iter().enumerate() {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta");
                        };

                            let path = nested.path().get_ident().unwrap().to_string();

                            match i {
                                0 => {
                                    armament._type = Some(path);
                                }
                                _ => match path.as_str() {
                                    "forward" => {
                                        set_f32(&mut armament.position_forward, nested);
                                    }
                                    "side" => {
                                        set_f32(&mut armament.position_side, nested);
                                    }
                                    "angle" => {
                                        set_angle(&mut armament.angle, nested);
                                    }
                                    "symmetrical" => {
                                        set_bool(&mut armament.symmetrical, nested);
                                    }
                                    "turret" => {
                                        set_usize(&mut armament.turret, nested);
                                    }
                                    "count" => {
                                        set_usize(&mut armament.count, nested);
                                    }
                                    "hidden" => {
                                        set_bool(&mut armament.hidden, nested);
                                    }
                                    "external" => {
                                        set_bool(&mut armament.external, nested);
                                    }
                                    "vertical" => {
                                        set_bool(&mut armament.vertical, nested);
                                    }
                                    _ => panic!("unexpected armament path: {path}"),
                                },
                            }
                        }

                        entity.armaments.push(armament);
                    }
                    "turret" => {
                        let mut turret = Turret::default();
                        let mut speed = None;

                        for (i, nested) in list.nested.into_iter().enumerate() {
                            let NestedMeta::Meta(nested) = nested else {
                                panic!("expected nested meta");
                            };

                            let path = nested.path().get_ident().unwrap().to_string();

                            if matches!(nested, Meta::Path(_)) && path != "symmetrical" {
                                if matches!(path.as_str(), "slow" | "medium" | "fast") {
                                    if speed.is_some() {
                                        panic!("duplicate turret speed");
                                    }
                                    speed = Some(path);
                                } else if i == 0 {
                                    turret._type = Some(path);
                                } else {
                                    panic!("unexpected turret path {path}");
                                }
                            } else {
                                match path.as_str() {
                                    "forward" => {
                                        set_f32(&mut turret.position_forward, nested);
                                    }
                                    "side" => {
                                        set_f32(&mut turret.position_side, nested);
                                    }
                                    "angle" => {
                                        set_angle(&mut turret.angle, nested);
                                    }
                                    "azimuth" => {
                                        set_angle(&mut turret.azimuth, nested);
                                    }
                                    "azimuth_b" => {
                                        set_angle(&mut turret.azimuth_b, nested);
                                    }
                                    "azimuth_bl" => {
                                        set_angle(&mut turret.azimuth_bl, nested);
                                    }
                                    "azimuth_br" => {
                                        set_angle(&mut turret.azimuth_br, nested);
                                    }
                                    "azimuth_f" => {
                                        set_angle(&mut turret.azimuth_f, nested);
                                    }
                                    "azimuth_fl" => {
                                        set_angle(&mut turret.azimuth_fl, nested);
                                    }
                                    "azimuth_fr" => {
                                        set_angle(&mut turret.azimuth_fr, nested);
                                    }
                                    "symmetrical" => {
                                        set_bool(&mut turret.symmetrical, nested);
                                    }
                                    _ => panic!("unexpected turret path: {path}"),
                                }
                            }
                        }

                        turret.speed = Some(match speed.as_deref() {
                            Some("slow") => Angle::PI * 0.3,
                            Some("fast") => Angle::PI * 0.6,
                            _ => Angle::PI * 0.45,
                        });

                        entity.turrets.push(turret);
                    }
                    "exhaust" => {
                        let mut exhaust = Exhaust::default();

                        for nested in list.nested {
                            let NestedMeta::Meta(nested) = nested else {
                            panic!("expected nested meta");
                        };

                            let path = nested.path().get_ident().unwrap().to_string();

                            match path.as_str() {
                                "forward" => {
                                    set_f32(&mut exhaust.position_forward, nested);
                                }
                                "side" => {
                                    set_f32(&mut exhaust.position_side, nested);
                                }
                                "symmetrical" => {
                                    set_bool(&mut exhaust.symmetrical, nested);
                                }
                                _ => panic!("unexpected exhaust path: {path}"),
                            }
                        }

                        entity.exhausts.push(exhaust);
                    }
                    _ => panic!("unexpected path {path}"),
                }
            }

            (variant.ident.to_string(), entity)
        })
        .collect::<HashMap<_, _>>();

    //panic!("{entities:?}");

    let original_entities = entities.clone();
    let mut max_radius = 0f32;
    let mut max_boat_level = 0;

    for (variant, entity) in &mut entities {
        if entity.speed.is_some() {
            match entity.kind() {
                "Weapon" => match entity.sub_kind() {
                    "Shell" => {
                        *entity.speed.as_mut().unwrap() *= 0.75;
                    }
                    _ => {}
                },
                "Aircraft" => {
                    *entity.speed.as_mut().unwrap() = entity.speed.unwrap().min(140.0);
                }
                _ => {}
            }
            *entity.speed.as_mut().unwrap() = entity.speed.unwrap().min(1000.0);
        }

        if entity.range.is_some() && variant != "Depositor" {
            let mut max_range = 1500.0;
            let mut avg_speed = entity.speed.unwrap();

            match entity.kind() {
                "Weapon" => {
                    match entity.sub_kind() {
                        "Sam" => {
                            max_range *= 0.5;
                        }
                        "RocketTorpedo" => {
                            entity
                                .sensors
                                .insert(String::from("sonar"), Sensor { range: None });
                        }
                        _ => {}
                    }
                    match entity.sub_kind() {
                        "Shell" => {
                            max_range = map_ranges(entity.length(), 0.2..2.0, 250.0..850.0, true);
                        }
                        "Sam" | "Rocket" | "RocketTorpedo" | "Missile" => {
                            max_range = map_ranges(entity.length(), 1.0..10.0, 500.0..1200.0, true);

                            avg_speed = 0.0;
                            let mut count = 0;
                            let mut speed = 0.0;
                            let seconds = 0.1;
                            let mut d = 0.0;
                            while d < max_range {
                                let delta = entity.speed.unwrap() - speed;
                                speed += delta.min(800.0 * seconds) * seconds;
                                avg_speed += speed;
                                count += 1;
                                d += speed * seconds;
                            }
                            avg_speed /= count as f32;
                        }
                        _ => {}
                    }
                }
                "Aircraft" => {
                    max_range = 5000.0;
                }
                _ => {}
            }
            entity.range = Some(entity.range.unwrap().min(max_range));
            let range_lifespan = 0.1f32.max(entity.range.unwrap() / avg_speed);
            if entity
                .lifespan
                .map(|lifespan| lifespan > range_lifespan)
                .unwrap_or(true)
            {
                entity.lifespan = Some(range_lifespan);
            }
            // Done with this.
            entity.range = None;
        }

        match entity.kind() {
            "Aircraft" => {
                entity.limited = true;
            }
            "Boat" => {
                match entity.sub_kind() {
                    "Dredger" | "Submarine" | "Tanker" => {}
                    _ => {
                        entity.anti_aircraft =
                            map_ranges(entity.length(), 30.0..300.0, 0.1..0.5, true);
                    }
                }

                if entity.ram_damage.is_none() {
                    entity.ram_damage = Some(1.0);
                }

                if entity.torpedo_resistance.is_none() {
                    match entity.sub_kind() {
                        "Battleship" => {
                            entity.torpedo_resistance = Some(0.4);
                        }
                        "Cruiser" => {
                            entity.torpedo_resistance = Some(0.2);
                        }
                        _ => {}
                    }
                }

                if entity.sub_kind() == "Pirate" {
                    entity.npc = true;
                }
            }
            _ => {}
        }

        let mut damage = None;
        match entity.kind() {
            "Boat" => {
                // Damage means health (i.e. how much damage before death).
                let factor: f32 = 20.0 / 10.0 / 60.0;
                damage = Some(factor.max(factor * entity.length()));
            }
            "Weapon" => {
                // Damage means damage dealt.
                match entity.sub_kind() {
                    "Torpedo" => {
                        damage = Some(0.27 * entity.length().powf(0.7));
                        // Homing torpedoes do less damage.
                        /*
                        if entity.sensors.contains_key("sonar") {
                            *damage.as_mut().unwrap() -= 0.1;
                        }
                         */
                    }
                    "Mine" => {
                        damage = Some(1.5);
                    }
                    "DepthCharge" => {
                        damage = Some(0.7);
                    }
                    "Rocket" | "Missile" => {
                        damage = Some(0.19 * entity.length().powf(0.7));
                    }
                    "RocketTorpedo" => damage = Some(0.0),
                    "Shell" => {
                        let normal = 0.5 * entity.length().powf(0.35);
                        let special = 0.14 * entity.length().powi(3);
                        if entity.width() > 0.3 {
                            damage = Some(normal.max(special));
                        } else {
                            // Very long, small shells do not benefit from "special" damage calculation.
                            damage = Some(normal);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if let Some(damage) = damage {
            entity.damage = Some(if let Some(multiplier) = entity.damage {
                damage * multiplier
            } else {
                damage
            });
        } else {
            assert_eq!(
                entity.damage, None,
                "unexpected damage multiplier for {variant}"
            );
        }

        if entity.reload.is_none() {
            match entity.kind() {
                "Weapon" => {
                    entity.reload = Some(match entity.sub_kind() {
                        "Depositor" => 1.0,
                        "Rocket" => 2.5,
                        "RocketTorpedo" => 20.0,
                        "Mine" => 30.0,
                        "Sam" => 16.0,
                        "Missile" => map_ranges(entity.length(), 1.0..6.0, 4.0..12.0, true),
                        "Shell" => map_ranges(entity.length(), 0.25..2.0, 8.0..15.0, true),
                        "Torpedo" => {
                            let mut reload = 8.0;
                            if !entity.sensors.is_empty() {
                                // Homing torpedoes take longer to reload
                                reload *= 1.5;
                            }
                            reload
                        }
                        "DepthCharge" => 16.0,
                        _ => 8.0,
                    });
                }
                "Aircraft" => {
                    entity.reload = Some(10.0);
                }
                "Decoy" => {
                    entity.reload = Some(20.0);
                }
                _ => {}
            }
        }

        let mut armaments = std::mem::take(&mut entity.armaments);
        let turrets = std::mem::take(&mut entity.turrets);
        let exhausts = std::mem::take(&mut entity.exhausts);

        for mut turret in turrets {
            turret.angle = Some(turret.angle.unwrap_or_default());

            if let Some(azimuth) = turret.azimuth {
                turret.azimuth_b = Some(azimuth);
                turret.azimuth_f = Some(azimuth);
                turret.azimuth = None;
            }
            if let Some(azimuth_f) = turret.azimuth_f {
                turret.azimuth_fl = Some(azimuth_f);
                turret.azimuth_fr = Some(azimuth_f);
                turret.azimuth_f = None;
            }
            if let Some(azimuth_b) = turret.azimuth_b {
                turret.azimuth_bl = Some(azimuth_b);
                turret.azimuth_br = Some(azimuth_b);
                turret.azimuth_b = None;
            }

            let symmetrical = std::mem::take(&mut turret.symmetrical);
            entity.turrets.push(turret.clone());
            if symmetrical {
                entity.turrets.push(Turret {
                    angle: turret.angle.map(|a| -a),
                    azimuth_fl: turret.azimuth_fr,
                    azimuth_fr: turret.azimuth_fl,
                    azimuth_bl: turret.azimuth_br,
                    azimuth_br: turret.azimuth_bl,
                    position_side: turret.position_side.map(|p| -p),
                    ..turret
                });
            }
        }
        for (i, turret) in entity.turrets.iter().enumerate() {
            if let Some(_type) = turret._type.as_deref() {
                for armament in original_entities.get(_type).unwrap().armaments.clone() {
                    armaments.push(Armament {
                        turret: Some(i),
                        ..armament
                    });
                }
            }
        }

        for mut armament in armaments {
            let count = std::mem::take(&mut armament.count).unwrap_or(1);
            assert!(count > 0, "zero armament count");
            let symmetrical = std::mem::take(&mut armament.symmetrical);
            for _ in 0..count {
                entity.armaments.push(armament.clone());
                if symmetrical {
                    entity.armaments.push(Armament {
                        position_side: armament.position_side.map(|p| -p),
                        angle: armament.angle.map(|a| -a),
                        ..armament.clone()
                    });
                }
            }
        }

        for mut exhaust in exhausts {
            let symmetrical = std::mem::take(&mut exhaust.symmetrical);
            entity.exhausts.push(exhaust.clone());
            if symmetrical {
                entity.exhausts.push(Exhaust {
                    position_side: exhaust.position_side.map(|p| -p),
                    ..exhaust
                });
            }
        }

        let mut sensors = std::mem::take(&mut entity.sensors);
        for (typ, sensor) in &mut sensors {
            if sensor.range.is_none() {
                let (base, factor) = match typ.as_str() {
                    "visual" => (400.0, 3.0),
                    "radar" => {
                        if entity.kind() == "Boat" {
                            (1000.0, 1.5)
                        } else {
                            (500.0, 5.0)
                        }
                    }
                    "sonar" => {
                        if entity.sub_kind() == "Submarine" {
                            (500.0, 1.25)
                        } else if entity.sub_kind() == "RocketTorpedo" {
                            (125.0, 0.0)
                        } else if entity.kind() == "Weapon" {
                            (250.0, 5.0)
                        } else {
                            (350.0, 0.5)
                        }
                    }
                    _ => unreachable!("invalid sensor {typ}"),
                };

                sensor.range = Some(2000f32.min(base + factor * entity.length()));
            }
        }
        entity.sensors = sensors;

        let mut armaments = std::mem::take(&mut entity.armaments);
        armaments.sort_by_key(|armament| {
            let armament_data = original_entities.get(armament._type()).unwrap();
            -match (armament_data.kind(), armament_data.sub_kind()) {
                ("Weapon", "Torpedo") => 10,
                ("Weapon", "Missile") => 9,
                ("Weapon", "Rocket") | ("Weapon", "RocketTorpedo") => 8,
                ("Weapon", "Shell") => {
                    if matches!(entity.sub_kind(), "Battleship" | "Cruiser") {
                        12
                    } else {
                        5
                    }
                }
                ("Weapon", "DepthCharge") | ("Weapon", "Mine") => 1,
                ("Weapon", "Sam") => -5,
                ("Decoy", _) => -8,
                ("Aircraft", _) => {
                    if entity.sub_kind() == "Carrier" {
                        12
                    } else {
                        -10
                    }
                }
                _ => {
                    panic!(
                        "unexpected {}/{}",
                        armament_data.kind(),
                        armament_data.sub_kind()
                    );
                    //0
                }
            }
        });
        entity.armaments = armaments;

        entity.stealth = Some(entity.stealth.unwrap_or_default());
        entity.radius = glam::Vec2::new(entity.width(), entity.length())
            .mul(0.5)
            .length();
        entity.inv_size =
            1.0 / (entity.radius * (1.0 / 30.0) * (1.0 - entity.stealth.unwrap()).powi(2)).min(1.0);

        max_radius = max_radius.max(entity.radius);
        if entity.kind() == "Boat" {
            max_boat_level = max_boat_level.max(entity.level.unwrap() as u8);
        }
    }

    let entity_datas = ordered_entity_names
        .iter()
        .map(|s| entities.get(&*s).unwrap());

    let entity_type_as_strs: Vec<EntityTypeAsStr> = ordered_entity_names
        .iter()
        .map(|s| EntityTypeAsStr::new(s.to_string()))
        .collect();

    let entity_type_from_strs: Vec<EntityTypeFromStr> = ordered_entity_names
        .iter()
        .map(|s| EntityTypeFromStr::new(s.to_string()))
        .collect();

    let entity_type_from_u8s: Vec<EntityTypeFromU8> = ordered_entity_names
        .iter()
        .enumerate()
        .map(|(i, s)| {
            EntityTypeFromU8::new(
                s.to_string(),
                i.try_into()
                    .expect("u8 cannot fit more than 256 entity types"),
            )
        })
        .collect();

    quote! {
        impl EntityType {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#entity_type_as_strs),*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                Some(match s {
                    #(#entity_type_from_strs),*,
                    _ => return None
                })
            }

            pub fn from_u8(i: u8) -> Option<Self> {
                Some(match i {
                    #(#entity_type_from_u8s),*,
                    _ => return None
                })
            }

            const DATA: &[EntityData] = &[
                #(#entity_datas),*
            ];
        }

        impl std::fmt::Debug for EntityType {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::fmt::Display for EntityType {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl EntityData {
            pub const MAX_RADIUS: f32 = #max_radius;
            pub const MAX_BOAT_LEVEL: u8 = #max_boat_level;
        }
    }
    .into()
}

#[derive(Clone, Debug, Default)]
struct Entity {
    name: Option<String>,
    label: Option<String>,
    link: Option<String>,
    kind: Option<String>,
    sub_kind: Option<String>,
    position_forward: Option<f32>,
    position_side: Option<f32>,
    level: Option<usize>,
    length: Option<f32>,
    width: Option<f32>,
    draft: Option<f32>,
    mast: Option<f32>,
    reload: Option<f32>,
    depth: Option<f32>,
    speed: Option<f32>,
    range: Option<f32>,
    lifespan: Option<f32>,
    stealth: Option<f32>,
    damage: Option<f32>,
    ram_damage: Option<f32>,
    torpedo_resistance: Option<f32>,
    sensors: HashMap<String, Sensor>,
    armaments: Vec<Armament>,
    turrets: Vec<Turret>,
    exhausts: Vec<Exhaust>,
    limited: bool,
    npc: bool,
    anti_aircraft: f32,
    radius: f32,
    inv_size: f32,
}

impl Entity {
    fn kind(&self) -> &str {
        self.kind.as_deref().unwrap()
    }

    fn sub_kind(&self) -> &str {
        self.sub_kind.as_deref().unwrap()
    }

    fn length(&self) -> f32 {
        self.length.unwrap()
    }

    fn width(&self) -> f32 {
        self.width.unwrap()
    }
}

#[derive(Clone, Debug, Default)]
struct Sensor {
    range: Option<f32>,
}

#[derive(Clone, Debug, Default)]
struct Armament {
    _type: Option<String>,
    position_forward: Option<f32>,
    position_side: Option<f32>,
    angle: Option<Angle>,
    symmetrical: bool,
    turret: Option<usize>,
    count: Option<usize>,
    hidden: bool,
    external: bool,
    vertical: bool,
}

impl Armament {
    fn _type(&self) -> &str {
        self._type.as_deref().unwrap()
    }
}

#[derive(Clone, Debug, Default)]
struct Turret {
    _type: Option<String>,
    position_forward: Option<f32>,
    position_side: Option<f32>,
    speed: Option<Angle>,
    angle: Option<Angle>,
    azimuth: Option<Angle>,
    azimuth_b: Option<Angle>,
    azimuth_br: Option<Angle>,
    azimuth_bl: Option<Angle>,
    azimuth_f: Option<Angle>,
    azimuth_fr: Option<Angle>,
    azimuth_fl: Option<Angle>,
    symmetrical: bool,
}

#[derive(Clone, Debug, Default)]
struct Exhaust {
    position_forward: Option<f32>,
    position_side: Option<f32>,
    symmetrical: bool,
}

fn name_to_string(name: &str) -> &str {
    name.trim_start_matches('_')
}

struct EntityTypeAsStr(String);

impl EntityTypeAsStr {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

impl quote::ToTokens for EntityTypeAsStr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = name_to_string(&self.0);
        let ident = string_to_ident(&self.0);

        let ts: proc_macro2::TokenStream = {
            quote! {
               Self::#ident => #name
            }
        }
        .into();

        tokens.extend(ts);
    }
}

struct EntityTypeFromStr(String);

impl EntityTypeFromStr {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

impl quote::ToTokens for EntityTypeFromStr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = name_to_string(&self.0);
        let ident = string_to_ident(&self.0);

        let ts: proc_macro2::TokenStream = {
            quote! {
               #name => Self::#ident
            }
        }
        .into();

        tokens.extend(ts);
    }
}

struct EntityTypeFromU8(String, u8);

impl EntityTypeFromU8 {
    pub fn new(name: String, index: u8) -> Self {
        Self(name, index)
    }
}

impl quote::ToTokens for EntityTypeFromU8 {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = string_to_ident(&self.0);
        let index = self.1;

        let ts: proc_macro2::TokenStream = {
            quote! {
               #index => Self::#ident
            }
        }
        .into();

        tokens.extend(ts);
    }
}

impl quote::ToTokens for Entity {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let kind = string_to_ident(self.kind());
        let sub_kind = string_to_ident(self.sub_kind());
        let level = self.level.unwrap_or_default() as u8;
        let limited = self.limited;
        let npc = self.npc;
        let lifespan = (self.lifespan.unwrap_or_default() * 1000.0) as u32;
        let reload = (self.reload.unwrap_or_default() * 1000.0) as u32;
        let speed = (self.speed.unwrap_or_default() * 100.0) as u32;
        let length = self.length();
        let width = self.width();
        let draft = self.draft.unwrap_or_default() as i16;
        let mast = self.mast.unwrap_or_default() as i16;
        let depth = self.depth.unwrap_or_default() as i16;
        let radius = self.radius;
        let inv_size = self.inv_size;
        let damage = self.damage.unwrap_or_default();
        let anti_aircraft = self.anti_aircraft;
        let ram_damage = self.ram_damage.unwrap_or_default();
        let torpedo_resistance = self.torpedo_resistance.unwrap_or_default();
        let stealth = self.stealth.unwrap_or_default();

        let visual_range = self
            .sensors
            .get("visual")
            .map(|s| s.range.unwrap_or_default())
            .unwrap_or_default();
        let radar_range = self
            .sensors
            .get("radar")
            .map(|s| s.range.unwrap_or_default())
            .unwrap_or_default();
        let sonar_range = self
            .sensors
            .get("sonar")
            .map(|s| s.range.unwrap_or_default())
            .unwrap_or_default();

        let armaments = &self.armaments;
        let turrets = &self.turrets;
        let exhausts = &self.exhausts;

        let label = self.label.as_deref().unwrap();
        let link = quote_option(self.link.as_deref());
        let range = self.range.unwrap_or_default();
        let position_forward = self.position_forward.unwrap_or_default();
        let position_side = self.position_side.unwrap_or_default();

        let ts: proc_macro2::TokenStream = {
            quote! {
                EntityData{
                    kind: EntityKind::#kind,
                    sub_kind: EntitySubKind::#sub_kind,
                    level: #level,
                    limited: #limited,
                    npc: #npc,
                    lifespan: Ticks::from_whole_millis(#lifespan),
                    reload: Ticks::from_whole_millis(#reload),
                    speed: Velocity::from_whole_cmps(#speed),
                    length: #length,
                    width: #width,
                    draft: Altitude::from_whole_meters(#draft),
                    mast: Altitude::from_whole_meters(#mast),
                    depth: Altitude::from_whole_meters(#depth),
                    radius: #radius,
                    inv_size: #inv_size,
                    damage: #damage,
                    anti_aircraft: #anti_aircraft,
                    ram_damage: #ram_damage,
                    torpedo_resistance: #torpedo_resistance,
                    stealth: #stealth,
                    sensors: Sensors{
                        visual: Sensor{
                            range: #visual_range,
                        },
                        radar: Sensor{
                            range: #radar_range,
                        },
                        sonar: Sensor{
                            range: #sonar_range,
                        }
                    },
                    armaments: &[#(#armaments),*],
                    turrets: &[#(#turrets),*],
                    exhausts: &[#(#exhausts),*],
                    label: #label,
                    link: #link,
                    range: #range,
                    position_forward: #position_forward,
                    position_side: #position_side,
                }
            }
        }
        .into();

        tokens.extend(ts);
    }
}

impl quote::ToTokens for Armament {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let entity_type = string_to_ident(self._type());
        let hidden = self.hidden;
        let external = self.external;
        let vertical = self.vertical;
        let position_forward = self.position_forward.unwrap_or_default();
        let position_side = self.position_side.unwrap_or_default();
        let angle = self.angle.unwrap_or_default().0;
        let turret = quote_option(self.turret);

        let ts: proc_macro2::TokenStream = {
            quote! {
                Armament{
                    entity_type: EntityType::#entity_type,
                    hidden: #hidden,
                    external: #external,
                    vertical: #vertical,
                    position_forward: #position_forward,
                    position_side: #position_side,
                    angle: Angle(#angle),
                    turret: #turret,
                }
            }
        }
        .into();

        tokens.extend(ts);
    }
}

impl quote::ToTokens for Turret {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let entity_type = quote_option(self._type.as_deref().map(|t| {
            let ident = string_to_ident(t);
            quote! {
                Self::#ident
            }
        }));
        let position_forward = self.position_forward.unwrap_or_default();
        let position_side = self.position_side.unwrap_or_default();
        let angle = self.angle.unwrap_or_default().0;
        let speed = self.speed.unwrap_or_default().0;
        let azimuth_fl = self.azimuth_fl.unwrap_or_default().0;
        let azimuth_fr = self.azimuth_fr.unwrap_or_default().0;
        let azimuth_bl = self.azimuth_bl.unwrap_or_default().0;
        let azimuth_br = self.azimuth_br.unwrap_or_default().0;

        let ts: proc_macro2::TokenStream = {
            quote! {
                Turret{
                    entity_type: #entity_type,
                    position_forward: #position_forward,
                    position_side: #position_side,
                    angle: Angle(#angle),
                    speed: Angle(#speed),
                    azimuth_fl: Angle(#azimuth_fl),
                    azimuth_fr: Angle(#azimuth_fr),
                    azimuth_bl: Angle(#azimuth_bl),
                    azimuth_br: Angle(#azimuth_br),
                }
            }
        }
        .into();

        tokens.extend(ts);
    }
}

impl quote::ToTokens for Exhaust {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let position_forward = self.position_forward.unwrap_or_default();
        let position_side = self.position_side.unwrap_or_default();

        let ts: proc_macro2::TokenStream = {
            quote! {
                Exhaust{
                    position_forward: #position_forward,
                    position_side: #position_side,
                }
            }
        }
        .into();

        tokens.extend(ts);
    }
}

fn string_to_ident(string: &str) -> Ident {
    Ident::new(string, Span::call_site())
}

fn quote_option<TT: ToTokens>(opt: Option<TT>) -> proc_macro2::TokenStream {
    if let Some(some) = opt {
        quote! {
            Some(#some)
        }
    } else {
        quote! {
            None
        }
    }
}
