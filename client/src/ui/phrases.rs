// SPDX-FileCopyrightText: 2021 Softbear, Inc.

use crate::game::{ACTIVE_KEY, SURFACE_KEY};
use common::death_reason::DeathReason;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use kodiak_client::{translate, PlayerAlias, RewardedAd, Translator};
use std::fmt::Display;

pub trait Mk48Phrases {
    fn entity_kind_name(&self, kind: EntityKind, sub_kind: EntitySubKind) -> String;
    fn entity_kind_hint(&self, kind: EntityKind, sub_kind: EntitySubKind) -> String;
    fn _rewarded_ad(&self, rewarded_ad: &RewardedAd) -> String;
    fn death_reason(&self, death_reason: &DeathReason) -> String;
    fn death_reason_collision(&self, thing: impl Display) -> String;
    fn death_reason_obstacle(&self, entity_type: EntityType) -> String;
    fn death_reason_weapon(&self, alias: PlayerAlias, entity_type: EntityType) -> String;
    //fn level(&self, level: u8) -> String;
    fn sensor_active_label(&self) -> String;
    fn sensor_active_hint(&self, sensors: &str) -> String;
    fn sensor_radar_label(&self) -> String;
    fn sensor_sonar_label(&self) -> String;
    fn ship_surface_label(&self) -> String;
    fn ship_surface_hint(&self) -> String;
    fn team_fleet_label(&self) -> String;
    fn team_fleet_name_placeholder(&self) -> String;
}

impl Mk48Phrases for Translator {
    fn entity_kind_name(&self, kind: EntityKind, sub_kind: EntitySubKind) -> String {
        match (kind, sub_kind) {
            (EntityKind::Aircraft, EntitySubKind::Heli) => {
                translate!(self, "helicopter")
            }
            (EntityKind::Aircraft, EntitySubKind::Plane) => {
                translate!(self, "plane")
            }
            (EntityKind::Boat, EntitySubKind::Battleship) => {
                translate!(self, "battleship")
            }
            (EntityKind::Boat, EntitySubKind::Carrier) => {
                translate!(self, "aircraft carrier")
            }
            (EntityKind::Boat, EntitySubKind::Corvette) => {
                translate!(self, "corvette")
            }
            (EntityKind::Boat, EntitySubKind::Cruiser) => {
                translate!(self, "cruiser")
            }
            (EntityKind::Boat, EntitySubKind::Destroyer) => {
                translate!(self, "destroyer")
            }
            (EntityKind::Boat, EntitySubKind::Dreadnought) => {
                translate!(self, "dreadnought")
            }
            (EntityKind::Boat, EntitySubKind::Dredger) => {
                translate!(self, "dredger")
            }
            (EntityKind::Boat, EntitySubKind::Hovercraft) => {
                translate!(self, "hovercraft")
            }
            (EntityKind::Boat, EntitySubKind::Icebreaker) => {
                translate!(self, "icebreaker")
            }
            (EntityKind::Boat, EntitySubKind::Lcs) => {
                translate!(self, "littoral combat ship")
            }
            (EntityKind::Boat, EntitySubKind::Minelayer) => {
                translate!(self, "minelayer")
            }
            (EntityKind::Boat, EntitySubKind::Mtb) => {
                translate!(self, "motor-torpedo boat")
            }
            (EntityKind::Boat, EntitySubKind::Pirate) => {
                translate!(self, "pirate")
            }
            (EntityKind::Boat, EntitySubKind::Ram) => {
                translate!(self, "ram")
            }
            (EntityKind::Boat, EntitySubKind::Submarine) => {
                translate!(self, "submarine")
            }
            (EntityKind::Boat, EntitySubKind::Tanker) => {
                translate!(self, "tanker")
            }
            (EntityKind::Decoy, EntitySubKind::Sonar) => {
                translate!(self, "sonar decoy")
            }
            (EntityKind::Obstacle, EntitySubKind::Structure) => {
                translate!(self, "structure")
            }
            (EntityKind::Weapon, EntitySubKind::Depositor) => {
                translate!(self, "depositor")
            }
            (EntityKind::Weapon, EntitySubKind::DepthCharge) => {
                translate!(self, "depth charge")
            }
            (EntityKind::Weapon, EntitySubKind::Mine) => {
                translate!(self, "mine")
            }
            (EntityKind::Weapon, EntitySubKind::Missile) => {
                translate!(self, "missile")
            }
            (EntityKind::Weapon, EntitySubKind::RocketTorpedo) => {
                translate!(self, "rocket torpedo")
            }
            (EntityKind::Weapon, EntitySubKind::Rocket) => {
                translate!(self, "rocket")
            }
            (EntityKind::Weapon, EntitySubKind::Sam) => {
                translate!(self, "surface-to-air missile")
            }
            (EntityKind::Weapon, EntitySubKind::Shell) => {
                translate!(self, "shell")
            }
            (EntityKind::Weapon, EntitySubKind::Torpedo) => {
                translate!(self, "torpedo")
            }
            _ => {
                debug_assert!(false, "missing name for {:?}/{:?}", kind, sub_kind);
                "???".to_string()
            }
        }
    }

