// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::angle::Angle;
use crate::entity::*;
use crate::guidance::Guidance;
use crate::ticks::Ticks;
use crate::transform::Transform;
use crate::util::make_mut_slice;
use crate::velocity::Velocity;
use bitvec::prelude::*;
use core_protocol::id::*;
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Formatter;
use std::iter::repeat_with;
use std::sync::Arc;

pub type ReloadsStorage = u32;

pub trait ContactTrait {
    fn altitude(&self) -> Altitude;

    fn damage(&self) -> Ticks;

    fn entity_type(&self) -> Option<EntityType>;

    fn guidance(&self) -> &Guidance;

    fn id(&self) -> EntityId;

    fn player_id(&self) -> Option<PlayerId>;

    fn reloads(&self) -> &BitSlice<ReloadsStorage>;

    /// Whether reloads() will return real data or all zeroes.
    fn reloads_known(&self) -> bool;

    fn transform(&self) -> &Transform;

    fn turrets(&self) -> &[Angle];

    /// Whether turrets() will return real data or all zeroes.
    fn turrets_known(&self) -> bool;

    #[inline]
    fn is_boat(&self) -> bool {
        self.entity_type()
            .map_or(false, |t| t.data().kind == EntityKind::Boat)
    }

    #[inline]
    fn data(&self) -> &'static EntityData {
        self.entity_type().unwrap().data()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    transform: Transform,
    altitude: Altitude,
    guidance: Guidance,
    damage: Ticks,
    entity_type: Option<EntityType>,
    id: EntityId,
    player_id: Option<PlayerId>,
    reloads: Option<BitArray<ReloadsStorage>>,
    turrets: Option<Arc<[Angle]>>,
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            altitude: Altitude::default(),
            damage: Ticks::default(),
            entity_type: None,
            guidance: Guidance::default(),
            id: EntityId::new(u32::MAX).unwrap(),
            player_id: None,
            reloads: None,
            transform: Transform::default(),
            turrets: None,
        }
    }
}

impl Contact {
    /// Initializes all (private) fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        altitude: Altitude,
        damage: Ticks,
        entity_type: Option<EntityType>,
        guidance: Guidance,
        id: EntityId,
        player_id: Option<PlayerId>,
        reloads: Option<BitArray<ReloadsStorage>>,
        transform: Transform,
        turrets: Option<Arc<[Angle]>>,
    ) -> Self {
        Self {
            altitude,
            damage,
            entity_type,
            guidance,
            id,
            player_id,
            reloads,
            transform,
            turrets,
        }
    }

    /// Simulate delta_seconds passing, by updating guidance and kinematics. This is an approximation
    /// of how the corresponding entity behaves on the server.
    pub fn simulate(&mut self, delta_seconds: f32) {
        if let Some(entity_type) = self.entity_type() {
            let guidance = *self.guidance();
            let max_speed = match entity_type.data().sub_kind {
                // Wait until risen to surface.
                EntitySubKind::Missile
                | EntitySubKind::Rocket
                | EntitySubKind::RocketTorpedo
                | EntitySubKind::Sam
                    if self.altitude().is_submerged() =>
                {
                    EntityData::SURFACING_PROJECTILE_SPEED_LIMIT
                }
                _ => f32::INFINITY,
            };

            self.transform_mut().apply_guidance(
                entity_type.data(),
                guidance,
                max_speed,
                delta_seconds,
            );
        }
        self.transform_mut().do_kinematics(delta_seconds);
    }

    /// Interpolates or snaps one contact's fields to another, assuming they share the same id.
    /// Optionally affects guidance, because that is more of an input, and is not subject to physics.
    pub fn interpolate_towards(
        &mut self,
        model: &Self,
        interpolate_guidance: bool,
        lerp: f32,
        delta_seconds: f32,
    ) {
        // Clamp to valid range once.
        let lerp = lerp.clamp(0.0, 1.0);

        assert_eq!(self.id, model.id);

        // Upgraded.
        let changed_type = self.entity_type != model.entity_type;
        self.entity_type = model.entity_type;

        self.altitude = self.altitude.lerp(model.altitude, lerp);
        self.damage = model.damage;
        self.player_id = model.player_id;
        self.reloads = model.reloads;
        if interpolate_guidance {
            self.guidance = model.guidance;
        }

        self.transform = Transform {
            position: self.transform.position.lerp(model.transform.position, lerp),
            direction: self
                .transform
                .direction
                .lerp(model.transform.direction, lerp),
            velocity: self.transform.velocity.lerp(model.transform.velocity, lerp),
        };

        if let Some((turrets, model_turrets)) = self
            .turrets
            .as_mut()
            .zip(model.turrets.as_ref())
            .filter(|_| !changed_type)
        {
            let turrets = make_mut_slice(turrets);
            let data: &'static EntityData = self.entity_type.unwrap().data();
            let turret_data = &*data.turrets;
            for ((v, m), t) in turrets
                .iter_mut()
                .zip(model_turrets.iter())
                .zip(turret_data)
            {
                let diff = *m - *v;

                // Don't let it get too far off.
                if diff.abs() > t.speed * Ticks::from_repr(2).to_secs() {
                    *v = *m;
                } else {
                    *v += diff.clamp_magnitude(t.speed * delta_seconds);
                }
            }
        } else {
            self.turrets = model.turrets.clone()
        }
    }

    /// Applies a control message to a contact (can be used to predict its outcome).
    pub fn predict_guidance(&mut self, guidance: &Guidance) {
        self.guidance = *guidance;
    }

    // TODO handle predictive physics in common.
    #[inline]
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
}

