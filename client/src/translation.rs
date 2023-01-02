// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::{ACTIVE_KEY, SURFACE_KEY};
use common::death_reason::DeathReason;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use core_protocol::id::LanguageId;
use core_protocol::id::LanguageId::*;
use core_protocol::name::PlayerAlias;
use std::fmt::Display;
use yew_frontend::frontend::RewardedAd;
use yew_frontend::s;

pub trait Mk48Translation: Sized {
    fn death_reason(self, death_reason: &DeathReason) -> String;
    fn death_reason_boat(self, alias: PlayerAlias) -> String {
        self.death_reason_collision(&alias)
    }
    s!(death_reason_border);
    fn death_reason_collision(self, thing: impl Display) -> String;
    fn death_reason_obstacle(self, entity_type: EntityType) -> String {
        self.death_reason_collision(&entity_type.data().label)
    }
    fn death_reason_ram(self, alias: PlayerAlias) -> String;
    s!(death_reason_terrain);
    fn death_reason_weapon(self, alias: PlayerAlias, entity_type: EntityType) -> String;

    fn entity_kind_name(self, kind: EntityKind, sub_kind: EntitySubKind) -> &'static str {
        match (kind, sub_kind) {
            (EntityKind::Aircraft, EntitySubKind::Heli) => self.entity_aircraft_heli_name(),
            (EntityKind::Aircraft, EntitySubKind::Plane) => self.entity_aircraft_plane_name(),
            (EntityKind::Boat, EntitySubKind::Battleship) => self.entity_boat_battleship_name(),
            (EntityKind::Boat, EntitySubKind::Carrier) => self.entity_boat_carrier_name(),
            (EntityKind::Boat, EntitySubKind::Corvette) => self.entity_boat_corvette_name(),
            (EntityKind::Boat, EntitySubKind::Cruiser) => self.entity_boat_cruiser_name(),
            (EntityKind::Boat, EntitySubKind::Destroyer) => self.entity_boat_destroyer_name(),
            (EntityKind::Boat, EntitySubKind::Dreadnought) => self.entity_boat_dreadnought_name(),
            (EntityKind::Boat, EntitySubKind::Dredger) => self.entity_boat_dredger_name(),
            (EntityKind::Boat, EntitySubKind::Hovercraft) => self.entity_boat_hovercraft_name(),
            (EntityKind::Boat, EntitySubKind::Icebreaker) => self.entity_boat_icebreaker_name(),
            (EntityKind::Boat, EntitySubKind::Lcs) => self.entity_boat_lcs_name(),
            (EntityKind::Boat, EntitySubKind::Minelayer) => self.entity_boat_minelayer_name(),
            (EntityKind::Boat, EntitySubKind::Mtb) => self.entity_boat_mtb_name(),
            (EntityKind::Boat, EntitySubKind::Pirate) => self.entity_boat_pirate_name(),
            (EntityKind::Boat, EntitySubKind::Ram) => self.entity_boat_ram_name(),
            (EntityKind::Boat, EntitySubKind::Submarine) => self.entity_boat_submarine_name(),
            (EntityKind::Boat, EntitySubKind::Tanker) => self.entity_boat_tanker_name(),
            (EntityKind::Decoy, EntitySubKind::Sonar) => self.entity_decoy_sonar_name(),
            (EntityKind::Obstacle, EntitySubKind::Structure) => {
                self.entity_obstacle_structure_name()
            }
            (EntityKind::Weapon, EntitySubKind::Depositor) => self.entity_weapon_depositor_name(),
            (EntityKind::Weapon, EntitySubKind::DepthCharge) => {
                self.entity_weapon_depth_charge_name()
            }
            (EntityKind::Weapon, EntitySubKind::Mine) => self.entity_weapon_mine_name(),
            (EntityKind::Weapon, EntitySubKind::Missile) => self.entity_weapon_missile_name(),
            (EntityKind::Weapon, EntitySubKind::RocketTorpedo) => {
                self.entity_weapon_rocket_torpedo_name()
            }
            (EntityKind::Weapon, EntitySubKind::Rocket) => self.entity_weapon_rocket_name(),
            (EntityKind::Weapon, EntitySubKind::Sam) => self.entity_weapon_sam_name(),
            (EntityKind::Weapon, EntitySubKind::Shell) => self.entity_weapon_shell_name(),
            (EntityKind::Weapon, EntitySubKind::Torpedo) => self.entity_weapon_torpedo_name(),
            _ => {
                debug_assert!(false, "missing name for {:?}/{:?}", kind, sub_kind);
                "???"
            }
        }
    }

    fn entity_kind_hint(self, kind: EntityKind, sub_kind: EntitySubKind) -> &'static str {
        match (kind, sub_kind) {
            (EntityKind::Boat, EntitySubKind::Battleship) => self.entity_boat_battleship_hint(),
            (EntityKind::Boat, EntitySubKind::Carrier) => self.entity_boat_carrier_hint(),
            (EntityKind::Boat, EntitySubKind::Corvette) => self.entity_boat_corvette_hint(),
            (EntityKind::Boat, EntitySubKind::Cruiser) => self.entity_boat_cruiser_hint(),
            (EntityKind::Boat, EntitySubKind::Destroyer) => self.entity_boat_destroyer_hint(),
            (EntityKind::Boat, EntitySubKind::Dreadnought) => self.entity_boat_dreadnought_hint(),
            (EntityKind::Boat, EntitySubKind::Dredger) => self.entity_boat_dredger_hint(),
            (EntityKind::Boat, EntitySubKind::Hovercraft) => self.entity_boat_hovercraft_hint(),
            (EntityKind::Boat, EntitySubKind::Icebreaker) => self.entity_boat_icebreaker_hint(),
            (EntityKind::Boat, EntitySubKind::Lcs) => self.entity_boat_lcs_hint(),
            (EntityKind::Boat, EntitySubKind::Minelayer) => self.entity_boat_minelayer_hint(),
            (EntityKind::Boat, EntitySubKind::Mtb) => self.entity_boat_mtb_hint(),
            (EntityKind::Boat, EntitySubKind::Ram) => self.entity_boat_ram_hint(),
            (EntityKind::Boat, EntitySubKind::Submarine) => self.entity_boat_submarine_hint(),
            (EntityKind::Boat, EntitySubKind::Tanker) => self.entity_boat_tanker_hint(),
            _ => {
                debug_assert!(false, "missing hint for {:?}/{:?}", kind, sub_kind);
                "???"
            }
        }
    }

    s!(entity_aircraft_heli_name);
    s!(entity_aircraft_plane_name);
    s!(entity_boat_battleship_hint);
    s!(entity_boat_battleship_name);
    s!(entity_boat_carrier_hint);
    s!(entity_boat_carrier_name);
    s!(entity_boat_corvette_hint);
    s!(entity_boat_corvette_name);
    s!(entity_boat_cruiser_hint);
    s!(entity_boat_cruiser_name);
    s!(entity_boat_destroyer_hint);
    s!(entity_boat_destroyer_name);
    s!(entity_boat_dreadnought_hint);
    s!(entity_boat_dreadnought_name);
    s!(entity_boat_dredger_hint);
    s!(entity_boat_dredger_name);
    s!(entity_boat_hovercraft_hint);
    s!(entity_boat_hovercraft_name);
    s!(entity_boat_icebreaker_hint);
    s!(entity_boat_icebreaker_name);
    s!(entity_boat_lcs_hint);
    s!(entity_boat_lcs_name);
    s!(entity_boat_minelayer_hint);
    s!(entity_boat_minelayer_name);
    s!(entity_boat_mtb_hint);
    s!(entity_boat_mtb_name);
    s!(entity_boat_pirate_name);
    s!(entity_boat_ram_hint);
    s!(entity_boat_ram_name);
    s!(entity_boat_submarine_hint);
    s!(entity_boat_submarine_name);
    s!(entity_boat_tanker_hint);
    s!(entity_boat_tanker_name);
    s!(entity_decoy_sonar_name);
    s!(entity_obstacle_structure_name);
    s!(entity_weapon_depositor_name);
    s!(entity_weapon_depth_charge_name);
    s!(entity_weapon_mine_name);
    s!(entity_weapon_missile_name);
    s!(entity_weapon_rocket_torpedo_name);
    s!(entity_weapon_rocket_name);
    s!(entity_weapon_sam_name);
    s!(entity_weapon_shell_name);
    s!(entity_weapon_torpedo_name);

    s!(instruction_basics_mouse);
    s!(instruction_basics_touch);
    s!(instruction_zoom_mouse);
    s!(instruction_zoom_touch);

    s!(sensor_active_label);
    fn sensor_active_hint(self, sensors: &str) -> String;
    s!(sensor_radar_label);
    s!(sensor_sonar_label);

    s!(ship_surface_label);
    fn ship_surface_hint(self) -> String;

    s!(team_fleet_label);
    s!(team_fleet_name_placeholder);

    fn rewarded_ad(self, rewarded_ad: &RewardedAd) -> &'static str {
        match rewarded_ad {
            RewardedAd::Available { .. } => self.rewarded_ad_available(),
            RewardedAd::Watching => self.rewarded_ad_watching(),
            RewardedAd::Watched { .. } => self.rewarded_ad_watched(),
            _ => self.rewarded_ad_error(),
        }
    }
    s!(rewarded_ad_available);
    s!(rewarded_ad_watching);
    s!(rewarded_ad_watched);
    s!(rewarded_ad_error);
}

