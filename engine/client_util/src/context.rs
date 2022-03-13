use crate::apply::Apply;
use crate::frontend::Frontend;
use crate::game_client::GameClient;
use crate::js_hooks::{domain_name_of, host, invitation_id, is_https, referrer, ws_protocol};
use crate::keyboard::KeyboardState;
use crate::local_storage::LocalStorage;
use crate::mouse::MouseState;
use crate::reconn_web_socket::ReconnWebSocket;
use crate::setting::CommonSettings;
use core_protocol::dto::{LeaderboardDto, LiveboardDto, MessageDto, PlayerDto, ServerDto, TeamDto};
use core_protocol::id::{InvitationId, PeriodId, PlayerId, ServerId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{
    ChatUpdate, ClientRequest, ClientUpdate, InvitationUpdate, LeaderboardUpdate, LiveboardUpdate,
    PlayerUpdate, Request, SystemUpdate, TeamUpdate, Update, WebSocketQuery,
};
use std::collections::{HashMap, VecDeque};

/// The context (except rendering) of a game.
pub struct Context<G: GameClient + ?Sized> {
    /// General client state.
    pub client: ClientState,
    /// Server state
    pub state: ServerState<G>,
    /// Server websocket
    pub socket: ReconnWebSocket<Update<G::Update>, Request<G::Command>, ServerState<G>>,
    /// Ui.
    pub ui: G::UiState,
    /// Keyboard input.
    pub keyboard: KeyboardState,
    /// Mouse input.
    pub mouse: MouseState,
    /// Settings.
    pub settings: G::Settings,
    /// Common settings.
    pub common_settings: CommonSettings,
    /// Local storage.
    pub(crate) local_storage: LocalStorage,
    pub(crate) frontend: Box<dyn Frontend<G::UiProps> + 'static>,
}

/// State common to all clients.
#[derive(Default)]
pub struct ClientState {
    /// Time of last or current update.
    pub update_seconds: f32,
}

/// Obtained from server via websocket.
pub struct ServerState<G: GameClient> {
    pub game: G::State,
    pub core: CoreState,
}

/// Server state specific to core functions
#[derive(Default)]
pub struct CoreState {
    pub player_id: Option<PlayerId>,
    pub created_invitation_id: Option<InvitationId>,
    /// Ordered, i.e. first is captain.
    pub members: Box<[PlayerId]>,
    pub joiners: Box<[PlayerId]>,
    pub joins: Box<[TeamId]>,
    pub leaderboards: [Vec<LeaderboardDto>; PeriodId::VARIANT_COUNT],
    pub liveboard: Vec<LiveboardDto>,
    pub messages: VecDeque<MessageDto>,
    pub(crate) players: HashMap<PlayerId, PlayerDto>,
    pub real_players: u32,
    pub teams: HashMap<TeamId, TeamDto>,
    pub servers: HashMap<ServerId, ServerDto>,
}

impl<G: GameClient> Default for ServerState<G> {
    fn default() -> Self {
        Self {
            game: G::State::default(),
            core: CoreState::default(),
        }
    }
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

impl<G: GameClient> Apply<Update<G::Update>> for ServerState<G> {
    fn apply(&mut self, update: Update<G::Update>) {
        match update {
            Update::Chat(update) => {
                match update {
                    ChatUpdate::Received(received) => {
                        for chat in received.iter() {
                            if self.core.messages.len() >= 9 {
                                // Keep at or below capacity of 10.
                                self.core.messages.pop_front();
                            }
                            self.core.messages.push_back(MessageDto::clone(chat));
                        }
                    }
                    _ => {}
                }
            }
            Update::Client(update) => match update {
                ClientUpdate::SessionCreated { player_id, .. } => {
                    self.core.player_id = Some(player_id);
                }
                _ => {}
            },
            Update::Game(update) => {
                self.game.apply(update);
            }
            Update::Invitation(update) => match update {
                InvitationUpdate::InvitationCreated(invitation_id) => {
                    self.core.created_invitation_id = Some(invitation_id);
                }
            },
            Update::Leaderboard(update) => match update {
                LeaderboardUpdate::Updated(period_id, leaderboard) => {
                    self.core.leaderboards[period_id as usize] =
                        leaderboard.iter().cloned().collect();
                }
            },
            Update::Liveboard(update) => {
                match update {
                    LiveboardUpdate::Updated { added, removed } => {
                        // Remove items first.
                        self.core
                            .liveboard
                            .drain_filter(|i| removed.contains(&i.player_id));

                        // Either update in place or add.
                        for new_item in added.iter() {
                            if let Some(old_item) = self
                                .core
                                .liveboard
                                .iter_mut()
                                .find(|i| i.player_id == new_item.player_id)
                            {
                                *old_item = new_item.clone();
                            } else {
                                self.core.liveboard.push(new_item.clone());
                            }
                        }

                        // Sort because deltas may reorder. Subtract from max so that larger scores are on top.
                        self.core
                            .liveboard
                            .sort_unstable_by_key(|dto| u32::MAX - dto.score);
                    }
                }
            }
            Update::Player(update) => match update {
                PlayerUpdate::Updated {
                    added,
                    removed,
                    real_players,
                } => {
                    for player in added.iter() {
                        self.core.players.insert(player.player_id, player.clone());
                    }
                    for remove in removed.iter() {
                        self.core.players.remove(remove);
                    }
                    self.core.real_players = real_players;
                }
                _ => {}
            },
            Update::System(update) => match update {
                SystemUpdate::Added(added) => {
                    for server in added.iter() {
                        self.core.servers.insert(server.server_id, server.clone());
                    }
                }
                SystemUpdate::Removed(removed) => {
                    for remove in removed.iter() {
                        self.core.servers.remove(remove);
                    }
                }
            },
            Update::Team(update) => match update {
                TeamUpdate::Members(members) => {
                    self.core.members = members[..].into();
                }
                TeamUpdate::Joiners(joiners) => {
                    self.core.joiners = joiners;
                }
                TeamUpdate::Joins(joins) => {
                    self.core.joins = joins;
                }
                TeamUpdate::AddedOrUpdated(added_or_updated) => {
                    for team in added_or_updated.iter() {
                        self.core.teams.insert(team.team_id, team.clone());
                    }
                }
                TeamUpdate::Removed(removed) => {
                    for remove in removed.iter() {
                        self.core.teams.remove(remove);
                    }
                }
                _ => {}
            },
        }
    }
}

impl<G: GameClient> Context<G> {
    pub(crate) fn new(
        mut local_storage: LocalStorage,
        mut common_settings: CommonSettings,
        settings: G::Settings,
        frontend: Box<dyn Frontend<G::UiProps> + 'static>,
    ) -> Self {
        let (host, server_id) = Self::compute_websocket_host(&common_settings, None, &*frontend);
        let socket = ReconnWebSocket::new(host, common_settings.protocol, None);
        common_settings.set_server_id(server_id, &mut local_storage);

        Self {
            client: ClientState::default(),
            state: ServerState::default(),
            socket,
            ui: G::UiState::default(),
            keyboard: KeyboardState::default(),
            mouse: MouseState::default(),
            settings,
            common_settings,
            local_storage,
            frontend,
        }
    }

    pub(crate) fn compute_websocket_host(
        common_settings: &CommonSettings,
        override_server_id: Option<ServerId>,
        frontend: &dyn Frontend<G::UiProps>,
    ) -> (String, Option<ServerId>) {
        let scheme = ws_protocol(frontend.get_real_encryption().unwrap_or(is_https()));
        let ideal_server_id = override_server_id.or(frontend.get_ideal_server_id());
        let host = frontend.get_real_host().unwrap_or(host());

        let ideal_host = ideal_server_id
            .filter(|_| !host.starts_with("localhost"))
            .map(|id| format!("{}.{}", id.0, domain_name_of(&host)))
            .unwrap_or(host);

        // crate::console_log!("override={:?} ideal server={:?}, host={:?}, ideal_host={:?}", override_server_id, ideal_server_id, host, ideal_host);

        let web_socket_query = WebSocketQuery {
            protocol: Some(common_settings.protocol),
            arena_id: common_settings.arena_id,
            session_id: common_settings.session_id,
            invitation_id: invitation_id(),
            referrer: referrer(),
        };

        let web_socket_query_url = serde_urlencoded::to_string(&web_socket_query).unwrap();

        (
            format!("{}://{}/ws/?{}", scheme, ideal_host, web_socket_query_url),
            ideal_server_id,
        )
    }

    /// Whether the game websocket is closed or errored (not open, opening, or nonexistent).
    pub fn connection_lost(&self) -> bool {
        self.socket.is_terminated()
    }

    /// Send a game command on the socket.
    pub fn send_to_game(&mut self, request: G::Command) {
        self.send_to_server(Request::Game(request));
    }

    /// Send a request to set the player's alias.
    pub fn send_set_alias(&mut self, alias: PlayerAlias) {
        self.send_to_server(Request::Client(ClientRequest::SetAlias(alias)));
    }

    /// Send a request to log an error message.
    pub fn send_trace(&mut self, message: String) {
        self.send_to_server(Request::Client(ClientRequest::Trace { message }));
    }

    /// Send a request on the socket.
    pub fn send_to_server(&mut self, request: Request<G::Command>) {
        self.socket.send(request);
    }

    /// Set the props used to render the UI. Javascript must implement part of this.
    pub fn set_ui_props(&mut self, props: G::UiProps) {
        self.frontend.set_ui_props(props);
    }
}