pub static ANGLE_ARRAY_ZERO: [Angle; 0] = [Angle::ZERO; 0];
pub static RELOADS_ARRAY_ZERO: BitArray<ReloadsStorage> = BitArray::ZERO;

impl ContactTrait for Contact {
    fn altitude(&self) -> Altitude {
        self.altitude
    }

    #[inline]
    fn damage(&self) -> Ticks {
        self.damage
    }

    #[inline]
    fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }

    #[inline]
    fn guidance(&self) -> &Guidance {
        &self.guidance
    }

    #[inline]
    fn id(&self) -> EntityId {
        self.id
    }

    #[inline]
    fn player_id(&self) -> Option<PlayerId> {
        self.player_id
    }

    #[inline]
    fn reloads(&self) -> &BitSlice<ReloadsStorage> {
        self.reloads.as_ref().map_or(&RELOADS_ARRAY_ZERO, |a| {
            &a.as_bitslice()[0..self.entity_type.unwrap().data().armaments.len()]
        })
    }

    #[inline]
    fn reloads_known(&self) -> bool {
        self.reloads.is_some()
    }

    #[inline]
    fn transform(&self) -> &Transform {
        &self.transform
    }

    #[inline]
    fn turrets(&self) -> &[Angle] {
        self.turrets
            .as_ref()
            .map_or(&ANGLE_ARRAY_ZERO, |a| a.as_ref())
    }

    #[inline]
    fn turrets_known(&self) -> bool {
        self.turrets.is_some()
    }
}

/// Useful for efficiently serializing contact.
struct ContactHeader {
    has_vel: bool,
    has_alt: bool,
    has_dir_target: bool,
    has_vel_target: bool,
    has_damage: bool,
    has_type: bool,
    has_player_id: bool,
    has_reloads: bool,
}

impl ContactHeader {
    fn as_bits(&self) -> u8 {
        let bools = [
            self.has_vel,
            self.has_alt,
            self.has_dir_target,
            self.has_vel_target,
            self.has_damage,
            self.has_type,
            self.has_player_id,
            self.has_reloads,
        ];

        let mut bits: u8 = 0;
        for (i, &bit) in bools.iter().enumerate() {
            bits |= (bit as u8) << i;
        }
        bits
    }

    fn from_bits(bits: u8) -> Self {
        let mut bools = [false; 8];
        for (i, bit) in bools.iter_mut().enumerate() {
            *bit = bits & (1 << i) != 0
        }

        let [has_vel, has_alt, has_dir_target, has_vel_target, has_damage, has_type, has_player_id, has_reloads] =
            bools;

        let header = Self {
            has_vel,
            has_alt,
            has_dir_target,
            has_vel_target,
            has_damage,
            has_type,
            has_player_id,
            has_reloads,
        };
        debug_assert_eq!(bits, header.as_bits());
        header
    }

    fn tuple_len(&self) -> usize {
        12 - self.as_bits().count_zeros() as usize
    }
}

impl Serialize for Contact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ContactSerializer::serialize_to(self, serializer)
    }
}

struct ContactSerializer<'a> {
    c: &'a Contact,
    h: ContactHeader,
}

