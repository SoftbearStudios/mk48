// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::ui::sprite::Sprite;
use common::entity::{EntitySubKind, EntityType};
use common::util::score_to_level;
use common::world::outside_strict_area;
use kodiak_client::glam::Vec2;
use kodiak_client::{
    translate, use_core_state, use_translator, GameClient, Position, RankNumber, Section,
    SectionArrow, Translator,
};
use std::collections::HashMap;
use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{
    classes, html, html_nested, use_state, use_state_eq, AttrValue, Callback, Children, Html,
    Properties,
};
use yew_icons::{Icon, IconId};

#[derive(Properties, PartialEq)]
pub struct ShipMenuProps {
    #[prop_or(None)]
    pub position: Option<Position>,
    #[prop_or(None)]
    pub style: Option<AttrValue>,
    /// If some, upgrading. Otherwise, spawning.
    #[prop_or(None)]
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
        grid-gap: 1rem 1rem;
        grid-template-columns: repeat(1, 1fr);
        margin: auto;
        margin-top: 1rem;
        user-select: none;
        width: min-content;
        -webkit-user-drag: none;
    "#
    );

    let columns_css = css!(
        r#"
        @media (min-width: 600px) {
            grid-template-columns: repeat(2, 1fr);
        }
        "#
    );

    let greyed_out_style = css!(
        r#"
        opacity: 0.6;
    "#
    );

    let sprite_scale = css!(
        r#"
        @media (max-width: 1400px) {
            zoom: 0.8;
        }

        @media (max-width: 1000px) {
            zoom: 0.7;
        }

        @media (max-width: 600px) {
            zoom: 0.6;
        }
        "#
    );

    let entity_type = props.entity.map(|(entity_type, _)| entity_type);
    let min_level = entity_type
        .map(|entity_type| entity_type.data().level + 1)
        .unwrap_or(1);
    let max_level = score_to_level(props.score);
    let level = use_state_eq(|| max_level);
    let locker = use_state(Locker::default);
    let t = use_translator();
    //let rewarded_ad = use_rewarded_ad();

    if min_level > max_level {
        // There are no choices now. This is possible for upgrade menu, but not spawn menu.
        debug_assert!(entity_type.is_some(), "no choices to spawn");

        // WARNING: Early return means no more hooks later on.
        return html! { {props.children.clone().into_iter().collect::<Html>()} };
    } else {
        level.set(level.clamp(min_level, max_level));
    }

    let rank = use_core_state().rank().flatten();

    let select_factory =
        |entity_type: EntityType| -> Result<Callback<MouseEvent>, (IconId, String)> {
            if let Some(lock_icon) = locker.lock_icon(
                entity_type,
                props.entity.map(|(_, position)| position),
                &t,
                rank,
                /* &rewarded_ad, */
            ) {
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
            locker.set(locker.attempt_to_unlock(entity_type, rank));
        })
    };

    let allow_npc = rank >= Some(RankNumber::Rank5);
    let (id, name, ships) = if let Some(entity_type) = entity_type {
        (
            "upgrade",
            t.upgrade_to_level_label(*level as u32),
            entity_type
                .upgrade_options(props.score, allow_npc)
                .filter(|entity_type| entity_type.data().level == *level)
                .collect::<Vec<_>>(),
        )
    } else {
        (
            "respawn",
            t.respawn_as_level_label(*level as u32),
            EntityType::spawn_options(props.score, allow_npc)
                .filter(|entity_type| entity_type.data().level == *level)
                .collect::<Vec<_>>(),
        )
    };

    html! {
        <Section
            {id}
            {name}
            position={props.position}
            style={props.style.clone()}
            left_arrow={increment_level_factory(-1)}
            right_arrow={increment_level_factory(1)}
            closable={props.closable}
        >
            <div class={classes!(ships_style, (ships.len() > 3).then(|| columns_css.clone()))}>
                {ships.into_iter().map(|entity_type| {
                    let mut onclick: Option<Callback<MouseEvent>> = None;
                    let mut icon_title: Option<(IconId, String)> = None;
                    match select_factory(entity_type) {
                        Ok(s) => onclick = Some(s),
                        Err(it) => icon_title = Some(it),
                    };

                    html_nested!{
                        <Sprite
                            {entity_type}
                            {onclick}
                            class={sprite_scale.clone()}
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
    fn attempt_to_unlock(&self, entity_type: EntityType, rank: Option<RankNumber>) -> Self {
        let mut clone = self.clone();
        let attempts = clone.attempts.entry(entity_type).or_default();
        if *attempts < Self::attempts_required(entity_type, rank) {
            *attempts += 1;
        }
        clone
    }

    fn lock_icon(
        &self,
        entity_type: EntityType,
        position: Option<Vec2>,
        t: &Translator,
        rank: Option<RankNumber>,
        /* rewarded_ad: &RewardedAd, */
    ) -> Option<(IconId, String)> {
        let attempts = self.attempts.get(&entity_type).cloned().unwrap_or_default();
        let rank_required = Self::rank_required(entity_type);
        let attempts_required = Self::attempts_required(entity_type, rank);
        if position
            .map(|p| outside_strict_area(entity_type, p))
            .unwrap_or(false)
        {
            // Vous ne pouvez pas choisir ce navire dans cette zone
            Some((
                IconId::BootstrapSnow2,
                translate!(t, "Cannot choose this ship in this area"),
            ))
        } else if attempts < attempts_required {
            Some((
                if attempts + 1 < attempts_required {
                    IconId::BootstrapLockFill
                } else {
                    IconId::BootstrapUnlockFill
                },
                if entity_type.data().sub_kind == EntitySubKind::Pirate {
                    translate!(t, "This ship is not combat-effective")
                } else {
                    t.earn_rank_to_unlock(Mk48Game::translate_rank_number(
                        t,
                        rank_required.unwrap_or(RankNumber::Rank1),
                    ))
                },
            ))
        }
        /* else if matches!(entity_type, EntityType::Skjold)
            && !matches!(
                rewarded_ad,
                RewardedAd::Unavailable | RewardedAd::Watched { .. }
            )
        {
            Some((
                IconId::OcticonsVideo16,
                "Watch video ad on the splash screen or respawn screen to unlock this ship",
            ))
        } */
        else {
            None
        }
    }

    fn rank_required(entity_type: EntityType) -> Option<RankNumber> {
        match entity_type {
            EntityType::TypeViic | EntityType::Oberon | EntityType::Golf => Some(RankNumber::Rank1),
            EntityType::Dredger
            | EntityType::Lublin
            | EntityType::TerryFox
            | EntityType::Tanker => Some(RankNumber::Rank2),
            _ => None,
        }
    }

    fn attempts_required(entity_type: EntityType, rank: Option<RankNumber>) -> u8 {
        if entity_type.data().sub_kind == EntitySubKind::Pirate {
            1
        } else if Self::rank_required(entity_type) > rank {
            5
        } else {
            0
        }
    }
}