impl Mk48Translation for LanguageId {
    /*
    fn example(self) -> &'static str {
        match self {
            Arabic => "",
            Bork => "",
            English => "",
            French => "",
            German => "",
            Hindi => "",
            Italian => "",
            Japanese => "",
            Russian => "",
            SimplifiedChinese => "",
            Spanish => "",
            Vietnamese => "",
        }
    }

    fn example_2(self, sensors: &str) -> String {
        match self {
            English => format!(""),
            Spanish => format!(""),
            French => format!(""),
            German => format!(""),
            Italian => format!(""),
            Russian => format!(""),
            Arabic => format!(""),
            Hindi => format!(""),
            SimplifiedChinese => format!(""),
            Japanese => format!(""),
            Vietnamese => format!(""),
            Bork => format!(""),
        }
    }
    */

    fn death_reason(self, death_reason: &DeathReason) -> String {
        match death_reason {
            &DeathReason::Boat(alias) => self.death_reason_boat(alias),
            DeathReason::Border => self.death_reason_border().to_owned(),
            &DeathReason::Obstacle(entity_type) => self.death_reason_obstacle(entity_type),
            &DeathReason::Ram(alias) => self.death_reason_ram(alias),
            DeathReason::Terrain => self.death_reason_terrain().to_owned(),
            &DeathReason::Weapon(alias, entity_type) => {
                self.death_reason_weapon(alias, entity_type)
            }
            _ => {
                debug_assert!(false, "unexpected {:?}", death_reason);
                String::from("Died of unexplained causes.")
            }
        }
    }

