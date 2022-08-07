// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::context::Context;
use crate::keyboard::KeyboardEvent;
use crate::mouse::MouseEvent;
use crate::renderer::renderer::{Layer, Renderer};
use crate::setting::Settings;
use crate::visibility::VisibilityEvent;
use core_protocol::id::GameId;
use core_protocol::rpc::ClientUpdate;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A modular game client-side.
pub trait GameClient: Sized + 'static {
    const GAME_ID: GameId;

    /// Audio files to play.
    #[cfg(feature = "audio")]
    type Audio: crate::audio::Audio;
    /// Game-specific command to server.
    type GameRequest: 'static + Serialize + Clone;
    /// Game-specific render layer.
    type RendererLayer: Layer;
    /// Game-specific state.
    type GameState: Apply<Self::GameUpdate>;
    /// Event from game UI.
    type UiEvent: DeserializeOwned;
    /// State of game UI.
    type UiState: Apply<Self::UiEvent>;
    /// Properties sent to game UI.
    type UiProps: 'static;
    /// Game-specific update from server.
    type GameUpdate: 'static + DeserializeOwned;
    /// Game-specific settings
    type GameSettings: Settings;

    fn new() -> Self;

    /// Creates the (game-specific) settings.
    fn init_settings(&mut self, renderer: &mut Renderer) -> Self::GameSettings;

    /// Creates the (game-specific) render layer.
    fn init_layer(
        &mut self,
        renderer: &mut Renderer,
        context: &mut Context<Self>,
    ) -> Self::RendererLayer;

    /// Peek at a core update before it is applied to `CoreState`.
    fn peek_core(&mut self, _inbound: &ClientUpdate, _context: &mut Context<Self>) {}

    /// Peek at a game update before it is applied to `GameState`.
    fn peek_game(
        &mut self,
        inbound: &Self::GameUpdate,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
        _layer: &mut Self::RendererLayer,
    ) {
        let _ = inbound;
    }

    /// Peek at a keyboard event before it is applied to `KeyboardState`.
    fn peek_keyboard(&mut self, _event: &KeyboardEvent, _context: &mut Context<Self>) {}

    /// Peek at a mouse event before it is applied to `MouseState`.
    fn peek_mouse(
        &mut self,
        event: &MouseEvent,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
    ) {
        let _ = event;
    }

    /// Peek at a visibility event before it is applied to `VisibilityState`.
    fn peek_visibility(
        &mut self,
        event: &VisibilityEvent,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
    ) {
        let _ = event;
    }

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
        self.update(elapsed_seconds, context, &*renderer);
        self.render(elapsed_seconds, context, renderer, renderer_layer);
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
    fn update(
        &mut self,
        _elapsed_seconds: f32,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
    ) {
    }
}
