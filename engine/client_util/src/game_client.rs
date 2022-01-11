// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::context::Context;
use crate::keyboard::KeyboardEvent;
use crate::mouse::MouseEvent;
use crate::renderer::renderer::{Layer, Renderer};
use crate::setting::Settings;
use core_protocol::id::GameId;
use core_protocol::rpc::ClientUpdate;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A modular game client-side.
pub trait GameClient {
    const GAME_ID: GameId;

    /// Game command to server.
    type Command: 'static + Serialize + Clone;
    /// Game render layer.
    type RendererLayer: Layer;
    /// Game state.
    type State: Apply<Self::Update>;
    /// Event from UI.
    type UiEvent: DeserializeOwned;
    /// State of UI.
    type UiState: Apply<Self::UiEvent>;
    /// Properties sent to UI.
    type UiProps: 'static + Serialize;
    /// Game update from server.
    type Update: 'static + DeserializeOwned;
    /// Game settings
    type Settings: Settings;

    fn new() -> Self;

    /// Creates the (game-specific) render layer.
    fn init(&mut self, renderer: &mut Renderer, context: &mut Context<Self>)
        -> Self::RendererLayer;

    /// Peek at a core update before it is applied to `CoreState`.
    fn peek_core(&mut self, _inbound: &ClientUpdate, _context: &mut Context<Self>) {}

    /// Peek at a game update before it is applied to `GameState`.
    fn peek_game(
        &mut self,
        _inbound: &Self::Update,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
        _layer: &mut Self::RendererLayer,
    ) {
    }

    /// Peek at a keyboard update before it is applied to `KeyboardState`.
    fn peek_keyboard(&mut self, _event: &KeyboardEvent, _context: &mut Context<Self>) {}

    /// Peek at a mouse update before it is applied to `MouseState`.
    fn peek_mouse(&mut self, _event: &MouseEvent, _context: &mut Context<Self>) {}

    /// Render the game. Optional, as this may be done in `tick`.
    fn render(
        &mut self,
        _elapsed_seconds: f32,
        _context: &Context<Self>,
        _renderer: &mut Renderer,
        _renderer_layer: &mut Self::RendererLayer,
    ) {
    }
    /// A game with update and render intertwined implements this method.
    /// Otherwise, it implements update() and render().
    fn tick(
        &mut self,
        elapsed_seconds: f32,
        context: &mut Context<Self>,
        renderer: &mut Renderer,
        renderer_layer: &mut Self::RendererLayer,
    ) {
        self.update(elapsed_seconds, context);
        self.render(elapsed_seconds, &*context, renderer, renderer_layer);
    }

    /// Peek at a UI event before it is applied to `UiState`.
    fn peek_ui(
        &mut self,
        _event: &Self::UiEvent,
        _context: &mut Context<Self>,
        _layer: &mut Self::RendererLayer,
    ) {
    }

    /// Updates the game. Optional, as may be done in `tick`.
    fn update(&mut self, _elapsed_seconds: f32, _context: &Context<Self>) {}
}
