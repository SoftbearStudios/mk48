// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::translation::Mk48Translation;
use crate::ui::about_dialog::AboutDialog;
use crate::ui::changelog_dialog::ChangelogDialog;
use crate::ui::help_dialog::HelpDialog;
use crate::ui::hint::Hint;
pub use crate::ui::instructions::InstructionStatus;
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
use yew_frontend::component::github_icon::GithubIcon;
use yew_frontend::component::invitation_icon::InvitationIcon;
use yew_frontend::component::invitation_link::InvitationLink;
use yew_frontend::component::language_menu::LanguageMenu;
use yew_frontend::component::positioner::{Flex, Position, Positioner};
use yew_frontend::component::privacy_link::PrivacyLink;
use yew_frontend::component::route_link::RouteLink;
use yew_frontend::component::settings_icon::SettingsIcon;
use yew_frontend::component::terms_link::TermsLink;
use yew_frontend::component::volume_icon::VolumeIcon;
use yew_frontend::component::x_button::XButton;
use yew_frontend::component::zoom_icon::ZoomIcon;
use yew_frontend::frontend::{use_gctw, use_outbound_enabled};
use yew_frontend::frontend::{use_rewarded_ad, PropertiesWrapper};
use yew_frontend::overlay::chat::ChatOverlay;
use yew_frontend::overlay::leaderboard::LeaderboardOverlay;
use yew_frontend::overlay::spawn::SpawnOverlay;
use yew_frontend::overlay::team::TeamOverlay;
use yew_frontend::translation::{use_translation, Translation};
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

    let gctw = use_gctw::<Mk48Game>();
    let t = use_translation();
    let on_play = gctw.send_ui_event_callback.reform(|alias| UiEvent::Spawn {
        alias,
        entity_type: EntityType::G5,
    });

    let margin = "0.75rem";
    let status = props.status.clone();
    let outbound_enabled = use_outbound_enabled();

    /*
       if (msg.includes('how')) {
           if (msg.includes('move')) {
               return 'If you are asking how you move, you click and hold (or right click) outside the inner ring of your ship to set your speed and direction (or use WASD)';
           }
           if (msg.includes('play')) {
               return '';
           }
           if (msg.includes('shoot') || msg.includes('use weapons') || msg.includes('fire')) {
               return '';
           }
       }
    */

    let shoot_hint = "First, select an available weapon. Then, click in the direction to fire. If you hold the click for too long, you won't shoot.";
    let hints = vec![
        ("Invitation links cannot currently be accepted by players that are already in game. They must send a join request instead.", vec!["/invite"]),
        ("If you are asking how you move, you click and hold to set your speed and direction (or use WASD).", vec!["how", "move"]),
        ("The controls are click and hold (or WASD) to move, click (or Space) to shoot.", vec!["how", "play"]),
        (shoot_hint, vec!["how", "shoot"]),
        (shoot_hint, vec!["how", "use weapons"]),
        (shoot_hint, vec!["how", "fire"])
    ];

    use yew_frontend::frontend::RewardedAd;
    use yew_icons::{Icon, IconId};
    let rewarded_ad = use_rewarded_ad();
    let rewarded_style = css!(
        r#"
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 0.5rem;
        background-color: #c0392b;
        border: 2px solid #e74c3c;
        border-radius: 0.5rem;
        color: white;
        padding: 0.25rem 0.5rem;
        font-size: 1rem;

        :disabled {
            filter: brightness(0.9);
        }
    "#
    );

    html! {
        <>
            if let UiStatus::Playing(playing) = status {
                <div class={classes!(gctw.settings_cache.cinematic.then_some(cinematic_style))}>
                    <Positioner id="status" position={Position::BottomMiddle{margin}} max_width="45%">
                        <StatusOverlay
                            status={playing.clone()}
                            score={props.score}
                            fps={gctw.settings_cache.fps_shown.then_some(props.fps)}
                        />
                    </Positioner>
                    <UpgradeOverlay
                        position={Position::TopMiddle{margin}}
                        status={playing.clone()}
                        score={props.score}
                    />
                    <ShipControls
                        position={Position::BottomLeft{margin}}
                        style="max-width:25%;"
                        status={playing.clone()}
                    />
                    <Positioner id="sidebar" position={Position::CenterRight{margin}} flex={Flex::Column}>
                        <InvitationIcon/>
                        <ZoomIcon amount={-4}/>
                        <ZoomIcon amount={4}/>
                        <VolumeIcon/>
                        <SettingsIcon<Mk48Route> route={Mk48Route::Settings}/>
                        <LanguageMenu/>
                    </Positioner>
                    <TeamOverlay
                        position={Position::TopLeft{margin}}
                        style="max-width:25%;"
                        team_proximity={playing.team_proximity.clone()}
                        label={LanguageId::team_fleet_label as fn(LanguageId) -> &'static str}
                        name_placeholder={LanguageId::team_fleet_name_placeholder as fn(LanguageId) -> &'static str}
                    />
                    <LeaderboardOverlay
                        position={Position::TopRight{margin}}
                        style="max-width:25%;"
                    />
                    <ChatOverlay
                        position={Position::BottomRight{margin}}
                        style="max-width:25%;"
                        {hints}
                        label={LanguageId::chat_radio_label as fn(LanguageId) -> &'static str}
                    />
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
                <Positioner id="back" position={Position::TopRight{margin}} flex={Flex::Row}>
                    <LanguageMenu/>
                </Positioner>
            }
            if !matches!(props.status, UiStatus::Playing(_)) {
                <Positioner id="invite" position={Position::BottomLeft{margin}}>
                    <InvitationLink/>
                </Positioner>
                <Positioner id="links" position={Position::BottomMiddle{margin}} flex={Flex::Row}>
                    <RouteLink<Mk48Route> route={Mk48Route::Help}>{t.help_hint()}</RouteLink<Mk48Route>>
                    <RouteLink<Mk48Route> route={Mk48Route::About}>{t.about_hint()}</RouteLink<Mk48Route>>
                    <PrivacyLink/>
                    <TermsLink/>
                </Positioner>
                if outbound_enabled {
                    <Positioner id="social" position={Position::BottomRight{margin}} flex={Flex::Row}>
                        <DiscordIcon/>
                        <GithubIcon repository_link={"https://github.com/SoftbearStudios/mk48"}/>
                    </Positioner>
                }
                if !matches!(rewarded_ad, RewardedAd::Unavailable) {
                    <button
                        id="rewarded"
                        onclick={if let RewardedAd::Available{request} = &rewarded_ad { Some(request.reform(|_| {})) } else { None }}
                        disabled={!matches!(rewarded_ad, RewardedAd::Available{..})}
                        style={Position::TopLeft{margin}.to_string()}
                        class={rewarded_style}
                    >
                        <Icon icon_id={IconId::OcticonsVideo16}/>
                        {t.rewarded_ad(&rewarded_ad)}
                    </button>
                }
            }
            <Switch<Mk48Route> render={switch}/>
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
    /// Sensors active.
    Active(bool),
    Armament(Option<EntityType>),
    GraphicsSettingsChanged,
    /// Go from respawning to spawning.
    #[allow(unused)]
    OverrideRespawn,
    Respawn(EntityType),
    Spawn {
        alias: PlayerAlias,
        entity_type: EntityType,
    },
    Submerge(bool),
    Upgrade(EntityType),
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
    pub instruction_status: InstructionStatus,
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

fn switch(routes: Mk48Route) -> Html {
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
