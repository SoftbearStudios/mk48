use crate::apply::Apply;
use crate::game_client::GameClient;
use crate::js_hooks::{host, invitation_id, is_https, referrer, ws_protocol};
use crate::keyboard::KeyboardState;
use crate::local_storage::LocalStorage;
use crate::mouse::MouseState;
use crate::reconn_web_socket::ReconnWebSocket;
use crate::setting::{CommonSettings, Settings};
use core_protocol::dto::{LeaderboardDto, LiveboardDto, MessageDto, PlayerDto, TeamDto};
use core_protocol::id::{InvitationId, PeriodId, PlayerId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{ClientRequest, ClientUpdate};
use core_protocol::web_socket::WebSocketFormat;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

/// The context (except rendering) of a game.
pub struct Context<G: GameClient + ?Sized> {
    /// General client state.
    pub client: ClientState,
    /// Core websocket/state.
    pub core_socket: ReconnWebSocket<ClientUpdate, ClientRequest, CoreState>,
    /// Game websocket/state
    pub game_socket: Option<ReconnWebSocket<G::Update, G::Command, G::State>>,
    /// Ui.
    pub ui: G::UiState,
    /// Keyboard input.
    pub keyboard: KeyboardState,
    /// Mouse input.
    pub mouse: MouseState,
    /// Settings.
    pub settings: G::Settings,
    /// Common settings.
    pub(crate) common_settings: CommonSettings,
    /// Local storage.
    pub(crate) local_storage: LocalStorage,
    /// Websocket info (host, protocol)
    pub(crate) web_socket_info: (String, &'static str),
}

/// State common to all clients.
#[derive(Default)]
pub struct ClientState {
    /// Time of last or current update.
    pub update_seconds: f32,
}

/// Obtained from core server via websocket.
#[derive(Default)]
pub struct CoreState {
    pub player_id: Option<PlayerId>,
    pub created_invitation_id: Option<InvitationId>,
    pub joins: HashSet<TeamId>,
    pub joiners: HashSet<PlayerId>,
    pub leaderboards: [Vec<LeaderboardDto>; PeriodId::VARIANT_COUNT],
    pub liveboard: Vec<LiveboardDto>,
    pub messages: VecDeque<MessageDto>,
    pub(crate) players: HashMap<PlayerId, PlayerDto>,
    pub teams: HashMap<TeamId, TeamDto>,
}

impl CoreState {
    /// Gets whether a player is friendly to an other player, taking into account team membership.
    /// Returns false if either `PlayerId` is None.
    pub fn is_friendly(&self, other_player_id: Option<PlayerId>) -> bool {
        self.are_friendly(self.player_id, other_player_id)
    }

    /// Gets whether player is friendly to other player, taking into account team membership.
    /// Returns false if either `PlayerId` is None.
    pub fn are_friendly(
        &self,
        player_id: Option<PlayerId>,
        other_player_id: Option<PlayerId>,
    ) -> bool {
        player_id
            .zip(other_player_id)
            .map(|(id1, id2)| {
                id1 == id2
                    || self
                        .team_id_lookup(id1)
                        .zip(self.team_id_lookup(id2))
                        .map(|(id1, id2)| id1 == id2)
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Gets player's `PlayerDto`.
    pub fn player(&self) -> Option<&PlayerDto> {
        self.player_id.and_then(|id| self.players.get(&id))
    }

    /// Player or bot (simulated) `PlayerDto`.
    pub fn player_or_bot(&self, player_id: PlayerId) -> Option<PlayerDto> {
        player_id
            .is_bot()
            .then(|| {
                Some(PlayerDto {
                    alias: PlayerAlias::from_bot_player_id(player_id),
                    player_id,
                    team_captain: false,
                    team_id: None,
                })
            })
            .unwrap_or_else(|| self.players.get(&player_id).map(|r| r.clone()))
    }

    /// Gets hashmap that contains players, but *not* bots.
    pub fn only_players(&self) -> &HashMap<PlayerId, PlayerDto> {
        &self.players
    }

    /// Gets player's team's `TeamDto`.
    pub fn team(&self) -> Option<&TeamDto> {
        self.team_id().and_then(|id| self.teams.get(&id))
    }

    /// Gets player's `TeamId`.
    pub fn team_id(&self) -> Option<TeamId> {
        self.player_id.and_then(|id| self.team_id_lookup(id))
    }

    /// Gets a player's `TeamId`.
    fn team_id_lookup(&self, player_id: PlayerId) -> Option<TeamId> {
        self.players.get(&player_id).and_then(|p| p.team_id)
    }
}

impl Apply<ClientUpdate> for CoreState {
    fn apply(&mut self, inbound: ClientUpdate) {
        match inbound {
            ClientUpdate::JoinersUpdated { added, removed } => {
                for &team_id in added.iter() {
                    self.joiners.insert(team_id);
                }
                for remove in removed.iter() {
                    self.joiners.remove(remove);
                }
            }
            ClientUpdate::JoinsUpdated { added, removed } => {
                for &team_id in added.iter() {
                    self.joins.insert(team_id);
                }
                for remove in removed.iter() {
                    self.joins.remove(remove);
                }
            }
            ClientUpdate::LeaderboardUpdated {
                leaderboard,
                period,
            } => {
                self.leaderboards[period as usize] = leaderboard.iter().cloned().collect();
            }
            ClientUpdate::LiveboardUpdated { added, removed } => {
                // Remove items first.
                self.liveboard
                    .drain_filter(|i| removed.contains(&i.player_id));

                // Either update in place or add.
                for new_item in added.iter() {
                    if let Some(old_item) = self
                        .liveboard
                        .iter_mut()
                        .find(|i| i.player_id == new_item.player_id)
                    {
                        *old_item = new_item.clone();
                    } else {
                        self.liveboard.push(new_item.clone());
                    }
                }

                // Sort because deltas may reorder.
                self.liveboard.sort_unstable();
            }
            ClientUpdate::MessagesUpdated { added } => {
                for chat in added.iter() {
                    if self.messages.len() >= 9 {
                        // Keep at or below capacity of 10.
                        self.messages.pop_front();
                    }
                    self.messages.push_back(chat.clone());
                }
            }
            ClientUpdate::PlayersUpdated { added, removed } => {
                for player in added.iter() {
                    self.players.insert(player.player_id, player.clone());
                }
                for remove in removed.iter() {
                    self.players.remove(remove);
                }
            }
            ClientUpdate::SessionCreated { player_id, .. } => {
                self.player_id = Some(player_id);
            }
            ClientUpdate::TeamsUpdated { added, removed } => {
                for team in added.iter() {
                    self.teams.insert(team.team_id, team.clone());
                }
                for remove in removed.iter() {
                    self.teams.remove(remove);
                }
            }
            ClientUpdate::InvitationCreated { invitation_id } => {
                self.created_invitation_id = Some(invitation_id);
            }
            _ => {}
        }
    }
}

impl<G: GameClient> Context<G> {
    pub(crate) fn new() -> Self {
        // Not guaranteed to set either or both to Some. Could fail to load.
        let local_storage = LocalStorage::new();

        let common_settings = CommonSettings::load(&local_storage);

        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            #[wasm_bindgen(js_name = "getRealHost", catch)]
            pub fn get_real_host() -> Result<String, JsValue>;

            #[wasm_bindgen(js_name = "getRealEncryption", catch)]
            pub fn get_real_encryption() -> Result<bool, JsValue>;
        }

        let web_socket_info = (
            get_real_host().unwrap_or(host()),
            ws_protocol(get_real_encryption().unwrap_or(is_https())),
        );

        let core = ReconnWebSocket::new(
            &format!("{}://{}/client/ws/", web_socket_info.1, web_socket_info.0),
            WebSocketFormat::Json,
            Some(ClientRequest::CreateSession {
                game_id: G::GAME_ID,
                invitation_id: invitation_id(),
                referrer: referrer(),
                saved_session_tuple: common_settings.session_tuple(),
            }),
        );

        Self {
            client: ClientState::default(),
            core_socket: core,
            game_socket: None,
            ui: G::UiState::default(),
            keyboard: KeyboardState::default(),
            mouse: MouseState::default(),
            settings: G::Settings::load(&local_storage),
            common_settings,
            local_storage,
            web_socket_info,
        }
    }

    /// Immutable reference to `CoreState`.
    pub fn core(&self) -> &CoreState {
        self.core_socket.state()
    }

    /// Mutable reference to `CoreState`.
    #[allow(dead_code)]
    pub(crate) fn core_mut(&mut self) -> &mut CoreState {
        self.core_socket.state_mut()
    }

    /// Assumes that game state exists i.e. there once was a game server connection.
    pub fn game(&self) -> &G::State {
        self.game_socket.as_ref().unwrap().state()
    }

    /// Assumes that game state exists i.e. there once was a game server connection.
    pub fn game_mut(&mut self) -> &mut G::State {
        self.game_socket.as_mut().unwrap().state_mut()
    }

    /// Send a request on the core websocket.
    pub fn send_to_core(&mut self, request: ClientRequest) {
        self.core_socket.send(request)
    }

    /// Whether the game websocket is closed or errored (not open, opening, or nonexistent).
    pub fn game_connection_lost(&self) -> bool {
        self.game_socket
            .as_ref()
            .map(|ws| ws.is_closed())
            .unwrap_or(false)
    }

    /// Send a request on the game websocket.
    pub fn send_to_game(&mut self, request: G::Command) {
        if let Some(ws) = self.game_socket.as_mut() {
            ws.send(request);
        }
    }

    /// Set the props used to render the UI. Javascript must implement part of this.
    pub fn set_ui_props(&mut self, props: G::UiProps) {
        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            // props must be a JsValue corresponding to a US instance.
            #[wasm_bindgen(js_name = "setProps")]
            pub fn set_props(props: JsValue);
        }

        let ser = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        set_props(props.serialize(&ser).unwrap());
    }
}
