// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(hash_raw_entry)]
#![feature(hash_drain_filter)]
#![feature(drain_filter)]
#![feature(must_not_suspend)]

#[macro_use]
mod animation;
mod audio;
mod buffer;
mod deque;
mod game;
mod input;
mod particle;
mod renderer;
mod settings;
mod shader;
mod text_cache;
mod texture;

use crate::game::Game;
use common::altitude::Altitude;
use common::angle::Angle;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use common::ticks::Ticks;
use common::velocity::Velocity;
use core_protocol::id::*;
use core_protocol::name::*;
use glam::Vec2;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::num::{NonZeroU32, NonZeroU64};
use std::panic;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPlayerModel {
    player_id: PlayerId,
    name: PlayerAlias,
    captain: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModel {
    name: PlayerAlias,
    player_id: Option<PlayerId>,
    team: Option<TeamName>,
    whisper: bool,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamModel {
    team_id: TeamId,
    name: TeamName,
    joining: bool,
}

#[derive(Serialize)]
pub struct LeaderboardItemModel {
    name: PlayerAlias,
    team: Option<TeamName>,
    score: u32,
}

#[derive(Serialize)]
pub struct DeathReasonModel {
    #[serde(rename = "type")]
    death_type: &'static str,
    player: Option<PlayerAlias>,
    entity: Option<EntityType>,
}

// For serializing a vec2 as {"x": ..., "y": ...} instead of [..., ...]
#[derive(Serialize)]
pub struct Vec2Model {
    x: f32,
    y: f32,
}

impl From<Vec2> for Vec2Model {
    fn from(vec2: Vec2) -> Self {
        Self {
            x: vec2.x,
            y: vec2.y,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[serde(rename_all = "camelCase")]
    Alive {
        #[serde(rename = "type")]
        entity_type: EntityType,
        velocity: Velocity,
        direction: Angle,
        position: Vec2Model,
        altitude: Altitude,
        #[serde(skip_serializing_if = "Option::is_none")]
        armament_consumption: Option<Arc<[Ticks]>>,
    },
    #[serde(rename_all = "camelCase")]
    Spawning {
        connection_lost: bool,
        death_reason: Option<DeathReasonModel>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    player_id: Option<PlayerId>,
    team_name: Option<TeamName>,
    invitation_id: Option<InvitationId>,
    score: u32,
    player_count: u32,
    fps: f32,
    status: Status,
    chats: Vec<ChatModel>,
    liveboard: Vec<LeaderboardItemModel>,
    leaderboards: HashMap<PeriodId, Vec<LeaderboardItemModel>>,
    team_captain: bool,
    team_members: Vec<TeamPlayerModel>,
    team_join_requests: Vec<TeamPlayerModel>,
    teams: Vec<TeamModel>,
}

#[wasm_bindgen(raw_module = "../../../src/App.svelte")]
extern "C" {
    // state must be a JsValue corresponding to a State instance.
    #[wasm_bindgen(js_name = "setState")]
    pub fn set_state_inner(state: JsValue);

    #[wasm_bindgen(js_name = "setSessionId")]
    pub fn set_session_id(arena_id: Option<String>, session_id: Option<String>);
}

#[wasm_bindgen(raw_module = "../../../src/util/compatibility.js")]
extern "C" {
    // status must be a JsValue corresponding to a Status instance.
    #[wasm_bindgen(js_name = "hasWebP")]
    pub fn has_webp() -> bool;
}

pub fn set_state(state: &State) {
    let ser = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    set_state_inner(state.serialize(&ser).unwrap());
}

/*
   There are a few restrictions on the cell that can be used here:
       - Game is !Send + !Sync due to its WebGlRenderingContext.
       - Even though there is one thread, Rust wants to enforce static (non mut) are Send + Sync.
         Making it static mut fixes the problem, but requires the use of unsafe at every use.
*/
static mut GAME: UnsafeCell<MaybeUninit<RefCell<Game>>> = UnsafeCell::new(MaybeUninit::uninit());

/// Easily get the game.
fn borrow_game() -> RefMut<'static, Game> {
    unsafe { GAME.get_mut().assume_init_ref().borrow_mut() }
}

/// Easily access the game.
///
/// This should be used instead of [`borrow_game()`] if mitigation of JavaScript "Immediate Events"
/// is a concern (if calls were observed to interrupt already-executing WebAssembly).
fn with_game<F: FnOnce(RefMut<'static, Game>)>(function: F) {
    unsafe {
        if let Ok(game) = GAME.get_mut().assume_init_ref().try_borrow_mut() {
            function(game);
        }
    }
}

#[wasm_bindgen(js_name = "handleSpawn")]
pub fn handle_spawn(name: String, entity_type: String) {
    with_game(move |mut game| game.spawn(name, parse_enum(&entity_type)));
}

#[wasm_bindgen]
pub enum MouseButton {
    Left = 0,
    Right = 2,
}

#[wasm_bindgen(js_name = "handleMouseButton")]
pub fn handle_mouse_button(button: MouseButton, down: bool) {
    with_game(move |mut game| game.input.handle_mouse_button(button, down));
}

#[wasm_bindgen(js_name = "handleMouseMove")]
pub fn handle_mouse_move(x: f32, y: f32) {
    with_game(|mut game| game.input.handle_mouse_move((x, y).into()));
}

#[wasm_bindgen(js_name = "handleWheel")]
pub fn handle_wheel(delta: f32) {
    with_game(|mut game| game.input.handle_wheel(delta));
}

#[wasm_bindgen(js_name = "handleJoystick")]
pub fn handle_joystick(x: f32, y: f32, stop: bool) {
    borrow_game()
        .input
        .handle_joystick(Some((x, y).into()), stop);
}

#[wasm_bindgen(js_name = "handleJoystickRelease")]
pub fn handle_joystick_release() {
    borrow_game().input.handle_joystick(None, false);
}

#[wasm_bindgen(js_name = "handleVolume")]
pub fn handle_volume(volume: f32) {
    borrow_game().audio_player.set_volume(volume);
}

#[wasm_bindgen(js_name = "handleShoot")]
pub fn handle_shoot(shoot: bool) {
    borrow_game().input.shoot = shoot;
}

#[wasm_bindgen(js_name = "handlePay")]
pub fn handle_pay(pay: bool) {
    borrow_game().input.pay = pay;
}

#[wasm_bindgen(js_name = "handleActive")]
pub fn handle_active(active: bool) {
    borrow_game().set_active(active);
}

#[wasm_bindgen(js_name = "handleAltitudeTarget")]
pub fn handle_altitude_target(altitude_target: f32) {
    borrow_game().set_altitude_target(Altitude::from_norm(altitude_target));
}

#[wasm_bindgen(js_name = "handleArmamentSelection")]
pub fn handle_armament_selection(armament_selection: String) {
    let segments: Vec<&str> = armament_selection.split('/').collect();
    if segments.len() != 2 {
        panic!("invalid armament selection {} segments", segments.len());
    }
    let kind: EntityKind = parse_enum(segments[0]);
    let sub_kind: EntitySubKind = parse_enum(segments[1]);
    borrow_game().input.armament_selection = Some((kind, sub_kind));
}

#[wasm_bindgen(js_name = "handleUpgrade")]
pub fn handle_upgrade(selection: String) {
    borrow_game().upgrade(parse_enum(&selection));
}

#[wasm_bindgen(js_name = "handleSendChat")]
pub fn handle_send_chat(message: String, team: bool) {
    borrow_game().send_chat(message, team);
}

#[wasm_bindgen(js_name = "handleCreateTeam")]
pub fn handle_create_team(name: String) {
    borrow_game().create_team(TeamName::new(&name));
}

#[wasm_bindgen(js_name = "handleRequestJoinTeam")]
pub fn handle_request_join_team(team_id: u32) {
    borrow_game().request_join_team(TeamId(NonZeroU32::new(team_id).unwrap()));
}

#[wasm_bindgen(js_name = "handleAcceptJoinTeam")]
pub fn handle_accept_join_team(player_id: u32) {
    borrow_game().accept_join_team(PlayerId(NonZeroU32::new(player_id).unwrap()));
}

#[wasm_bindgen(js_name = "handleRejectJoinTeam")]
pub fn handle_reject_join_team(player_id: u32) {
    borrow_game().reject_join_team(PlayerId(NonZeroU32::new(player_id).unwrap()));
}

#[wasm_bindgen(js_name = "handleKickFromTeam")]
pub fn handle_kick_from_team(player_id: u32) {
    borrow_game().kick_from_team(PlayerId(NonZeroU32::new(player_id).unwrap()));
}

#[wasm_bindgen(js_name = "handleLeaveTeam")]
pub fn handle_leave_team() {
    borrow_game().leave_team();
}

#[wasm_bindgen(js_name = "handleMutePlayer")]
pub fn handle_mut_player(player_id: u32, mute: bool) {
    borrow_game().mute_player(PlayerId(NonZeroU32::new(player_id).unwrap()), mute);
}

#[wasm_bindgen(js_name = "handleWebSocketFormat")]
pub fn handle_web_socket_format(format: String) {
    if let Some(socket) = borrow_game().server_web_socket.as_mut() {
        socket.set_format(parse_enum(&format));
    }
}

#[wasm_bindgen(js_name = "handleCinematic")]
pub fn handle_cinematic(cinematic: bool) {
    borrow_game().cinematic = cinematic;
}

/// For testing purposes only.
#[wasm_bindgen(js_name = "handleDropWebSocket")]
pub fn handle_drop_web_sockets(core: bool, server: bool) {
    let mut game = borrow_game();
    if core {
        game.core_web_socket.drop();
    }
    if server {
        if let Some(socket) = game.server_web_socket.as_mut() {
            socket.drop();
        }
    }
}

/// Enables logging of latency-related info.
#[wasm_bindgen(js_name = "handleDebugLatency")]
pub fn handle_debug_latency(debug_latency: bool) {
    borrow_game().debug_latency = debug_latency;
}

/// run is the entry point for actually taking arguments.
#[wasm_bindgen]
pub fn run(settings: JsValue, aid: Option<String>, sid: Option<String>, inv_id: Option<String>) {
    let settings = serde_wasm_bindgen::from_value(settings).unwrap_or_default();
    let arena_id = aid
        .and_then(|id| NonZeroU32::from_str(&id).ok())
        .map(|id| ArenaId(id));
    let session_id = sid
        .and_then(|id| NonZeroU64::from_str(&id).ok())
        .map(|id| SessionId(id));
    let invitation_id = inv_id
        .and_then(|id| NonZeroU32::from_str(&id).ok())
        .map(|id| InvitationId(id));
    unsafe {
        // SAFETY: This has to run before any calls to borrow_game()
        *GAME.get_mut() = MaybeUninit::new(RefCell::new(Game::new(
            settings,
            arena_id.zip(session_id),
            invitation_id,
        )));
    }
}

#[wasm_bindgen]
pub fn frame(time_seconds: f32) {
    borrow_game().frame(time_seconds);
}

/// start is the actual entry point.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    unsafe {
        // SAFETY: As per spec, only called once (before .data()) is called.
        EntityType::init();
    }

    Ok(())
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// parse_enum deserializes a string into an enum, panicking if it doesn't match any variant.
fn parse_enum<E: DeserializeOwned>(string: &str) -> E {
    let fmt = format!("\"{}\"", string);
    serde_json::from_str(&fmt).unwrap()
}
