// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::audio::Audio;
use crate::game::{Mk48Game, Mk48Layer};
use crate::particle::Mk48Particle;
use client_util::audio::AudioPlayer;
use client_util::context::Context;
use common::contact::{Contact, ContactTrait};
use common::entity::EntityId;
use common::entity::{EntityData, EntityKind, EntitySubKind};
use common::ticks::Ticks;
use common_util::angle::Angle;
use common_util::range::map_ranges;
use glam::Vec2;
use js_hooks::console_log;
use rand::{thread_rng, Rng};
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
    /// Initializes an interpolated contact.
    pub(crate) fn new(contact: Contact) -> Self {
        // When a new contact appears, its model and view are identical.
        Self {
            model: contact.clone(),
            view: contact,
            error: 0.0,
            idle: Ticks::ZERO,
        }
    }

    /// Updates measure of discrepancy between model and view, known as "error."
    pub fn update_error_bound(
        &mut self,
        elapsed_seconds: f32,
        debug_latency_entity_id: Option<EntityId>,
    ) {
        let positional_inaccuracy = self
            .model
            .transform()
            .position
            .distance_squared(self.view.transform().position);
        let directional_inaccuracy = (self.model.transform().direction
            - self.view.transform().direction)
            .abs()
            .to_radians();
        let velocity_inaccuracy = self
            .model
            .transform()
            .velocity
            .difference(self.view.transform().velocity)
            .to_mps();

        if Some(self.view.id()) == debug_latency_entity_id {
            console_log!(
                "err: {:.2}, pos: {:.2}, dir: {:.2}, vel: {:.2}",
                self.error,
                positional_inaccuracy.sqrt(),
                directional_inaccuracy,
                velocity_inaccuracy
            );
        }
        self.error = (self.error * 0.5f32.powf(elapsed_seconds)
            + elapsed_seconds
                * (positional_inaccuracy * 0.4
                    + directional_inaccuracy * 2.0
                    + velocity_inaccuracy * 0.08))
            .clamp(0.0, 10.0);
    }

    /// Generates particles from changes between model and view, such as muzzle flash particles when
    /// an armament goes from available to consumed.
    pub fn generate_particles(&mut self, layer: &mut Mk48Layer) {
        // If reloads are known before and after, and one goes from zero to non-zero, it was fired.
        if let Some(entity_type) = self.model.entity_type() {
            let data: &EntityData = entity_type.data();
            if self.view.entity_type() == self.model.entity_type()
                && self.view.reloads_known()
                && self.model.reloads_known()
                && self.view.turrets_known()
            {
                let model_reloads = self.model.reloads();
                for (i, old) in self.view.reloads().iter().enumerate() {
                    let new = model_reloads[i];

                    if new || !old {
                        // Wasn't just fired
                        continue;
                    }

                    let armament = &data.armaments[i];
                    let armament_entity_data = armament.entity_type.data();

                    if !matches!(
                        armament_entity_data.sub_kind,
                        EntitySubKind::Shell
                            | EntitySubKind::Rocket
                            | EntitySubKind::RocketTorpedo
                            | EntitySubKind::Missile
                    ) {
                        // Don't generate particles.
                        continue;
                    }

                    let boat_velocity = self.view.transform().direction.to_vec()
                        * self.view.transform().velocity.to_mps();

                    let armament_transform =
                        *self.view.transform() + data.armament_transform(self.view.turrets(), i);

                    let direction_vector: Vec2 = if armament.vertical {
                        // Straight up.
                        Vec2::ZERO
                    } else {
                        armament_transform.direction.into()
                    };

                    let mut rng = thread_rng();

                    let forward_offset = armament
                        .turret
                        .and_then(|t| data.turrets[t].entity_type)
                        .map(|t| t.data().length * 0.4)
                        .unwrap_or(2.0);
                    let forward_velocity = 0.5 * armament_entity_data.speed.to_mps().min(100.0);

                    let is_submerged = self.view.altitude().is_submerged();

                    // Add muzzle flash particles.
                    let amount = 10;
                    for i in 0..amount {
                        let particle = Mk48Particle {
                            position: armament_transform.position
                                + direction_vector * forward_offset,
                            velocity: boat_velocity
                                + direction_vector
                                    * forward_velocity
                                    * (i as f32 * (1.0 / amount as f32))
                                + direction_vector.perp()
                                    * forward_velocity
                                    * 0.15
                                    * (rng.gen::<f32>() - 0.5),
                            radius: (armament_entity_data.width * 5.0).clamp(1.0, 3.0),
                            color: -1.0,
                            smoothness: 1.0,
                        };

                        if is_submerged {
                            layer.sea_level_particles.add(particle);
                        } else {
                            layer.airborne_particles.add(particle);
                        }
                    }
                }
            }
        }
    }

    /// Performs interpolation. Takes the entity id of the player's boat.
    pub fn interpolate(&mut self, elapsed_seconds: f32, player_entity_id: Option<EntityId>) {
        // Don't interpolate view's guidance if this is the player's boat, so that it doesn't jerk around.
        self.view.interpolate_towards(
            &self.model,
            Some(self.model.id()) != player_entity_id,
            elapsed_seconds * self.error,
            elapsed_seconds,
        );
        self.model.simulate(elapsed_seconds);
        self.view.simulate(elapsed_seconds);
    }
}