impl<'a> ContactSerializer<'a> {
    fn serialize_to<S>(c: &'a Contact, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Assert that all boats have turrets.
        assert_eq!(c.turrets.is_some(), c.is_boat());

        // Assert that, if reloads are known, so is entity type.
        debug_assert!(!(c.reloads.is_some() && c.entity_type.is_none()), "{:?}", c);

        let s = Self {
            c,
            h: ContactHeader {
                has_type: c.entity_type.is_some(),
                has_vel: c.transform.velocity != Velocity::ZERO,
                has_alt: c.altitude != Altitude::ZERO,
                has_dir_target: c.guidance.direction_target != c.transform.direction,
                has_vel_target: c.guidance.velocity_target != c.transform.velocity,
                has_damage: c.damage != Ticks::ZERO,
                has_player_id: c.player_id.is_some(),
                has_reloads: c.reloads.is_some(),
            },
        };

        // Contains bits, variable tuple.
        let mut header = serializer.serialize_tuple(2)?;
        header.serialize_element(&s.h.as_bits())?;
        header.serialize_element(&s)?;
        header.end()
    }
}

impl<'a> Serialize for ContactSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(self.h.tuple_len())?;

        // 3 required elements.
        tup.serialize_element(&self.c.id)?;
        tup.serialize_element(&self.c.transform.position)?;
        tup.serialize_element(&self.c.transform.direction)?;

        // 8 optional elements.
        if self.h.has_vel {
            tup.serialize_element(&self.c.transform.velocity)?;
        }
        if self.h.has_alt {
            tup.serialize_element(&self.c.altitude)?;
        }
        if self.h.has_dir_target {
            tup.serialize_element(&self.c.guidance.direction_target)?;
        }
        if self.h.has_vel_target {
            tup.serialize_element(&self.c.guidance.velocity_target)?;
        }
        if self.h.has_damage {
            tup.serialize_element(&self.c.damage)?;
        }
        if self.h.has_type {
            tup.serialize_element(&self.c.entity_type.unwrap())?;
        }
        if self.h.has_player_id {
            tup.serialize_element(&self.c.player_id)?;
        }
        if self.h.has_reloads {
            // Round bits up to bytes.
            let size: usize = (self.c.entity_type.unwrap().data().armaments.len() + 7) / 8;
            let reloads = &self.c.reloads.unwrap().data.to_le_bytes()[..size];
            if reloads.is_empty() {
                tup.serialize_element(&())?;
            } else {
                tup.serialize_element(&ByteSerializer::new(reloads))?;
            }
        }

        // 1 option or unit element.
        if self.c.is_boat() {
            let turrets = self.c.turrets.as_ref().unwrap();
            if turrets.is_empty() {
                tup.serialize_element(&())?;
            } else {
                tup.serialize_element(&KnownSizeSerializer::new(turrets))?;
            }
        } else {
            tup.serialize_element(&())?;
        }

        tup.end()
    }
}

/// Serializes a slice of bytes without length (known size).
struct ByteSerializer<'a> {
    items: &'a [u8],
}

impl<'a> ByteSerializer<'a> {
    fn new(items: &'a [u8]) -> Self {
        debug_assert!(!items.is_empty());
        Self { items }
    }
}

impl<'a> Serialize for ByteSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(self.items.len())?;
        for item in self.items {
            tup.serialize_element(item)?;
        }
        tup.end()
    }
}

struct KnownSizeSerializer<'a, T> {
    items: &'a [T],
}

impl<'a, T> KnownSizeSerializer<'a, T> {
    fn new(items: &'a [T]) -> Self {
        debug_assert!(!items.is_empty());
        Self { items }
    }
}

impl<'a, T: Serialize> Serialize for KnownSizeSerializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(self.items.len())?;
        for item in self.items {
            tup.serialize_element(item)?;
        }
        tup.end()
    }
}

impl<'de> Deserialize<'de> for Contact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        pub struct HeaderVisitor;

        impl<'de> Visitor<'de> for HeaderVisitor {
            type Value = Contact;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a header tuple")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let h = ContactHeader::from_bits(seq.next_element()?.unwrap());

                let mut contact = Contact::default();
                let d = ContactDeserializer { h, c: &mut contact };

                seq.next_element_seed(d)?;

                Ok(contact)
            }
        }

        deserializer.deserialize_tuple(2, HeaderVisitor)
    }
}

