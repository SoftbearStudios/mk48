// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::game::Mk48Game;
use client_util::audio::AudioLayer;
use client_util::context::Context;
use common::contact::{Contact, ContactTrait};
use common::entity::EntityId;
use common::entity::{EntityData, EntityKind, EntitySubKind};
use common::ticks::Ticks;
use common_util::angle::Angle;
use common_util::range::map_ranges;
use glam::Vec2;
use std::collections::HashMap;

/// A contact that may be locally controlled by simulated elsewhere (by the server).
pub struct InterpolatedContact {
    /// The more accurate representation of the contact, which is snapped to server updates.
    pub model: Contact,
    /// The visual representation of the contact, which is gradually interpolated towards model.
    pub view: Contact,
    /// Integrate error to control rubber banding strength. Having an error for longer means stronger
    /// interpolation back to model.
    pub error: f32,
    /// Idle ticks, i.e. how many updates since last seen. If exceeds entity_type.data().keep_alive(),
    /// assume entity went away.
    pub idle: Ticks,
}

impl InterpolatedContact {
    pub(crate) fn new(contact: Contact) -> Self {
        // When a new contact appears, its model and view are identical.
        Self {
            model: contact.clone(),
            view: contact,
            error: 0.0,
            idle: Ticks::ZERO,
        }
    }
}

impl Mk48Game {
    pub fn lost_contact(
        &mut self,
        player_position: Vec2,
        contact: &Contact,
        audio_layer: &AudioLayer,
        animations: &mut Vec<Animation>,
    ) {
        if let Some(entity_type) = contact.entity_type() {
            // Contact lost (of a previously known entity type), spawn a splash and make a sound.
            let volume =
                Mk48Game::volume_at(player_position.distance(contact.transform().position))
                    .min(0.25);
            let name = match entity_type.data().kind {
                EntityKind::Boat | EntityKind::Aircraft => "splash",
                EntityKind::Weapon => match entity_type.data().sub_kind {
                    EntitySubKind::Missile
                    | EntitySubKind::Sam
                    | EntitySubKind::Rocket
                    | EntitySubKind::Shell => "explosion",
                    _ => "splash",
                },
                EntityKind::Collectible => {
                    audio_layer.play_with_volume("collect", volume);
                    return;
                }
                _ => return,
            };

            if entity_type.data().kind == EntityKind::Boat {
                audio_layer.play_with_volume("explosion_long", volume);
            } else {
                audio_layer.play_with_volume("explosion_short", volume);
            }

            animations.push(Animation::new(
                name,
                contact.transform().position,
                contact.altitude().to_norm(),
                10.0,
            ));
        }
    }

    pub(crate) fn maybe_contact_mut(
        contacts: &mut HashMap<EntityId, InterpolatedContact>,
        entity_id: Option<EntityId>,
    ) -> Option<&mut InterpolatedContact> {
        entity_id.map(move |id| contacts.get_mut(&id).unwrap())
    }

    pub fn new_contact(
        &mut self,
        contact: &Contact,
        player_position: Vec2,
        context: &Context<Mk48Game>,
        audio_layer: &AudioLayer,
    ) {
        let position_diff = contact.transform().position - player_position;
        let direction = Angle::from(position_diff);
        let inbound = (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

        let friendly = context.core().is_friendly(contact.player_id());
        let volume = Mk48Game::volume_at(position_diff.length());

        if let Some(entity_type) = contact.entity_type() {
            let data: &EntityData = entity_type.data();

            match data.kind {
                EntityKind::Boat => {
                    if !friendly && inbound && context.game().entity_id.is_some() {
                        audio_layer.play_with_volume("alarm_slow", 0.25 * volume.max(0.5));
                    }
                }
                EntityKind::Weapon => match data.sub_kind {
                    EntitySubKind::Torpedo => {
                        if friendly {
                            audio_layer.play_with_volume("torpedo_launch", volume.min(0.5));
                            audio_layer.play_with_volume_and_delay("splash", volume, 0.1);
                        }
                        if data.sensors.sonar.range > 0.0 {
                            audio_layer.play_with_volume_and_delay(
                                "sonar3",
                                volume,
                                if friendly { 1.0 } else { 0.0 },
                            );
                        }
                    }
                    EntitySubKind::Missile | EntitySubKind::Rocket => {
                        if !friendly
                            && inbound
                            && context.game().entity_id.is_some()
                            && self.alarm_fast_rate_limiter.ready()
                        {
                            audio_layer.play_with_volume("alarm_fast", volume.max(0.5));
                        }
                        audio_layer.play_with_volume("rocket", volume);
                    }
                    EntitySubKind::Sam => {
                        audio_layer.play_with_volume("rocket", volume);
                    }
                    EntitySubKind::DepthCharge | EntitySubKind::Mine => {
                        audio_layer.play_with_volume("splash", volume);
                        if !friendly && context.game().entity_id.is_some() {
                            audio_layer.play_with_volume("alarm_slow", volume.max(0.5));
                        }
                    }
                    EntitySubKind::Shell => {
                        audio_layer.play_with_volume(
                            "shell",
                            volume * map_ranges(data.length, 0.5..1.5, 0.5..1.0, true),
                        );
                    }
                    _ => {}
                },
                EntityKind::Aircraft => {
                    if !friendly && inbound {
                        audio_layer.play_with_volume("alarm_slow", 0.1 * volume.max(0.5));
                    }
                }
                EntityKind::Decoy => {
                    if data.sub_kind == EntitySubKind::Sonar {
                        audio_layer.play_with_volume("sonar3", volume);
                    }
                }
                _ => {}
            }
        }
    }

    /// Simulate delta_seconds passing, by updating guidance and kinematics.
    pub(crate) fn propagate_contact(contact: &mut Contact, delta_seconds: f32) {
        if let Some(entity_type) = contact.entity_type() {
            let guidance = *contact.guidance();
            let max_speed = match entity_type.data().sub_kind {
                // Wait until risen to surface.
                EntitySubKind::Missile | EntitySubKind::Rocket | EntitySubKind::Sam
                    if contact.altitude().is_submerged() =>
                {
                    EntityData::SURFACING_PROJECTILE_SPEED_LIMIT
                }
                _ => f32::INFINITY,
            };

            contact.transform_mut().apply_guidance(
                entity_type.data(),
                guidance,
                max_speed,
                delta_seconds,
            );
        }
        contact.transform_mut().do_kinematics(delta_seconds);
    }
}
