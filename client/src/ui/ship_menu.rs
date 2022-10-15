// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::sprite::Sprite;
use common::entity::{EntitySubKind, EntityType};
use common::util::score_to_level;
use common::world::outside_strict_area;
use glam::Vec2;
use std::collections::HashMap;
use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{
    classes, html, html_nested, use_state, use_state_eq, Callback, Children, Html, Properties,
};
use yew_frontend::component::section::{Section, SectionArrow};
use yew_frontend::translation::{t, Translation};
use yew_icons::{Icon, IconId};

#[derive(Properties, PartialEq)]
pub struct ShipMenuProps {
    /// If some, upgrading. Otherwise, spawning.
    pub entity: Option<(EntityType, Vec2)>,
    pub score: u32,
    pub onclick: Callback<EntityType>,
    #[prop_or(true)]
    pub open: bool,
    #[prop_or(true)]
    pub closable: bool,
    #[prop_or_default]
    pub children: Children,
}

#[styled_component(ShipMenu)]
pub fn ship_menu(props: &ShipMenuProps) -> Html {
    let ships_style = css!(
        r#"
        display: grid;
        grid-gap: 1.5rem 1.5rem;
        grid-template-columns: repeat(1, 1fr);
        margin: auto;
        padding-top: 1.5rem;
        user-select: none;
        width: min-content;
        -webkit-user-drag: none;

        @media (min-width: 600px) and (max-height: 500px) {
            grid-template-columns: repeat(2, 1fr);
        }
    "#
    );

    let columns_css = css!(
        r#"
        @media (min-width: 1000px) {
            grid-template-columns: repeat(2, 1fr);
        }
        "#
    );

    let greyed_out_style = css!(
        r#"
        opacity: 0.6;
    "#
    );

    let entity_type = props.entity.map(|(entity_type, _)| entity_type);
    let min_level = entity_type
        .map(|entity_type| entity_type.data().level + 1)
        .unwrap_or(1);
    let max_level = score_to_level(props.score);
    let level = use_state_eq(|| max_level);
    let locker = use_state(Locker::default);

    if min_level > max_level {
        // There are no choices now. This is possible for upgrade menu, but not spawn menu.
        debug_assert!(entity_type.is_some(), "no choices to spawn");
        return html! { {props.children.clone().into_iter().collect::<Html>()} };
    } else {
        level.set(level.clamp(min_level, max_level));
    }

    let select_factory =
        |entity_type: EntityType| -> Result<Callback<MouseEvent>, (IconId, &'static str)> {
            if let Some(lock_icon) =
                locker.lock_icon(entity_type, props.entity.map(|(_, position)| position))
            {
                Err(lock_icon)
            } else {
                Ok(props.onclick.reform(move |_| entity_type))
            }
        };

    let increment_level_factory = |increment: i8| -> SectionArrow {
        let new = level.saturating_add_signed(increment);
        SectionArrow::sometimes((min_level..=max_level).contains(&new).then(|| {
            let level = level.clone();
            Callback::from(move |_: MouseEvent| {
                level.set(new);
            })
        }))
    };

    let attempt_to_unlock_factory = |entity_type: EntityType| -> Callback<MouseEvent> {
        let locker = locker.clone();
        Callback::from(move |_: MouseEvent| {
            locker.set(locker.attempt_to_unlock(entity_type));
        })
    };

    let t = t();
    let (name, ships) = if let Some(entity_type) = entity_type {
        (
            t.upgrade_to_level_label(*level as u32),
            entity_type
                .upgrade_options(props.score, false)
                .filter(|entity_type| entity_type.data().level == *level)
                .collect::<Vec<_>>(),
        )
    } else {
        (
            t.respawn_as_level_label(*level as u32),
            EntityType::spawn_options(props.score, false)
                .filter(|entity_type| entity_type.data().level == *level)
                .collect::<Vec<_>>(),
        )
    };

    html! {
        <Section {name} left_arrow={increment_level_factory(-1)} right_arrow={increment_level_factory(1)}>
            <div class={classes!(ships_style, (ships.len() > 3).then(|| columns_css.clone()))}>
                {ships.into_iter().map(|entity_type| {
                    let mut onclick: Option<Callback<MouseEvent>> = None;
                    let mut icon_title: Option<(IconId, &'static str)> = None;
                    match select_factory(entity_type) {
                        Ok(s) => onclick = Some(s),
                        Err(it) => icon_title = Some(it),
                    };

                    html_nested!{
                        <Sprite
                            {entity_type}
                            {onclick}
                            image_class={classes!(icon_title.is_some().then(|| greyed_out_style.clone()))}
                            >
                            if let Some((icon_id, title)) = icon_title {
                                <Icon {icon_id} {title} onclick={attempt_to_unlock_factory(entity_type)}/>
                            }
                        </Sprite>
                    }
                }).collect::<Html>()}
            </div>
        </Section>
    }
}

/// Some ships are foot-guns for new players, so restrict them for a bit (although allow the player
/// to override the restriction).
#[derive(Clone, Default)]
struct Locker {
    attempts: HashMap<EntityType, u8>,
}

impl Locker {
    fn attempt_to_unlock(&self, entity_type: EntityType) -> Self {
        let mut clone = self.clone();
        let attempts = clone.attempts.entry(entity_type).or_default();
        if *attempts < Self::attempts_required(entity_type) {
            *attempts += 1;
        }
        clone
    }

    fn lock_icon(
        &self,
        entity_type: EntityType,
        position: Option<Vec2>,
    ) -> Option<(IconId, &'static str)> {
        let attempts = self.attempts.get(&entity_type).cloned().unwrap_or_default();
        let attempts_required = Self::attempts_required(entity_type);
        if position
            .map(|p| outside_strict_area(entity_type, p))
            .unwrap_or(false)
        {
            Some((
                IconId::BootstrapSnow2,
                "Cannot choose this ship in this area",
            ))
        } else if attempts >= attempts_required
            || client_util::joined::minutes_since_u8() >= Self::minutes_required(entity_type)
        {
            None
        } else if attempts + 1 == attempts_required {
            Some((
                IconId::BootstrapUnlockFill,
                "New players are not advised to choose this ship",
            ))
        } else {
            Some((
                IconId::BootstrapLockFill,
                "New players are not advised to choose this ship",
            ))
        }
    }

    fn minutes_required(entity_type: EntityType) -> u8 {
        match entity_type.data().sub_kind {
            EntitySubKind::Dredger => 15,
            EntitySubKind::Minelayer => 30,
            EntitySubKind::Icebreaker => 45,
            EntitySubKind::Tanker => 60,
            _ => 0,
        }
    }

    fn attempts_required(entity_type: EntityType) -> u8 {
        if Self::minutes_required(entity_type) > 0 {
            5
        } else {
            0
        }
    }
}

/*
import Locked from "svelte-bootstrap-icons/lib/LockFill";
import Restricted from "svelte-bootstrap-icons/lib/Snow2";
import Unlocked from "svelte-bootstrap-icons/lib/UnlockFill";
import {onMount} from 'svelte';

let forcedUnlocks = {};
const FORCE_UNLOCKS = 5;

$: level = clamp(level || minLevel, minLevel, maxLevel);
$: ships = availableShips(level, type);
$: columns = ships.length > 3;

onMount(() => {
level = maxLevel;
});

// $: console.log(`min=${minLevel}, max=${maxLevel}, level=${level}`);

function handleSelectShip(shipType) {
if (onSelectShip && !(restricted(shipType, restrictions) || locked(shipType, forcedUnlocks))) {
onSelectShip(shipType);
}
}

function incrementIndex(value) {
level = clamp(level + value, minLevel, maxLevel);
}

// Some ships are difficult/confusing. Lock them until the player has a bit of experience with the game.
function locked(type, forcedUnlocks) {
const data = entityData[type];
const minutesPlayed = (Date.now() - (storage.join || 0)) / (60 * 1000);
return (forcedUnlocks[type] || 0) < FORCE_UNLOCKS && minutesPlayed < ({dredger: 15, minelayer: 30, icebreaker: 45, tanker: 60}[data.subkind] || -1);
}

function restricted(type, restrictions) {
return restrictions ? restrictions.includes(type) : false;
}

// Protest locking of ships (click the lock x times to unlock manually).
function unlockShip(type) {
forcedUnlocks[type] = (forcedUnlocks[type] || 0) + 1;
}
</script>

<Section disableLeftArrow={level == minLevel} disableRightArrow={level == maxLevel} headerAlign='center' name={name} bind:open onLeftArrow={() => incrementIndex(-1)} onRightArrow={() => incrementIndex(1)} {closable}>
<div class="ships" class:columns={ships.length > 3}>
{#each ships as shipType}
<Sprite
title={`${entityData[shipType].label} (${summarizeType($t, shipType)})`}
consumed={restricted(shipType, restrictions) || locked(shipType, forcedUnlocks)}
icon={restricted(shipType, restrictions) ? Restricted : locked(shipType, forcedUnlocks) ? ((forcedUnlocks[shipType] || 0) < FORCE_UNLOCKS - 1 ? Locked : Unlocked) : null}
iconTitle={restricted(shipType, restrictions) ? 'Cannot choose this ship in this area' : 'New players are not advised to choose this ship'}
onIconClick={() => unlockShip(shipType)}
on:click={() => handleSelectShip(shipType)}
name={shipType}
/>
{/each}
</div>
</Section>

<style>
div.ships {
display: grid;
grid-gap: 1.5rem 1.5rem;
grid-template-columns: repeat(1, 1fr);
margin: auto;
padding-top: 1.5rem;
user-select: none;
width: min-content;
-webkit-user-drag: none;
}

@media(min-width: 1000px) {
div.ships.columns {
grid-template-columns: repeat(2, 1fr);
}
}

@media(min-width: 600px) and (max-height: 500px) {
div.ships {
grid-template-columns: repeat(2, 1fr);
}
}
</style>
 */