struct ContactDeserializer<'a> {
    c: &'a mut Contact,
    h: ContactHeader,
}

impl<'de, 'a> DeserializeSeed<'de> for ContactDeserializer<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(self.h.tuple_len(), self)
    }
}

impl<'de, 'a> Visitor<'de> for ContactDeserializer<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a contact tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // 3 required elements.
        self.c.id = seq.next_element()?.unwrap();
        self.c.transform.position = seq.next_element()?.unwrap();
        self.c.transform.direction = seq.next_element()?.unwrap();

        // 8 optional elements.
        if self.h.has_vel {
            self.c.transform.velocity = seq.next_element()?.unwrap();
        }
        if self.h.has_alt {
            self.c.altitude = seq.next_element()?.unwrap();
        }
        if self.h.has_dir_target {
            self.c.guidance.direction_target = seq.next_element()?.unwrap();
        } else {
            self.c.guidance.direction_target = self.c.transform.direction;
        }
        if self.h.has_vel_target {
            self.c.guidance.velocity_target = seq.next_element()?.unwrap();
        } else {
            self.c.guidance.velocity_target = self.c.transform.velocity
        }
        if self.h.has_damage {
            self.c.damage = seq.next_element()?.unwrap();
        }
        if self.h.has_type {
            self.c.entity_type = Some(seq.next_element()?.unwrap());
        }
        if self.h.has_player_id {
            self.c.player_id = seq.next_element()?.unwrap();
        }
        if self.h.has_reloads {
            // Must be after type is assigned.
            // Round bits up to bytes.
            let size: usize = (self.c.entity_type.unwrap().data().armaments.len() + 7) / 8;
            if size == 0 {
                let _: () = seq.next_element()?.unwrap();
                self.c.reloads = Some(BitArray::ZERO)
            } else {
                let bytes = seq
                    .next_element_seed(
                        ByteDeserializer::<{ ReloadsStorage::BITS as usize / 8 }>::new(size),
                    )?
                    .unwrap();
                self.c.reloads = Some(BitArray::from(ReloadsStorage::from_le_bytes(bytes)));
            }
        }

        // 1 option or unit element.
        if self.c.is_boat() {
            // Must be after type is assigend.
            let size = self.c.entity_type.unwrap().data().turrets.len();
            if size == 0 {
                let _: () = seq.next_element()?.unwrap();
                self.c.turrets = Some(Arc::new([]))
            } else {
                self.c.turrets = Some(
                    seq.next_element_seed(KnownSizeDeserializer::new(size))?
                        .unwrap(),
                );
            }
        } else {
            let _: () = seq.next_element()?.unwrap();
        }

        Ok(())
    }
}

struct ByteDeserializer<const MAX: usize> {
    items: [u8; MAX],
    size: usize,
}

impl<const MAX: usize> ByteDeserializer<MAX> {
    fn new(size: usize) -> Self {
        debug_assert!(size != 0);
        Self {
            items: [0; MAX],
            size,
        }
    }
}

impl<'de, const MAX: usize> DeserializeSeed<'de> for ByteDeserializer<MAX> {
    type Value = [u8; MAX];

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(self.size, self)
    }
}

impl<'de, const MAX: usize> Visitor<'de> for ByteDeserializer<MAX> {
    type Value = [u8; MAX];

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an array of bytes")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut i = 0;
        while let Some(v) = seq.next_element()? {
            self.items[i] = v;
            i += 1;
        }
        Ok(self.items)
    }
}

struct KnownSizeDeserializer<T> {
    items: Arc<[T]>,
}

impl<'de, T: Deserialize<'de> + Default> KnownSizeDeserializer<T> {
    fn new(size: usize) -> Self {
        debug_assert!(size != 0);
        Self {
            items: repeat_with(T::default).take(size).collect(),
        }
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for KnownSizeDeserializer<T> {
    type Value = Arc<[T]>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ItemVisitor<I>(Arc<[I]>);

        impl<'de, I: Deserialize<'de>> Visitor<'de> for ItemVisitor<I> {
            type Value = Arc<[I]>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("an array")
            }

            fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let items = Arc::get_mut(&mut self.0).unwrap();
                let mut i = 0;
                while let Some(v) = seq.next_element()? {
                    items[i] = v;
                    i += 1;
                }
                Ok(self.0)
            }
        }

        deserializer.deserialize_tuple(self.items.len(), ItemVisitor(self.items))
    }
}
