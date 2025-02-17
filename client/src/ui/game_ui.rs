// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::ui::about_dialog::AboutDialog;
use crate::ui::help_dialog::HelpDialog;
use crate::ui::hint::Hint;
use crate::ui::logo::logo;
use crate::ui::references_dialog::ReferencesDialog;
use crate::ui::respawn_overlay::RespawnOverlay;
use crate::ui::ships_dialog::ShipsDialog;
use crate::ui::status_overlay::StatusOverlay;
use crate::ui::team::TeamOverlay;
use crate::ui::upgrade_overlay::UpgradeOverlay;
use crate::ui::Mk48Phrases;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::EntityType;
use common::protocol::{TeamDto, TeamRequest};
use common::velocity::Velocity;
use kodiak_client::glam::Vec2;
use kodiak_client::yew_router::Routable;
use kodiak_client::{
    splash_links, splash_nexus_icons, splash_sign_in_link, splash_social_media, translate, use_ctw,
    use_gctw, ChatOverlay, ClientContext, GameClient, Instruction, LeaderboardOverlay, PathParam,
    PlayerAlias, PlayerId, Position, Positioner, PropertiesWrapper, RoutableExt, SmolRoutable,
    SpawnOverlay, SplashNexusIconsProps, SplashSocialMediaProps, TeamId, Translator,
};
use std::collections::HashMap;
use stylist::yew::styled_component;
use yew::prelude::*;

#[styled_component(Mk48Ui)]
pub fn mk48_ui(props: &PropertiesWrapper<UiProps>) -> Html {
    let ctw = use_ctw();
    let nexus = ctw.escaping.is_escaping();
    let gctw = use_gctw::<Mk48Game>();
    let splash_social_media_props = SplashSocialMediaProps::default()
        .github("https://github.com/SoftbearStudios/mk48")
        .google_play("https://play.google.com/store/apps/details?id=com.softbear.mk48");
    let on_play = gctw.send_ui_event_callback.reform(|alias| UiEvent::Spawn {
        alias,
        entity_type: EntityType::G5,
    });

    let margin = "0.5rem";
    let status = props.status.clone();

    const SHOOT_HINT: &str = "First, select an available weapon. Then, click in the direction to fire. If you hold the click for too long, you won't shoot.";
    const HINTS: &[(&str, &[&str])] = &[
        ("Invitation links cannot currently be accepted by players that are already in game. They must send a join request instead.", &["/invite"]),
        ("If you are asking how you move, you click and hold to set your speed and direction (or use WASD).", &["how", "move"]),
        ("The controls are click and hold (or WASD) to move, click (or Space) to shoot.", &["how", "play"]),
        (SHOOT_HINT, &["how", "shoot"]),
        (SHOOT_HINT, &["how", "use weapons"]),
        (SHOOT_HINT, &["how", "fire"])
    ];

    html! {
        <>
            if matches!(status, UiStatus::Playing(_) | UiStatus::Respawning(_)) && !nexus {
                if let UiStatus::Playing(playing) = status {
                    <Positioner id="status" position={Position::BottomMiddle{margin: "0"}} max_width="45%">
                        <StatusOverlay
                            status={playing.clone()}
                            fps={gctw.settings_cache.fps_shown.then_some(props.fps)}
                        />
                    </Positioner>
                    <UpgradeOverlay
                        position={Position::TopMiddle{margin}}
                        status={playing.clone()}
                        score={props.score}
                    />
                    <TeamOverlay
                        position={Position::TopLeft{margin}}
                        style="max-width:25%;"
                        team_proximity={playing.team_proximity.clone()}
                        teams={props.teams.clone()}
                        members={props.members.clone()}
                        joiners={props.joiners.clone()}
                        joins={props.joins.clone()}
                        label={Mk48Phrases::team_fleet_label as fn(&Translator) -> String}
                        name_placeholder={Mk48Phrases::team_fleet_name_placeholder as fn(&Translator) -> String}
                    />
                    <Hint entity_type={playing.entity_type}/>
                } else if let UiStatus::Respawning(respawning) = status {
                    <RespawnOverlay status={respawning} score={props.score}/>
                }
                <ChatOverlay
                    position={Position::BottomLeft{margin}}
                    style="max-width:25%;"
                    hints={HINTS}
                    label={Translator::chat_radio_label as fn(&Translator) -> String}
                />
            } else {
                if let UiStatus::Spawning = status {
                    <SpawnOverlay {on_play}>
                        {logo()}
                    </SpawnOverlay>
                }
                {splash_social_media(&ctw, splash_social_media_props)}
                {splash_links(&ctw, &[Mk48Route::Help], Default::default())}
                {splash_sign_in_link(&ctw)}
            }
            {splash_nexus_icons(&ctw, SplashNexusIconsProps::default().invitation(true))}
            <LeaderboardOverlay
                position={Position::TopRight{margin}}
                style="max-width:25%;"
                liveboard={matches!(props.status, UiStatus::Playing(_)) && !nexus}
            />
        </>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, SmolRoutable)]