    fn entity_kind_hint(&self, kind: EntityKind, sub_kind: EntitySubKind) -> String {
        match (kind, sub_kind) {
            (EntityKind::Boat, EntitySubKind::Battleship) => {
                translate!(self, "Your ship has powerful guns and plenty of armor!")
            }
            (EntityKind::Boat, EntitySubKind::Carrier) => translate!(
                self,
                "Your ship can launch aircraft with weapons of their own!"
            ),
            (EntityKind::Boat, EntitySubKind::Corvette) => {
                translate!(self, "Your ship is small and difficult to hit!")
            }
            (EntityKind::Boat, EntitySubKind::Cruiser) => translate!(
                self,
                "Your ship is equipped with anti-ship and anti-submarine weapons!"
            ),
            (EntityKind::Boat, EntitySubKind::Destroyer) => {
                translate!(self, "Your ship is equipped with a variety of weapons!")
            }
            (EntityKind::Boat, EntitySubKind::Dreadnought) => {
                translate!(self, "Your ship has powerful cannons!")
            }
            (EntityKind::Boat, EntitySubKind::Dredger) => {
                translate!(self, "Your ship can create and destroy land!")
            }
            (EntityKind::Boat, EntitySubKind::Hovercraft) => {
                translate!(self, "Your boat can travel on both land and water!")
            }
            (EntityKind::Boat, EntitySubKind::Icebreaker) => {
                translate!(self, "Your ship can plow through ice sheets!")
            }
            (EntityKind::Boat, EntitySubKind::Lcs) => translate!(
                self,
                "Your boat can unleash deadly weapons from within small island groups!"
            ),
            (EntityKind::Boat, EntitySubKind::Minelayer) => {
                translate!(self, "Your boat can lay deadly magnetic mines")
            }
            (EntityKind::Boat, EntitySubKind::Mtb) => {
                translate!(self, "Your boat has weapons to sink other boats!")
            }
            (EntityKind::Boat, EntitySubKind::Ram) => {
                translate!(self, "Your boat is designed to ram other boats!")
            }
            (EntityKind::Boat, EntitySubKind::Submarine) => {
                translate!(self, "Your boat can deliver weapons from underwater!")
            }
            (EntityKind::Boat, EntitySubKind::Tanker) => {
                translate!(self, "Your boat gets double the value from oil barrels!")
            }
            (EntityKind::Boat, EntitySubKind::Pirate) => {
                translate!(self, "Yer ship be a sittin' duck")
            }
            _ => {
                debug_assert!(false, "missing hint for {:?}/{:?}", kind, sub_kind);
                "???".to_string()
            }
        }
    }

    fn _rewarded_ad(&self, rewarded_ad: &RewardedAd) -> String {
        match rewarded_ad {
            RewardedAd::Available { .. } => {
                translate!(self, "Unlock bonus content")
            }
            RewardedAd::Watching { .. } => {
                translate!(self, "Requesting ad...")
            }
            RewardedAd::Watched { .. } => translate!(self, "Unlocked!"),
            _ => translate!(self, "Ad error"),
        }
    }

    fn death_reason(&self, death_reason: &DeathReason) -> String {
        match death_reason {
            &DeathReason::Boat(alias) => self.death_reason_collision(alias),
            DeathReason::Border => {
                translate!(self, "Crashed into the border!")
            }
            &DeathReason::Obstacle(entity_type) => self.death_reason_obstacle(entity_type),
            &DeathReason::Ram(alias) => translate!(self, "Rammed by {alias}!"),
            DeathReason::Terrain => {
                translate!(self, "Crashed into the ground!")
            }
            &DeathReason::Weapon(alias, entity_type) => {
                self.death_reason_weapon(alias, entity_type)
            }
            _ => {
                debug_assert!(false, "unexpected {:?}", death_reason);
                String::from("Died of unexplained causes.")
            }
        }
    }

    fn death_reason_collision(&self, thing: impl Display) -> String {
        translate!(self, "Crashed into {thing}!")
    }

    fn death_reason_obstacle(&self, entity_type: EntityType) -> String {
        self.death_reason_collision(&entity_type.data().label)
    }

    fn death_reason_weapon(&self, alias: PlayerAlias, entity_type: EntityType) -> String {
        let data = entity_type.data();
        let weapon = self.entity_kind_name(data.kind, data.sub_kind);
        translate!(self, "Sunk by {alias} with a {weapon}!")
    }

    /*
    fn level(&self, level: u8) -> String {
        translate!(self, "Level {level}")
    }
    */

    fn sensor_active_label(&self) -> String {
        translate!(self, "Active sensors")
    }

    fn sensor_active_hint(&self, sensors: &str) -> String {
        let key = ACTIVE_KEY;
        translate!(
            self,
            "({key}) Active {sensors} helps you see more, but may also give away your position"
        )
    }

    fn sensor_radar_label(&self) -> String {
        translate!(self, "Radar")
    }

    fn sensor_sonar_label(&self) -> String {
        translate!(self, "Sonar")
    }

    fn ship_surface_label(&self) -> String {
        translate!(self, "Surface")
    }

    fn ship_surface_hint(&self) -> String {
        let key = SURFACE_KEY;
        translate!(self, "({key}) You can surface your ship whenever you want, but diving is sometimes limited by the depth of the water")
    }

    fn team_fleet_label(&self) -> String {
        translate!(self, "Fleet")
    }

    fn team_fleet_name_placeholder(&self) -> String {
        translate!(self, "Fleet name")
    }
}
