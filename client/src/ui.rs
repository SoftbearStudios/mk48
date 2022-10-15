// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::translation::Mk48Translation;
use crate::ui::about_dialog::AboutDialog;
use crate::ui::changelog_dialog::ChangelogDialog;
use crate::ui::help_dialog::HelpDialog;
use crate::ui::hint::Hint;
pub use crate::ui::instructions::InstructionsProps;
use crate::ui::levels_dialog::LevelsDialog;
use crate::ui::logo::logo;
use crate::ui::respawn_overlay::RespawnOverlay;
use crate::ui::settings_dialog::SettingsDialog;
use crate::ui::ship_controls::ShipControls;
use crate::ui::ships_dialog::ShipsDialog;
use crate::ui::status_overlay::StatusOverlay;
use crate::ui::upgrade_overlay::UpgradeOverlay;
use client_util::context::Context;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::EntityType;
use common::velocity::Velocity;
use core_protocol::id::{LanguageId, TeamId};
use core_protocol::name::PlayerAlias;
use engine_macros::SmolRoutable;
use glam::Vec2;
use std::collections::HashMap;
use stylist::yew::styled_component;
use yew::prelude::*;
use yew_frontend::component::discord_icon::DiscordIcon;
use yew_frontend::component::invitation_icon::InvitationIcon;
use yew_frontend::component::invitation_link::InvitationLink;
use yew_frontend::component::language_menu::LanguageMenu;
use yew_frontend::component::positioner::{Align, Flex, Position, Positioner};
use yew_frontend::component::privacy_link::PrivacyLink;
use yew_frontend::component::route_link::RouteLink;
use yew_frontend::component::settings_icon::SettingsIcon;
use yew_frontend::component::terms_link::TermsLink;
use yew_frontend::component::volume_icon::VolumeIcon;
use yew_frontend::component::x_button::XButton;
use yew_frontend::component::zoom_icon::ZoomIcon;
use yew_frontend::frontend::Ctw;
use yew_frontend::frontend::{Gctw, PropertiesWrapper};
use yew_frontend::overlay::chat::ChatOverlay;
use yew_frontend::overlay::leaderboard::LeaderboardOverlay;
use yew_frontend::overlay::spawn::SpawnOverlay;
use yew_frontend::overlay::team::TeamsOverlay;
use yew_frontend::translation::{t, Translation};
use yew_router::{Routable, Switch};

mod about_dialog;
mod changelog_dialog;
mod help_dialog;
mod hint;
mod instructions;
mod levels_dialog;
mod logo;
mod respawn_overlay;
mod settings_dialog;
mod ship_controls;
mod ship_menu;
mod ships_dialog;
mod sprite;
mod status_overlay;
mod upgrade_overlay;