impl Mk48Game {
    /// Call when a contact disappears (keep alive already expired).
    ///
    /// Fine not to call if audio and animations not desired.
    pub fn play_lost_contact_audio_and_animations(
        &mut self,
        player_position: Vec2,
        contact: &Contact,
        audio_layer: &AudioPlayer<Audio>,
        animations: &mut Vec<Animation>,
        time_seconds: f32,
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
                    | EntitySubKind::RocketTorpedo
                    | EntitySubKind::Shell => "explosion",
                    _ => "splash",
                },
                EntityKind::Collectible => {
                    audio_layer.play_with_volume(Audio::Collect, volume);
                    return;
                }
                _ => return,
            };

            let data = entity_type.data();
            if data.kind == EntityKind::Boat {
                audio_layer.play_with_volume(Audio::ExplosionLong, volume);
            } else {
                audio_layer.play_with_volume(Audio::ExplosionShort, volume);
            }

            // The more damage/health the entity has the larger its explosion is.
            debug_assert!(data.damage >= 0.0);
            let scale = (data.damage.sqrt() * 10.0).clamp(5.0, 40.0);

            animations.push(Animation::new(
                name,
                contact.transform().position,
                contact.altitude().to_norm(),
                scale,
                time_seconds,
            ));
        }
    }

    pub(crate) fn maybe_contact_mut(
        contacts: &mut HashMap<EntityId, InterpolatedContact>,
        entity_id: Option<EntityId>,
    ) -> Option<&mut InterpolatedContact> {
        entity_id.map(move |id| contacts.get_mut(&id).unwrap())
    }

    /// Call when a previously-unseen contact appears.
    ///
    /// Fine not to call if sounds are undesired.
    pub fn play_new_contact_audio(
        &mut self,
        contact: &Contact,
        player_position: Vec2,
        context: &Context<Mk48Game>,
        audio_layer: &AudioPlayer<Audio>,
    ) {
        let position_diff = contact.transform().position - player_position;
        let direction = Angle::from(position_diff);
        let inbound = (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

        let friendly = context.state.core.is_friendly(contact.player_id());
        let volume = Mk48Game::volume_at(position_diff.length());

        if let Some(entity_type) = contact.entity_type() {
            let data: &EntityData = entity_type.data();

            match data.kind {
                EntityKind::Boat => {
                    if !friendly && inbound && context.state.game.entity_id.is_some() {
                        audio_layer.play_with_volume(Audio::AlarmSlow, 0.25 * volume.max(0.5));
                    }
                }
                EntityKind::Weapon => match data.sub_kind {
                    EntitySubKind::Torpedo => {
                        if friendly {
                            audio_layer.play_with_volume(Audio::TorpedoLaunch, volume.min(0.5));
                            audio_layer.play_with_volume_and_delay(Audio::Splash, volume, 0.1);
                        }
                        if data.sensors.sonar.range > 0.0 {
                            audio_layer.play_with_volume_and_delay(
                                Audio::Sonar3,
                                volume,
                                if friendly { 1.0 } else { 0.0 },
                            );
                        }
                    }
                    EntitySubKind::Missile | EntitySubKind::Rocket => {
                        if !friendly
                            && inbound
                            && context.state.game.entity_id.is_some()
                            && self.alarm_fast_rate_limiter.ready()
                        {
                            audio_layer.play_with_volume(Audio::AlarmFast, volume.max(0.5));
                        }
                        audio_layer.play_with_volume(Audio::Rocket, volume);
                    }
                    EntitySubKind::Sam | EntitySubKind::RocketTorpedo => {
                        audio_layer.play_with_volume(Audio::Rocket, volume);
                    }
                    EntitySubKind::DepthCharge | EntitySubKind::Mine => {
                        audio_layer.play_with_volume(Audio::Splash, volume);
                        if !friendly && context.state.game.entity_id.is_some() {
                            audio_layer.play_with_volume(Audio::AlarmSlow, volume.max(0.5));
                        }
                    }
                    EntitySubKind::Shell => {
                        audio_layer.play_with_volume(
                            Audio::Shell,
                            volume * map_ranges(data.length, 0.5..1.5, 0.5..1.0, true),
                        );
                    }
                    _ => {}
                },
                EntityKind::Aircraft => {
                    if !friendly && inbound {
                        audio_layer.play_with_volume(Audio::AlarmSlow, 0.1 * volume.max(0.5));
                    }
                }
                EntityKind::Decoy => {
                    if data.sub_kind == EntitySubKind::Sonar {
                        audio_layer.play_with_volume(Audio::Sonar3, volume);
                    }
                }
                _ => {}
            }
        }
    }
}