pub enum Mk48Route {
    #[at("/about/")]
    About,
    #[at("/references/")]
    References,
    #[at("/help/")]
    Help,
    #[at("/ships/")]
    Ships,
    #[at("/ships/:selected")]
    ShipsSelected { selected: PathParam<EntityType> },
}

impl RoutableExt for Mk48Route {
    fn category(&self) -> Option<&'static str> {
        match self {
            Self::About | Self::Help | Self::Ships | Self::ShipsSelected { .. } => Some("help"),
            _ => None,
        }
    }

    fn label(&self, t: &Translator) -> String {
        match self {
            Self::Help => t.help_hint(),
            Self::About => t.about_hint(),
            Self::References => translate!(t, "References"),
            Self::Ships | Self::ShipsSelected { .. } => translate!(t, "Ships"),
        }
    }

    fn render<G: GameClient>(self) -> Html {
        match self {
            Self::About => html! {
                <AboutDialog/>
            },
            Self::References => html! {
                <ReferencesDialog/>
            },
            Self::Help => html! {
                <HelpDialog/>
            },
            Self::Ships => html! {
                <ShipsDialog/>
            },
            Self::ShipsSelected { selected } => html! {
                <ShipsDialog selected={selected.0}/>
            },
        }
    }

    fn tabs() -> impl Iterator<Item = Self> + 'static {
        [Self::Help, Self::Ships, Self::About].into_iter()
    }
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
    Respawn(EntityType),
    Spawn {
        alias: PlayerAlias,
        entity_type: EntityType,
    },
    Submerge(bool),
    Upgrade(EntityType),
    Team(TeamRequest),
}

#[derive(PartialEq, Clone, Default)]
pub struct UiProps {
    pub fps: f32,
    pub score: u32,
    pub status: UiStatus,
    pub teams: HashMap<TeamId, TeamDto>,
    pub members: Box<[PlayerId]>,
    pub joiners: Box<[PlayerId]>,
    pub joins: Box<[TeamId]>,
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
    pub primary: Instruction,
    pub secondary: Instruction,
    pub armament: Option<EntityType>,
    pub armament_consumption: Box<[bool]>,
    pub team_proximity: HashMap<TeamId, f32>,
}

#[derive(PartialEq, Clone)]
pub struct UiStatusRespawning {
    pub death_reason: DeathReason,
}

impl Mk48Game {
    pub(crate) fn update_ui_props(&self, context: &mut ClientContext<Self>, status: UiStatus) {
        let in_game = !matches!(status, UiStatus::Spawning);
        let props = UiProps {
            fps: self.fps_counter.last_sample().unwrap_or(0.0),
            score: context.state.game.score,
            status,
            teams: context.state.game.teams.clone(),
            members: context.state.game.members.clone(),
            joiners: context.state.game.joiners.clone(),
            joins: context.state.game.joins.clone(),
        };

        context.set_ui_props(props, in_game);
    }
}