#[styled_component(Mk48Ui)]
pub fn mk48_ui(props: &PropertiesWrapper<UiProps>) -> Html {
    let cinematic_style = css!(
        r#"
        transition: opacity 0.25s;

        :not(:hover) {
		    opacity: 0;
	    }
    "#
    );

    let gctw = Gctw::<Mk48Game>::use_gctw();
    let on_play = gctw.send_ui_event_callback.reform(|alias| UiEvent::Spawn {
        alias,
        entity_type: EntityType::GFive,
    });

    let margin = "0.75rem";
    let status = props.status.clone();
    let outbound_enabled = Ctw::use_outbound_enabled();

    html! {
        <>
            if let UiStatus::Playing(playing) = status {
                <div class={classes!(gctw.settings_cache.cinematic.then_some(cinematic_style))}>
                    <Positioner position={Position::BottomMiddle{margin}}>
                        <StatusOverlay
                            status={playing.clone()}
                            score={props.score}
                            fps={gctw.settings_cache.fps_shown.then_some(props.fps)}
                        />
                    </Positioner>
                    <Positioner position={Position::TopMiddle{margin}}>
                        <UpgradeOverlay status={playing.clone()} score={props.score}/>
                    </Positioner>
                    <Positioner position={Position::BottomLeft{margin}}>
                        <ShipControls status={playing.clone()}/>
                    </Positioner>
                    <Positioner position={Position::CenterRight{margin}} flex={Flex::Column}>
                        <InvitationIcon/>
                        <ZoomIcon amount={-4}/>
                        <ZoomIcon amount={4}/>
                        <VolumeIcon/>
                        <SettingsIcon<Mk48Route> route={Mk48Route::Settings}/>
                        <LanguageMenu/>
                    </Positioner>
                    <Positioner position={Position::TopLeft{margin}} max_width="25%">
                        <TeamsOverlay
                            team_proximity={playing.team_proximity.clone()}
                            label={LanguageId::team_fleet_label as fn(LanguageId) -> &'static str}
                            name_placeholder={LanguageId::team_fleet_name_placeholder as fn(LanguageId) -> &'static str}
                        />
                    </Positioner>
                    <Positioner position={Position::TopRight{margin}} max_width="25%">
                        <LeaderboardOverlay/>
                    </Positioner>
                    <Positioner position={Position::BottomRight{margin}} align={Align::Left} max_width="25%">
                        <ChatOverlay label={LanguageId::chat_radio_label as fn(LanguageId) -> &'static str}/>
                    </Positioner>
                </div>
                if !gctw.settings_cache.cinematic {
                    <Hint entity_type={playing.entity_type}/>
                }
            } else if let UiStatus::Respawning(respawning) = status {
                <RespawnOverlay status={respawning} score={props.score}/>
                <Positioner position={Position::TopRight{margin}} max_width="25%">
                    <XButton onclick={gctw.send_ui_event_callback.reform(|_| UiEvent::OverrideRespawn)}/>
                </Positioner>
            } else {
                <SpawnOverlay {on_play}>
                    {logo()}
                </SpawnOverlay>
                <Positioner position={Position::TopRight{margin}} flex={Flex::Row}>
                    <LanguageMenu/>
                </Positioner>
            }
            if !matches!(props.status, UiStatus::Playing(_)) {
                <Positioner position={Position::BottomLeft{margin}}>
                    <InvitationLink/>
                </Positioner>
                <Positioner position={Position::BottomMiddle{margin}} flex={Flex::Row}>
                    <RouteLink<Mk48Route> route={Mk48Route::Help}>{t().help_hint()}</RouteLink<Mk48Route>>
                    <RouteLink<Mk48Route> route={Mk48Route::About}>{t().about_hint()}</RouteLink<Mk48Route>>
                    <PrivacyLink/>
                    <TermsLink/>
                </Positioner>
                if outbound_enabled {
                    <Positioner position={Position::BottomRight{margin}} flex={Flex::Row}>
                        <DiscordIcon/>
                    </Positioner>
                }
            }
            <div>
                <Switch<Mk48Route> render={Switch::render(switch)}/>
            </div>
        </>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, SmolRoutable)]
pub enum Mk48Route {
    #[at("/about/")]
    About,
    #[at("/changelog/")]
    Changelog,
    #[at("/help/")]
    Help,
    #[at("/ships/")]
    Ships,
    #[at("/levels/")]
    Levels,
    #[at("/settings/")]
    Settings,
    #[not_found]
    #[at("/")]
    Home,
}

/// State of UI inputs.
pub struct UiState {
    pub active: bool,
    pub submerge: bool,
    pub armament: Option<EntityType>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active: true,
            submerge: false,
            armament: None,
        }
    }
}

pub enum UiEvent {
    Spawn {
        alias: PlayerAlias,
        entity_type: EntityType,
    },
    Respawn(EntityType),
    Upgrade(EntityType),
    /// Sensors active.
    Active(bool),
    Submerge(bool),
    Armament(Option<EntityType>),
    /// Go from respawning to spawning.
    #[allow(unused)]
    OverrideRespawn,
}

#[derive(PartialEq, Clone, Default)]
pub struct UiProps {
    pub fps: f32,
    pub score: u32,
    pub status: UiStatus,
}

/// Mutually exclusive statuses.
#[derive(Default, PartialEq, Clone)]
pub enum UiStatus {
    #[default]
    Spawning,
    Playing(UiStatusPlaying),
    Respawning(UiStatusRespawning),
}

#[derive(PartialEq, Clone)]
pub struct UiStatusPlaying {
    pub entity_type: EntityType,
    pub velocity: Velocity,
    pub direction: Angle,
    pub position: Vec2,
    pub altitude: Altitude,
    pub submerge: bool,
    /// Active sensors.
    pub active: bool,
    pub instruction_props: InstructionsProps,
    pub armament: Option<EntityType>,
    pub armament_consumption: Box<[bool]>,
    pub team_proximity: HashMap<TeamId, f32>,
}

#[derive(PartialEq, Clone)]
pub struct UiStatusRespawning {
    pub death_reason: DeathReason,
}

impl Mk48Game {
    pub(crate) fn update_ui_props(&self, context: &mut Context<Self>, status: UiStatus) {
        let props = UiProps {
            fps: self.fps_counter.last_sample().unwrap_or(0.0),
            score: context.state.game.score,
            status,
        };

        context.set_ui_props(props);
    }
}

fn switch(routes: &Mk48Route) -> Html {
    match routes {
        Mk48Route::About => html! {
            <AboutDialog/>
        },
        Mk48Route::Changelog => html! {
            <ChangelogDialog/>
        },
        Mk48Route::Help => html! {
            <HelpDialog/>
        },
        Mk48Route::Ships => html! {
            <ShipsDialog/>
        },
        Mk48Route::Levels => html! {
            <LevelsDialog/>
        },
        Mk48Route::Settings => html! {
            <SettingsDialog/>
        },
        Mk48Route::Home => html! {},
    }
}