    fn death_reason_border(self) -> &'static str {
        match self {
            Arabic => "تحطمت في الحدود!",
            Bork => "Borked by borkder!",
            English => "Crashed into the border!",
            French => "S'est écrasé à la frontière!",
            German => "In die Grenze gekracht!",
            Hindi => "सीमा में घुस गया!",
            Italian => "Schiantato al confine!",
            Japanese => "国境に激突!",
            Russian => "Врезался в границу!",
            SimplifiedChinese => "坠入边境!",
            Spanish => "¡Se estrelló contra la frontera!",
            Vietnamese => "Đập vào biên giới!",
        }
    }

    fn death_reason_collision(self, thing: impl Display) -> String {
        match self {
            Arabic => format!("تحطمت في {thing}!"),
            Bork => format!("Borked by {thing}!"),
            English => format!("Crashed into {thing}!"),
            French => format!("Crash dans {thing}!"),
            German => format!("In {thing} gekracht!"),
            Hindi => format!("{thing} में दुर्घटनाग्रस्त हो गया!"),
            Italian => format!("Schiantato contro {thing}!"),
            Japanese => format!("{thing}にクラッシュしました!"),
            Russian => format!("Разбился на {thing}!"),
            SimplifiedChinese => format!("撞到了 {thing}!"),
            Spanish => format!("¡Se estrelló contra {thing}!"),
            Vietnamese => format!("Đã đâm vào {thing}!"),
        }
    }

    fn death_reason_ram(self, alias: PlayerAlias) -> String {
        match self {
            Arabic => format!("صدم {alias}!"),
            Bork => format!("Borked by {alias}!"),
            English => format!("Rammed by {alias}!"),
            French => format!("Battu par {alias}!"),
            German => format!("Von {alias} gerammt!"),
            Hindi => format!("{alias} द्वारा घुसा!"),
            Italian => format!("Speronato da {alias}!"),
            Japanese => format!("{alias}に突っ込まれました!"),
            Russian => format!("Таранит {alias}!"),
            SimplifiedChinese => format!("被{alias}撞了！"),
            Spanish => format!("¡Embestido por {alias}!"),
            Vietnamese => format!("Bị tấn công bởi {alias}!"),
        }
    }

    fn death_reason_terrain(self) -> &'static str {
        match self {
            Arabic => "تحطمت في الأرض!",
            Bork => "Borked by the bround!",
            English => "Crashed into the ground!",
            French => "Enfoncé dans le sol!",
            German => "Ins Land gekracht!",
            Hindi => "जमीन में गिर गया!",
            Italian => "Schiantato contro il terreno!",
            Japanese => "地面に激突!",
            Russian => "Врезался в землю!",
            SimplifiedChinese => "摔在地上!",
            Spanish => "¡Se estrelló contra el suelo!",
            Vietnamese => "Đập xuống đất!",
        }
    }

    fn death_reason_weapon(self, alias: PlayerAlias, entity_type: EntityType) -> String {
        let weapon: String = format!("{:?}", entity_type.data().sub_kind);
        match self {
            Arabic => format!("غرق بواسطة {alias} {weapon}!"),
            Bork => format!("Borked by {alias} with a {weapon}!"),
            English => format!("Sunk by {alias} with a {weapon}!"),
            French => format!("Coulé par {alias} avec un {weapon}!"),
            German => format!("Von {alias} mit {weapon} versenkt!"),
            Hindi => format!("{alias} द्वारा {weapon} के साथ डूब गया!"),
            Italian => format!("Affondato da {alias} con un {weapon}!"),
            Japanese => format!("{weapon}で{alias}に沈められました!"),
            Russian => format!("Потоплен {alias} с помощью {weapon}!"),
            SimplifiedChinese => format!("被 {alias} 用 {weapon} 击沉!"),
            Spanish => format!("¡Hundido por {alias} con un {weapon}!"),
            Vietnamese => format!("Chìm đắm bởi {alias} với {weapon}!"),
        }
    }

    fn entity_aircraft_heli_name(self) -> &'static str {
        match self {
            Arabic => "هليكوبتر",
            Bork => "borkopter",
            English => "helicopter",
            French => "hélicoptère",
            German => "Helicopter",
            Hindi => "हेलीकॉप्टर",
            Italian => "elicottero",
            Japanese => "ヘリコプター",
            Russian => "вертолет",
            SimplifiedChinese => "直升机",
            Spanish => "helicóptero",
            Vietnamese => "máy bay trực thăng",
        }
    }

    fn entity_aircraft_plane_name(self) -> &'static str {
        match self {
            Arabic => "طائرة",
            Bork => "flying bork",
            English => "plane",
            French => "avion",
            German => "Flugzeug",
            Hindi => "विमान",
            Italian => "aereo",
            Japanese => "飛行機",
            Russian => "самолет",
            SimplifiedChinese => "机",
            Spanish => "avión",
            Vietnamese => "phi cơ",
        }
    }

    fn entity_boat_battleship_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك لديها بنادق قوية والكثير من الدروع!",
            Bork => "Bork is the strongest bork",
            English => "Your ship has powerful guns and plenty of armor!",
            French => "Votre vaisseau a des canons puissants et beaucoup d'armures!",
            German => "Dein Schiff hat starke Kanonen und viel Panzerung!",
            Hindi => "आपके जहाज में शक्तिशाली बंदूकें और ढेर सारे कवच हैं!",
            Italian => "La tua nave ha armi potenti e molta armatura!",
            Japanese => "あなたの船は強力な銃とたくさんの鎧を持っています!",
            Russian => "У вашего корабля мощные орудия и много брони!",
            SimplifiedChinese => "你的船有强大的枪支和大量的装甲!",
            Spanish => "¡Tu barco tiene armas poderosas y mucha armadura!",
            Vietnamese => "Tàu của bạn có súng mạnh và nhiều áo giáp!",
        }
    }

    fn entity_boat_battleship_name(self) -> &'static str {
        match self {
            Arabic => "سفينة حربية",
            Bork => "borkererer",
            English => "battleship",
            French => "bataille navale",
            German => "Kriegsschiff",
            Hindi => "युद्धपोत",
            Italian => "corazzata",
            Japanese => "戦艦",
            Russian => "линкор",
            SimplifiedChinese => "战舰",
            Spanish => "acorazado",
            Vietnamese => "tàu chiến",
        }
    }

    fn entity_boat_carrier_hint(self) -> &'static str {
        match self {
            Arabic => "يمكن لسفينتك إطلاق طائرات بأسلحة خاصة بهم!",
            Bork => "Bork launches airborks!",
            English => "Your ship can launch aircraft with weapons of their own!",
            French => "Votre vaisseau peut lancer des avions avec leurs propres armes!",
            German => "Dein Schiff kann Fluggeräte abheben lassen welche Waffen besitzen!",
            Hindi => "आपका जहाज अपने स्वयं के हथियारों के साथ विमान लॉन्च कर सकता है!",
            Italian => "La tua nave può lanciare aerei con proprie armi!",
            Japanese => "あなたの船は彼ら自身の武器で航空機を発射することができます!",
            Russian => "Ваш корабль может запускать самолеты с собственным оружием!",
            SimplifiedChinese => "你的船可以用自己的武器发射飞机!",
            Spanish => "¡Tu barco puede lanzar aviones con sus propias armas!",
            Vietnamese => "Tàu của bạn có thể phóng máy bay với vũ khí của riêng mình!",
        }
    }

    fn entity_boat_carrier_name(self) -> &'static str {
        match self {
            Arabic => "حاملة طائرات",
            Bork => "bork carrier",
            English => "aircraft carrier",
            French => "porte-avions",
            German => "Flugzeugträger",
            Hindi => "विमान वाहक",
            Italian => "portaerei",
            Japanese => "空母",
            Russian => "авианосец",
            SimplifiedChinese => "航空母舰",
            Spanish => "portaaviones",
            Vietnamese => "tàu sân bay",
        }
    }

    fn entity_boat_corvette_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك صغيرة ويصعب الوصول إليها!",
            Bork => "Bork is a small bork!",
            English => "Your ship is small and difficult to hit!",
            French => "Votre vaisseau est petit et difficile à toucher!",
            German => "Dein Schiff ist klein und schwierig zu treffen!",
            Hindi => "आपका जहाज छोटा है और हिट करना मुश्किल है!",
            Italian => "La tua nave è piccola e difficile da colpire!",
            Japanese => "あなたの船は小さくて打撃が難しいです!",
            Russian => "Ваш корабль маленький, и в него сложно попасть!",
            SimplifiedChinese => "你的船很小，很难被击中!",
            Spanish => "¡Tu nave es pequeña y difícil de alcanzar!",
            Vietnamese => "Tàu của bạn nhỏ và khó đánh!",
        }
    }

    fn entity_boat_corvette_name(self) -> &'static str {
        match self {
            Arabic => "كورفيت",
            Bork => "minibork",
            English => "corvette",
            French => "corvette",
            German => "Korvette",
            Hindi => "कौर्वेट",
            Italian => "corvetta",
            Japanese => "コルベット",
            Russian => "корвет",
            SimplifiedChinese => "护卫舰",
            Spanish => "corbeta",
            Vietnamese => "tàu hộ tống",
        }
    }

    fn entity_boat_cruiser_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك مجهزة بأسلحة مضادة للسفن والغواصات!",
            Bork => "Bork is even gooder at borking other borks!",
            English => "Your ship is equipped with anti-ship and anti-submarine weapons!",
            French => "Votre navire est équipé d'armes anti-navire et anti-sous-marine!",
            German => "Dein Schiff ist mit Anti-Schiff und Anti-U-Boot Waffen ausgerüstet!",
            Hindi => "आपका जहाज जहाज रोधी और पनडुब्बी रोधी हथियारों से लैस है!",
            Italian => "La tua nave è equipaggiata con armi anti-sottomarino e anti-nave!",
            Japanese => "あなたの船には対艦兵器と対潜水艦兵器が装備されています!",
            Russian => "Ваш корабль оснащен противокорабельным и противолодочным вооружением!",
            SimplifiedChinese => "你的船配备了反舰和反潜武器!",
            Spanish => "¡Tu barco está equipado con armas antibuque y antisubmarinas!",
            Vietnamese => "Tàu của bạn được trang bị vũ khí chống hạm và chống tàu ngầm!",
        }
    }

    fn entity_boat_cruiser_name(self) -> &'static str {
        match self {
            Arabic => "طراد",
            Bork => "borkerer",
            English => "cruiser",
            French => "croiseur",
            German => "Kreuzer",
            Hindi => "क्रूजर",
            Italian => "incrociatore",
            Japanese => "クルーザー",
            Russian => "крейсер",
            SimplifiedChinese => "巡洋舰",
            Spanish => "crucero",
            Vietnamese => "tàu tuần dương",
        }
    }

    fn entity_boat_destroyer_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك مجهزة بمجموعة متنوعة من الأسلحة!",
            Bork => "Bork is good at borking other borks!",
            English => "Your ship is equipped with a variety of weapons!",
            French => "Votre vaisseau est équipé d'une variété d'armes!",
            German => "Dein Schiff ist mit einer Menge verschiedener Waffen ausgerüstet!",
            Hindi => "आपका जहाज विभिन्न प्रकार के हथियारों से लैस है!",
            Italian => "La tua nave è equipaggiata con una varietà di armi!",
            Japanese => "あなたの船にはさまざまな武器が装備されています!",
            Russian => "Ваш корабль оснащен разнообразным вооружением!",
            SimplifiedChinese => "你的船配备了各种武器!",
            Spanish => "¡Tu barco está equipado con una variedad de armas!",
            Vietnamese => "Tàu của bạn được trang bị nhiều loại vũ khí!",
        }
    }

    fn entity_boat_destroyer_name(self) -> &'static str {
        match self {
            Arabic => "مدمر",
            Bork => "borker",
            English => "destroyer",
            French => "destructeur",
            German => "Zerstörer",
            Hindi => "मिटाने वाला",
            Italian => "cacciatorpediniere",
            Japanese => "駆逐艦",
            Russian => "миноносец",
            SimplifiedChinese => "驱逐舰",
            Spanish => "destructor",
            Vietnamese => "kẻ huỷ diệt",
        }
    }

    fn entity_boat_dreadnought_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك لديها مدافع قوية!",
            Bork => "Bork is a well-armed bork!",
            English => "Your ship has powerful cannons!",
            French => "Votre navire est doté de puissants canons !",
            German => "Dein Schiff hat starke Kanonen!",
            Hindi => "आपके जहाज में शक्तिशाली तोपें हैं!",
            Italian => "La tua nave ha cannoni potenti!",
            Japanese => "あなたの船には強力な大砲があります!",
            Russian => "У вашего корабля мощные пушки!",
            SimplifiedChinese => "你的船有强大的大炮!",
            Spanish => "¡Tu barco tiene poderosos cañones!",
            Vietnamese => "Tàu của bạn có những khẩu đại bác mạnh mẽ!",
        }
    }

    fn entity_boat_dreadnought_name(self) -> &'static str {
        match self {
            Arabic => "مدرعة",
            Bork => "dreadbork",
            English => "dreadnought",
            French => "cuirassé",
            German => "Schlachtschiff",
            Hindi => "एक प्रकार का लड़ाई का जहाज़",
            Italian => "corazzata",
            Japanese => "ドレッドノート",
            Russian => "дредноут",
            SimplifiedChinese => "无畏",
            Spanish => "acorazado",
            Vietnamese => "thiết giáp hạm",
        }
    }

    fn entity_boat_dredger_hint(self) -> &'static str {
        match self {
            Arabic => "سفينتك يمكن أن تخلق وتدمر الأرض!",
            Bork => "Bork can bork land!",
            English => "Your ship can create and destroy land!",
            French => "Votre navire peut créer et détruire des terres!",
            German => "Dein Schiff kann Land erschaffen und zerstören!",
            Hindi => "आपका जहाज भूमि बना और नष्ट कर सकता है!",
            Italian => "La tua nave può creare e distruggere il terreno!",
            Japanese => "あなたの船は土地を作り、破壊することができます!",
            Russian => "Ваш корабль может создавать и разрушать землю!",
            SimplifiedChinese => "你的船可以创造和摧毁土地!",
            Spanish => "¡Tu barco puede crear y destruir tierra!",
            Vietnamese => "Tàu của bạn có thể tạo ra và phá hủy đất liền!",
        }
    }

    fn entity_boat_dredger_name(self) -> &'static str {
        match self {
            Arabic => "الحفارة",
            Bork => "land borker",
            English => "dredger",
            French => "dragueur",
            German => "Baggerschiff",
            Hindi => "ड्रैजेर",
            Italian => "draga",
            Japanese => "地面に激突!",
            Russian => "экскаватор",
            SimplifiedChinese => "挖泥船",
            Spanish => "draga",
            Vietnamese => "tàu cuốc",
        }
    }

    fn entity_boat_hovercraft_hint(self) -> &'static str {
        match self {
            Arabic => "يمكن للقارب الخاص بك السفر على اليابسة والماء!",
            Bork => "Bork is a landbork",
            English => "Your boat can travel on both land and water!",
            French => "Votre bateau peut voyager aussi bien sur terre que sur l'eau!",
            German => "Dein Schiff kann sich auf Wasser und Land fortbewegen!",
            Hindi => "आपकी नाव जमीन और पानी दोनों पर चल सकती है!",
            Italian => "La tua barca può navigare sia sull' acqua che sul terreno!",
            Japanese => "あなたのボートは陸と水の両方を旅することができます!",
            Russian => "Ваша лодка может путешествовать как по суше, так и по воде!",
            SimplifiedChinese => "您的船可以在陆地和水上行驶!",
            Spanish => "¡Su barco puede viajar tanto por tierra como por agua!",
            Vietnamese => "Thuyền của bạn có thể đi cả trên cạn và dưới nước!",
        }
    }

    fn entity_boat_hovercraft_name(self) -> &'static str {
        match self {
            Arabic => "الحوامات",
            Bork => "hoverbork",
            English => "hovercraft",
            French => "aéroglisseur",
            German => "Luftkissenfahrzeug",
            Hindi => "हुवरक्रफ़्ट",
            Italian => "hovercraft",
            Japanese => "ホバークラフト",
            Russian => "судно на воздушной подушке",
            SimplifiedChinese => "气垫船",
            Spanish => "aerodeslizador",
            Vietnamese => "thủy phi cơ",
        }
    }

    fn entity_boat_icebreaker_hint(self) -> &'static str {
        match self {
            Arabic => "تستطيع سفينتك أن تحرث الصفائح الجليدية!",
            Bork => "Bork can bork ice sheets!",
            English => "Your ship can plow through ice sheets!",
            French => "Votre navire peut traverser les calottes glaciaires!",
            German => "Dein Schiff kann durch Eisschilde pflügen!",
            Hindi => "आपका जहाज बर्फ की चादर में हल चला सकता है!",
            Italian => "La tua nave può solcare le lastre di ghiaccio!",
            Japanese => "あなたの船は氷床を耕すことができます!",
            Russian => "Ваш корабль может преодолевать ледяные щиты!",
            SimplifiedChinese => "你的船可以穿过冰盖!",
            Spanish => "¡Tu barco puede atravesar capas de hielo!",
            Vietnamese => "Con tàu của bạn có thể xuyên qua các tảng băng!",
        }
    }

    fn entity_boat_icebreaker_name(self) -> &'static str {
        match self {
            Arabic => "كاسحة الجليد",
            Bork => "iceborker",
            English => "icebreaker",
            French => "brise-glace",
            German => "Eisbrecher",
            Hindi => "आइसब्रेकर",
            Italian => "rompighiaccio",
            Japanese => "砕氷船",
            Russian => "ледокол",
            SimplifiedChinese => "破冰船",
            Spanish => "rompehielos",
            Vietnamese => "tàu phá băng",
        }
    }

    fn entity_boat_lcs_hint(self) -> &'static str {
        match self {
            Arabic => {
                "يمكن للقارب الخاص بك إطلاق العنان للأسلحة الفتاكة من داخل مجموعات الجزر الصغيرة!"
            }
            Bork => "Bork can bork other borks from small island groups!",
            English => "Your boat can unleash deadly weapons from within small island groups!",
            French => {
                "Votre bateau peut déchaîner des armes mortelles au sein de petits groupes d'îles!"
            }
            German => "Dein Schiff kann tödliche Waffen in der Nähe von Inselgruppen abfeuern!",
            Hindi => "आपकी नाव छोटे द्वीप समूहों के भीतर से घातक हथियार निकाल सकती है!",
            Italian => "La tua barca può rilasciare armi letali da piccoli gruppi di isole!",
            Japanese => {
                "あなたのボートは小さな島のグループの中から致命的な武器を解き放つことができます!"
            }
            Russian => {
                "Ваша лодка может выпускать смертоносное оружие из небольших островных групп!"
            }
            SimplifiedChinese => "您的船可以从小岛群内释放致命的武器!",
            Spanish => {
                "¡Tu barco puede desencadenar armas mortales desde pequeños grupos de islas!"
            }
            Vietnamese => {
                "Thuyền của bạn có thể giải phóng vũ khí chết người từ trong các nhóm đảo nhỏ!"
            }
        }
    }

    fn entity_boat_lcs_name(self) -> &'static str {
        match self {
            Arabic => "سفينة قتالية ساحلية",
            Bork => "coastal bork",
            English => "littoral combat ship",
            French => "navire de combat côtier",
            German => "Küstennahes Kampfschiff",
            Hindi => "समुद्रतटीय लड़ाकू जहाज",
            Italian => "nave da combattimento costiera",
            Japanese => "沿海域戦闘艦",
            Russian => "прибрежный боевой корабль",
            SimplifiedChinese => "濒海战斗舰",
            Spanish => "barco de combate costero",
            Vietnamese => "tàu chiến đấu ven biển",
        }
    }

    fn entity_boat_minelayer_hint(self) -> &'static str {
        match self {
            Arabic => "يمكن للقارب الخاص بك وضع ألغام مغناطيسية مميتة",
            Bork => "Bork lays magnetic borks!",
            English => "Your boat can lay deadly magnetic mines",
            French => "Votre bateau peut poser des mines magnétiques mortelles!",
            German => "Dein Schiff kann tötliche Minen legen!",
            Hindi => "आपकी नाव घातक चुंबकीय खदानें बिछा सकती है",
            Italian => "La tua barca può depositare mine magnetiche mortali",
            Japanese => "あなたのボートは致命的な磁気地雷を置くことができます",
            Russian => "Ваша лодка может устанавливать смертельные магнитные мины!",
            SimplifiedChinese => "你的船可以布下致命的磁性水雷!",
            Spanish => "¡Su barco puede colocar minas magnéticas mortales!",
            Vietnamese => "Thuyền của bạn có thể đặt những quả mìn từ trường chết người",
        }
    }

    fn entity_boat_minelayer_name(self) -> &'static str {
        match self {
            Arabic => "طبقة منجم",
            Bork => "borklayer",
            English => "minelayer",
            French => "mouilleur de mines",
            German => "Minenleger",
            Hindi => "सुरंग लगानेवाला जहाज़",
            Italian => "posamine",
            Japanese => "機雷敷設艦",
            Russian => "заградитель",
            SimplifiedChinese => "矿工",
            Spanish => "minador",
            Vietnamese => "thợ mỏ",
        }
    }

    fn entity_boat_mtb_hint(self) -> &'static str {
        match self {
            Arabic => "قاربك لديه أسلحة لإغراق قوارب أخرى!",
            Bork => "Bork can bork other borks!",
            English => "Your boat has weapons to sink other boats!",
            French => "Votre bateau a des armes pour couler d'autres bateaux!",
            German => "Dein Schiff hat Waffen um andere Schiffe zu versenken!",
            Hindi => "आपकी नाव में अन्य नावों को डुबाने के लिए हथियार हैं!",
            Italian => "La tua barca ha armi per affondare altre barche!",
            Japanese => "あなたのボートには他のボートを沈めるための武器があります!",
            Russian => "У вашей лодки есть оружие, чтобы топить другие лодки!",
            SimplifiedChinese => "你的船有武器可以击沉其他船只!",
            Spanish => "¡Tu barco tiene armas para hundir otros barcos!",
            Vietnamese => "Thuyền của bạn có vũ khí để đánh chìm thuyền khác!",
        }
    }

    fn entity_boat_mtb_name(self) -> &'static str {
        match self {
            Arabic => "قارب طوربيد بمحرك",
            Bork => "motor-torpedo bork",
            English => "motor-torpedo boat",
            French => "bateau lance-torpilles",
            German => "Motor-Torpedo Boot",
            Hindi => "मोटर-टारपीडो नाव",
            Italian => "barca motosilurante",
            Japanese => "モーター魚雷艇",
            Russian => "моторно-торпедный катер",
            SimplifiedChinese => "机动鱼雷艇",
            Spanish => "barco de motor-torpedo",
            Vietnamese => "thuyền máy phóng ngư lôi",
        }
    }

    fn entity_boat_pirate_name(self) -> &'static str {
        match self {
            Arabic => "القرصان",
            Bork => "illegal bork",
            English => "pirate",
            French => "pirate",
            German => "Pirat",
            Hindi => "समुद्री डाकू",
            Italian => "pirata",
            Japanese => "海賊",
            Russian => "пират",
            SimplifiedChinese => "海盗",
            Spanish => "pirata",
            Vietnamese => "cướp biển",
        }
    }

    fn entity_boat_ram_hint(self) -> &'static str {
        match self {
            Arabic => "تم تصميم قاربك لصد القوارب الأخرى!",
            Bork => "Bork is designed to bork other borks!",
            English => "Your boat is designed to ram other boats!",
            French => "Votre bateau est conçu pour éperonner d'autres bateaux!",
            German => "Dein Schiff kann andere Schiffe rammen!",
            Hindi => "आपकी नाव को अन्य नावों को चलाने के लिए डिज़ाइन किया गया है!",
            Italian => "La tua barca è progettata per speronare altre barche!",
            Japanese => "あなたのボートは他のボートにぶつかるように設計されています!",
            Russian => "Ваша лодка создана, чтобы таранить другие лодки!",
            SimplifiedChinese => "您的船旨在撞击其他船只!",
            Spanish => "¡Su barco está diseñado para embestir a otros barcos!",
            Vietnamese => "Thuyền của bạn được thiết kế để đâm những chiếc thuyền khác!",
        }
    }

    fn entity_boat_ram_name(self) -> &'static str {
        match self {
            Arabic => "سفينة الكبش",
            Bork => "pointy bork",
            English => "ram",
            French => "bélier",
            German => "Rammbock",
            Hindi => "राम जहाज",
            Italian => "ariete",
            Japanese => "牡羊",
            Russian => "таран",
            SimplifiedChinese => "羝",
            Spanish => "ariete",
            Vietnamese => "máy đem nước lên",
        }
    }

    fn entity_boat_submarine_hint(self) -> &'static str {
        match self {
            Arabic => "يمكن للقارب الخاص بك تسليم الأسلحة من تحت الماء!",
            Bork => "Bork can bork other borks from underwater!",
            English => "Your boat can deliver weapons from underwater!",
            French => "Votre bateau peut livrer des armes sous l'eau!",
            German => "Dein Schiff kann Waffen unterwasser abfeuern!",
            Hindi => "आपकी नाव पानी के नीचे से हथियार पहुंचा सकती है!",
            Italian => "La tua barca può lanciare armi sott'acqua!",
            Japanese => "あなたのボートは水中から武器を届けることができます!",
            Russian => "Ваша лодка может доставлять оружие из-под воды!",
            SimplifiedChinese => "你的船可以从水下运送武器!",
            Spanish => "¡Tu barco puede lanzar armas desde el agua!",
            Vietnamese => "Thuyền của bạn có thể cung cấp vũ khí từ dưới nước!",
        }
    }

    fn entity_boat_submarine_name(self) -> &'static str {
        match self {
            Arabic => "غواصة",
            Bork => "underwater bork",
            English => "submarine",
            French => "sous-marin",
            German => "U-Boot",
            Hindi => "पनडुब्बी",
            Italian => "sottomarino",
            Japanese => "潜水艦",
            Russian => "подводная лодка",
            SimplifiedChinese => "潜艇",
            Spanish => "submarina",
            Vietnamese => "tàu ngầm",
        }
    }

    fn entity_boat_tanker_hint(self) -> &'static str {
        match self {
            Arabic => "يحصل قاربك على ضعف القيمة من براميل النفط!",
            Bork => "Bork good at drinking oil!",
            English => "Your boat gets double the value from oil barrels!",
            French => "Votre bateau obtient le double de la valeur des barils de pétrole!",
            German => "Dein Schiff kriegt zwei mal so viele Punkte für Öl-Fässer!",
            Hindi => "आपकी नाव को तेल के बैरल से दोगुना मूल्य मिलता है!",
            Italian => "La tua barca riceve il doppio dei punti dai barili di petrolio!",
            Japanese => "あなたのボートは石油バレルから2倍の価値を得ます!",
            Russian => "Ваша лодка получает вдвое большую ценность из-за нефтяных бочек!",
            SimplifiedChinese => "您的船从油桶中获得两倍的价值!",
            Spanish => "¡Su barco obtiene el doble del valor de los barriles de petróleo!",
            Vietnamese => "Thuyền của bạn nhận được gấp đôi giá trị từ thùng dầu!",
        }
    }

    fn entity_boat_tanker_name(self) -> &'static str {
        match self {
            Arabic => "ناقلة",
            Bork => "oil borker",
            English => "tanker",
            French => "pétrolier",
            German => "Tanker",
            Hindi => "टैंकर",
            Italian => "petroliera",
            Japanese => "タンカー",
            Russian => "нефтяной танкер",
            SimplifiedChinese => "油船",
            Spanish => "petrolero",
            Vietnamese => "tàu chở dầu",
        }
    }

    fn entity_decoy_sonar_name(self) -> &'static str {
        match self {
            Arabic => "شرك السونار",
            Bork => "underwater bork distractor",
            English => "sonar decoy",
            French => "leurre sonar",
            German => "Echolot Täuschkörper",
            Hindi => "सोनार डिकॉय",
            Italian => "esca sonar",
            Japanese => "ソナーデコイ",
            Russian => "гидролокатор-ловушка",
            SimplifiedChinese => "声纳诱饵",
            Spanish => "señuelo de sonar",
            Vietnamese => "mồi nhử sonar",
        }
    }

    fn entity_obstacle_structure_name(self) -> &'static str {
        match self {
            Arabic => "بنية",
            Bork => "structure",
            English => "structure",
            French => "structure",
            German => "Struktur",
            Hindi => "संरचना",
            Italian => "struttura",
            Japanese => "構造",
            Russian => "состав",
            SimplifiedChinese => "结构",
            Spanish => "estructura",
            Vietnamese => "vật kiến trúc",
        }
    }

    fn entity_weapon_depositor_name(self) -> &'static str {
        match self {
            Arabic => "المودع",
            Bork => "land borker",
            English => "depositor",
            French => "déposant",
            German => "Ablagerer",
            Hindi => "जमाकर्ता",
            Italian => "depositante",
            Japanese => "寄託者",
            Russian => "депозитор",
            SimplifiedChinese => "储户",
            Spanish => "transportadora",
            Vietnamese => "người gửi tiền",
        }
    }

    fn entity_weapon_depth_charge_name(self) -> &'static str {
        match self {
            Arabic => "عمق الشحن",
            Bork => "depth bork",
            English => "depth charge",
            French => "grenade sous-marine",
            German => "Wasserbombe",
            Hindi => "जलगत बम",
            Italian => "carica di profondità",
            Japanese => "爆雷",
            Russian => "глубинная бомба",
            SimplifiedChinese => "深水炸弹",
            Spanish => "carga de profundidad",
            Vietnamese => "xạc sâu",
        }
    }

    fn entity_weapon_mine_name(self) -> &'static str {
        match self {
            Arabic => "لغم بحري",
            Bork => "magnetic bork",
            English => "mine",
            French => "mine",
            German => "Mine",
            Hindi => "नौसेना खान",
            Italian => "mina",
            Japanese => "機雷",
            Russian => "мина",
            SimplifiedChinese => "水雷",
            Spanish => "mina",
            Vietnamese => "mìn",
        }
    }

    fn entity_weapon_missile_name(self) -> &'static str {
        match self {
            Arabic => "صاروخ",
            Bork => "guided airbork",
            English => "missile",
            French => "missile",
            German => "Lenkrakete",
            Hindi => "मिसाइल",
            Italian => "missile",
            Japanese => "ミサイル",
            Russian => "ракета",
            SimplifiedChinese => "导弹",
            Spanish => "misil",
            Vietnamese => "hỏa tiễn",
        }
    }

    fn entity_weapon_rocket_torpedo_name(self) -> &'static str {
        match self {
            Arabic => "صاروخ طوربيد",
            Bork => "air-launched underwater bork",
            English => "rocket torpedo",
            French => "torpille de fusée",
            German => "Anti-U-Boot Rakete",
            Hindi => "रॉकेट टारपीडो",
            Italian => "razzo siluro",
            Japanese => "ロケット魚雷",
            Russian => "ракетная торпеда",
            SimplifiedChinese => "火箭鱼雷",
            Spanish => "torpedo cohete",
            Vietnamese => "ngư lôi tên lửa",
        }
    }

    fn entity_weapon_rocket_name(self) -> &'static str {
        match self {
            Arabic => "صاروخ",
            Bork => "airbork",
            English => "rocket",
            French => "fusée",
            German => "Rakete",
            Hindi => "राकेट",
            Italian => "razzo",
            Japanese => "ロケット",
            Russian => "ракета",
            SimplifiedChinese => "火箭",
            Spanish => "cohete",
            Vietnamese => "tên lửa",
        }
    }

    fn entity_weapon_sam_name(self) -> &'static str {
        match self {
            Arabic => "صاروخ أرض جو",
            Bork => "surface-to-air bork",
            English => "surface-to-air missile",
            French => "missile sol-air",
            German => "Flugabwehrrakete",
            Hindi => "सतह से हवा में मार करने वाली मिसाइल",
            Italian => "missile terra-aria",
            Japanese => "地対空ミサイル",
            Russian => "ракета земля-воздух",
            SimplifiedChinese => "地对空导弹",
            Spanish => "misil tierra-aire",
            Vietnamese => "tên lửa đất đối không",
        }
    }

    fn entity_weapon_shell_name(self) -> &'static str {
        match self {
            Arabic => "قذيفة",
            Bork => "destructive bork",
            English => "shell",
            French => "obus",
            German => "Kanone",
            Hindi => "खोल",
            Italian => "proiettile",
            Japanese => "弾丸",
            Russian => "снаряд",
            SimplifiedChinese => "炮击",
            Spanish => "caparazón",
            Vietnamese => "vỏ bọc",
        }
    }

    fn entity_weapon_torpedo_name(self) -> &'static str {
        match self {
            Arabic => "نسف",
            Bork => "underwater bork",
            English => "torpedo",
            French => "torpille",
            German => "Torpedo",
            Hindi => "टारपीडो",
            Italian => "siluro",
            Japanese => "魚雷",
            Russian => "торпеда",
            SimplifiedChinese => "鱼雷",
            Spanish => "torpedo",
            Vietnamese => "điện ngư",
        }
    }

    fn instruction_basics_mouse(self) -> &'static str {
        match self {
            Arabic => "انقر مع الاستمرار للتحرك ، انقر لإطلاق طوربيدات",
            English | Bork => "Click and hold to move, click to fire torpedoes",
            French => "Cliquez et maintenez pour vous déplacer, cliquez pour tirer des torpilles",
            German => "Klicke und halte um dich zu bewegen. Klick kurz um zu feuern.",
            Hindi => "स्थानांतरित करने के लिए क्लिक करें और दबाए रखें, टॉरपीडो फायर करने के लिए क्लिक करें",
            Italian => "Fai clic e mantieni premuto per muoverti, clicca per sparare siluri",
            Japanese => "クリックして押し続けると移動し、クリックして魚雷を発射します",
            Russian => "Нажмите и удерживайте, чтобы двигаться, нажмите, чтобы стрелять торпедами",
            SimplifiedChinese => "点击并按住移动，点击发射鱼雷",
            Spanish => {
                "Haga clic y mantenga presionado para moverse, haga clic para disparar torpedos"
            }
            Vietnamese => "Nhấp và giữ để di chuyển, nhấp để bắn ngư lôi",
        }
    }

    fn instruction_basics_touch(self) -> &'static str {
        match self {
            Arabic => "المس في اتجاه للتحرك ، انقر لإطلاق طوربيدات",
            English | Bork => "Touch in a direction to move, tap to fire torpedoes",
            French => {
                "Touchez dans une direction pour vous déplacer, touchez pour tirer des torpilles"
            }
            German => "Tippe und halte um dich zu bewegen. Tippe kurz um zu feuern.",
            Hindi => "स्थानांतरित करने के लिए एक दिशा में स्पर्श करें, टॉरपीडो को आग लगाने के लिए टैप करें",
            Italian => "Mantieni il tocco in una direzione per muoverti, tap per sparare siluri",
            Japanese => "移動する方向にタッチし、タップして魚雷を発射します",
            Russian => "Коснитесь в направлении движения, коснитесь, чтобы запустить торпеды",
            SimplifiedChinese => "触摸一个方向移动，点击发射鱼雷",
            Spanish => "Toque en una dirección para moverse, toque para disparar torpedos",
            Vietnamese => "Chạm vào một hướng để di chuyển, chạm để bắn ngư lôi",
        }
    }

    fn instruction_zoom_mouse(self) -> &'static str {
        match self {
            Arabic => "قم بالتمرير للتصغير للحصول على عرض أفضل",
            English | Bork => "Scroll to zoom out for a better view",
            French => "Faites défiler pour dézoomer pour une meilleure vue",
            German => "Scrolle um herauszuzoomen.",
            Hindi => "बेहतर दृश्य के लिए ज़ूम आउट करने के लिए स्क्रॉल करें",
            Italian => "Scrolla per rimpicciolire per una vista migliore",
            Japanese => "スクロールしてズームアウトすると、見やすくなります",
            Russian => "Прокрутите, чтобы уменьшить масштаб для лучшего обзора",
            SimplifiedChinese => "滚动以缩小以获得更好的视图",
            Spanish => "Desplácese para alejar y ver mejor",
            Vietnamese => "Cuộn để thu nhỏ để xem tốt hơn",
        }
    }

    fn instruction_zoom_touch(self) -> &'static str {
        match self {
            Arabic => "قرصة للتصغير للحصول على عرض أفضل",
            English | Bork => "Pinch to zoom out for a better view",
            French => "Pincez pour dézoomer pour une meilleure vue",
            German => "Ziehe die Finger zusammen um herauszuzoomen.",
            Hindi => "बेहतर दृश्य के लिए ज़ूम आउट करने के लिए पिंच करें",
            Italian => "Pizzica per rimpicciolire per una vista migliore",
            Japanese => "ピンチしてズームアウトすると見やすくなります",
            Russian => "Сведите пальцы, чтобы уменьшить масштаб для лучшего обзора",
            SimplifiedChinese => "捏合以缩小以获得更好的视图",
            Spanish => "Pellizca para alejar y ver mejor",
            Vietnamese => "Chụm để thu nhỏ để có chế độ xem tốt hơn",
        }
    }

    fn sensor_active_label(self) -> &'static str {
        match self {
            Arabic => "أجهزة استشعار نشطة",
            Bork => "Sensors go brrr",
            English => "Active sensors",
            French => "Capteurs actifs",
            German => "Aktive Sensoren",
            Hindi => "सक्रिय सेंसर",
            Italian => "Sensori attivi",
            Japanese => "アクティブセンサー",
            Russian => "Активные датчики",
            SimplifiedChinese => "有源传感器",
            Spanish => "Sensores activo",
            Vietnamese => "Cảm biến hoạt động",
        }
    }

    fn sensor_active_hint(self, sensors: &str) -> String {
        let key = ACTIVE_KEY;
        match self {
            English => format!("({key}) Active {sensors} helps you see more, but may also give away your position"),
            Spanish => format!("({key}) Los {sensors} activos te ayudan a ver más, pero también pueden revelar tu posición"),
            French => format!("({key}) Les {sensors} actifs vous aident à voir plus, mais peuvent également révéler votre position"),
            German => format!("({key}) Dein {sensors} hilft dir mehr zu sehen, aber es verrät auch deine Position."),
            Italian => format!("({key}) I {sensors} attivi ti aiutano a vedere di più, ma possono anche rivelare la tua posizione"),
            Russian => format!("({key}) Активные {sensors} помогают вам видеть больше, но также могут выдавать ваше местоположение"),
            Arabic => format!("({key}) تساعدك {sensors} النشطة على رؤية المزيد ، ولكنها قد تفقد موضعك أيضًا"),
            Hindi => format!("({key}) सक्रिय {sensors} आपको अधिक देखने में मदद करता है, लेकिन यह आपकी स्थिति को दूर भी कर सकता है"),
            SimplifiedChinese => format!("({key}) 活跃的 {sensors} 可以帮助您看到更多，但也可能会泄露您的位置"),
            Japanese => format!("({key}) アクティブな{sensors}は、より多くの情報を確認するのに役立ちますが、自分の位置を示す可能性もあります"),
            Vietnamese => format!("({key}) {sensors} đang hoạt động giúp bạn nhìn thấy nhiều hơn, nhưng cũng có thể làm mất đi vị trí của bạn"),
            Bork => format!("({key}) Active {sensors} helps bork see more, but may also give away bork's position"),
        }
    }

    fn sensor_radar_label(self) -> &'static str {
        match self {
            Arabic => "رادار",
            Bork => "Radar",
            English => "Radar",
            French => "Radar",
            German => "Radar",
            Hindi => "राडार",
            Italian => "Radar",
            Japanese => "レーダー",
            Russian => "Радар",
            SimplifiedChinese => "雷达",
            Spanish => "Radar",
            Vietnamese => "Rađa",
        }
    }

    fn sensor_sonar_label(self) -> &'static str {
        match self {
            Arabic => "سونار",
            Bork => "Sonar",
            English => "Sonar",
            French => "Sonar",
            German => "Echolot",
            Hindi => "सोनार",
            Italian => "Sonar",
            Japanese => "ソナー",
            Russian => "Сонар",
            SimplifiedChinese => "声纳",
            Spanish => "Sonar",
            Vietnamese => "Sonar",
        }
    }

    fn ship_surface_label(self) -> &'static str {
        match self {
            Arabic => "سطح",
            Bork => "Surface",
            English => "Surface",
            French => "Surface",
            German => "Tauchen",
            Hindi => "सतह",
            Italian => "Superficie",
            Japanese => "水面",
            Russian => "Поверхность",
            SimplifiedChinese => "表面",
            Spanish => "Superficie",
            Vietnamese => "Mặt",
        }
    }

    fn ship_surface_hint(self) -> String {
        let key = SURFACE_KEY;
        match self {
            Arabic => format!("({key}) يمكنك سطح سفينتك وقتما تشاء ، لكن الغوص أحيانًا يكون مقيدًا بعمق الماء"),
            Bork => format!("({key}) Bork can surface wherever bork wants, but diving is sometimes limited by the depth of the water"),
            English => format!("({key}) You can surface your ship whenever you want, but diving is sometimes limited by the depth of the water"),
            French => format!("({key}) Vous pouvez faire surfacer votre bateau quand vous le souhaitez, mais la plongée est parfois limitée par la profondeur de l'eau"),
            German => format!("({key}) Du kannst jederzeit auftauchen aber die Tauchtiefe ist von der Wassertiefe begrenzt"),
            Hindi => format!("({key}) आप जब चाहें अपने जहाज को सतह पर ला सकते हैं, लेकिन कभी-कभी गोताखोरी पानी की गहराई से सीमित होती है।"),
            Italian => format!("({key}) Puoi far emergere la tua nave quando vuoi, ma le immersioni a volte sono limitate dalla profondità dell'acqua"),
            Japanese => format!("({key}) いつでも船を浮上させることができますが、ダイビングは水深によって制限されることがあります。"),
            Russian => format!("({key}) Вы можете всплыть на поверхность своего корабля, когда захотите, но погружение иногда ограничивается глубиной воды"),
            SimplifiedChinese => format!("({key}) 您可以随时浮出水面，但潜水有时会受到水深的限制"),
            Spanish => format!("({key}) Puede salir a la superficie de su barco cuando lo desee, pero el buceo a veces está limitado por la profundidad del agua"),
            Vietnamese => format!("({key}) Bạn có thể nổi con tàu của mình bất cứ khi nào bạn muốn, nhưng việc lặn đôi khi bị giới hạn bởi độ sâu của nước"),
        }
    }

    fn team_fleet_label(self) -> &'static str {
        match self {
            Arabic => "أسطول",
            Bork => "Borks",
            English => "Fleet",
            French => "Flotte",
            German => "Flotte",
            Hindi => "बेड़ा",
            Italian => "Flotta",
            Japanese => "海軍",
            Russian => "Флот",
            SimplifiedChinese => "舰队",
            Spanish => "Flota",
            Vietnamese => "Hạm đội",
        }
    }

    fn team_fleet_name_placeholder(self) -> &'static str {
        match self {
            Arabic => "اسم الأسطول",
            Bork => "Name of borks",
            English => "Fleet name",
            French => "Nom de la flotte",
            German => "Flottenname",
            Hindi => "बेड़े का नाम",
            Italian => "Nome della flotta",
            Japanese => "艦隊名",
            Russian => "Название флота",
            SimplifiedChinese => "机队名称",
            Spanish => "Nombre",
            Vietnamese => "Tên hạm đội",
        }
    }

    fn rewarded_ad_available(self) -> &'static str {
        match self {
            Arabic => "فتح محتوى المكافأة",
            Bork => "Unbork bonus content",
            English => "Unlock bonus content",
            French => "Débloquez du contenu bonus",
            German => "Bonusinhalte freischalten",
            Hindi => "बोनस सामग्री अनलॉक करें",
            Italian => "Sblocca contenuti bonus",
            Japanese => "ボーナスコンテンツのロックを解除",
            Russian => "Разблокировать бонусный контент",
            SimplifiedChinese => "解锁奖励内容",
            Spanish => "Desbloquear contenido extra",
            Vietnamese => "Mở khóa nội dung tiền thưởng",
        }
    }

    fn rewarded_ad_watching(self) -> &'static str {
        match self {
            Arabic => "طلب إعلان",
            English | Bork => "Requesting ad...",
            French => "Demande d'annonce...",
            German => "Anzeige anfordern...",
            Hindi => "विज्ञापन का अनुरोध किया जा रहा है...",
            Italian => "Richiesta annuncio...",
            Japanese => "広告をリクエストしています...",
            Russian => "Запрос объявления...",
            SimplifiedChinese => "请求广告...",
            Spanish => "Solicitando anuncio...",
            Vietnamese => "Yêu cầu quảng cáo...",
        }
    }

    fn rewarded_ad_watched(self) -> &'static str {
        match self {
            Arabic => "مفتوحة!",
            Bork => "Unborked!",
            English => "Unlocked!",
            French => "Débloqué !",
            German => "Entsperrt!",
            Hindi => "अनलॉक!",
            Italian => "Sbloccato!",
            Japanese => "ロック解除！",
            Russian => "Разблокировано!",
            SimplifiedChinese => "解锁！",
            Spanish => "¡Desbloqueado!",
            Vietnamese => "Đã mở khóa!",
        }
    }

    fn rewarded_ad_error(self) -> &'static str {
        match self {
            Arabic => "خطأ إعلان",
            English | Bork => "Ad error",
            French => "Erreur d'annonce",
            German => "Anzeigenfehler",
            Hindi => "विज्ञापन त्रुटि।",
            Italian => "Errore annuncio",
            Japanese => "広告エラー",
            Russian => "Ошибка объявления",
            SimplifiedChinese => "广告错误",
            Spanish => "Error de anuncio",
            Vietnamese => "Lỗi quảng cáo",
        }
    }
}
