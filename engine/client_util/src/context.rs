// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::browser_storage::BrowserStorages;
use crate::frontend::Frontend;
use crate::game_client::GameClient;
use crate::js_util::{domain_name_of, host, invitation_id, is_https, ws_protocol};
use crate::keyboard::KeyboardState;
use crate::mouse::MouseState;
use crate::reconn_web_socket::ReconnWebSocket;
use crate::setting::CommonSettings;
use crate::visibility::VisibilityState;
use core_protocol::dto::{LeaderboardDto, LiveboardDto, MessageDto, PlayerDto, ServerDto, TeamDto};
use core_protocol::id::{CohortId, InvitationId, LoginType, PeriodId, PlayerId, ServerId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{
    ChatUpdate, ClientRequest, ClientUpdate, InvitationUpdate, LeaderboardUpdate, LiveboardUpdate,
    PlayerUpdate, Request, SystemUpdate, TeamUpdate, Update, WebSocketQuery,
};
use heapless::HistoryBuffer;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use web_sys::UrlSearchParams;

#[cfg(feature = "audio")]
use crate::audio::AudioPlayer;

/// The context (except rendering) of a game.
pub struct Context<G: GameClient + ?Sized> {
    /// General client state.
    pub client: ClientState,
    /// Server state
    pub state: ServerState<G>,
    /// Server websocket
    pub socket: ReconnWebSocket<Update<G::GameUpdate>, Request<G::GameRequest>, ServerState<G>>,
    /// Audio player (volume managed automatically).
    #[cfg(feature = "audio")]
    pub audio: AudioPlayer<G::Audio>,
    /// Keyboard input.
    pub keyboard: KeyboardState,
    /// Mouse input.
    pub mouse: MouseState,
    /// Whether the page is visible.
    pub visibility: VisibilityState,
    /// Settings.
    pub settings: G::GameSettings,
    /// Common settings.
    pub common_settings: CommonSettings,
    /// Local storage.
    pub browser_storages: BrowserStorages,
    pub(crate) frontend: Box<dyn Frontend<G::UiProps> + 'static>,
}

/// State common to all clients.
#[derive(Default)]
pub struct ClientState {
    /// Time of last or current update.
    pub time_seconds: f32,
}

/// Obtained from server via websocket.
pub struct ServerState<G: GameClient> {
    pub game: G::GameState,
    pub core: Rc<CoreState>,
}

/// Server state specific to core functions
#[derive(Default)]
pub struct CoreState {
    pub cohort_id: Option<CohortId>,
    pub player_id: Option<PlayerId>,
    pub created_invitation_id: Option<InvitationId>,
    /// Ordered, i.e. first is captain.
    pub members: Box<[PlayerId]>,
    pub joiners: Box<[PlayerId]>,
    pub joins: Box<[TeamId]>,
    /// TODO: Deprecate `pub`
    pub leaderboards: [Box<[LeaderboardDto]>; std::mem::variant_count::<PeriodId>()],
    pub liveboard: Vec<LiveboardDto>,
    pub messages: HistoryBuffer<MessageDto, 9>,
    pub(crate) players: HashMap<PlayerId, PlayerDto>,
    pub real_players: u32,
    pub teams: HashMap<TeamId, TeamDto>,
    pub servers: HashMap<ServerId, ServerDto>,
}

impl<G: GameClient> Default for ServerState<G> {
    fn default() -> Self {
        Self {
            game: G::GameState::default(),
            core: Default::default(),
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
                    moderator: false,
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

    pub fn leaderboard(&self, period_id: PeriodId) -> &[LeaderboardDto] {
        &self.leaderboards[period_id as usize]
    }
}

impl<G: GameClient> Apply<Update<G::GameUpdate>> for ServerState<G> {
    fn apply(&mut self, update: Update<G::GameUpdate>) {
        // Use rc_borrow_mut to keep semantics of shared references the same while sharing with
        // yew_frontend.
        use rc_borrow_mut::RcBorrowMut;
        let mut core = Rc::borrow_mut(&mut self.core);

        match update {
            Update::Chat(update) => {
                match update {
                    ChatUpdate::Received(received) => {
                        // Need to use into_vec since
                        // https://github.com/rust-lang/rust/issues/59878 is incomplete.
                        core.messages.extend(received.into_vec());
                    }
                    _ => {}
                }
            }
            Update::Client(update) => match update {
                ClientUpdate::SessionCreated {
                    cohort_id,
                    player_id,
                    ..
                } => {
                    core.cohort_id = Some(cohort_id);
                    core.player_id = Some(player_id);
                }
                _ => {}
            },
            Update::Game(update) => {
                self.game.apply(update);
            }
            Update::Invitation(update) => match update {
                InvitationUpdate::InvitationCreated(invitation_id) => {
                    core.created_invitation_id = Some(invitation_id);
                }
            },
            Update::Leaderboard(update) => match update {
                LeaderboardUpdate::Updated(period_id, leaderboard) => {
                    core.leaderboards[period_id as usize] = leaderboard;
                }
            },
            Update::Liveboard(update) => {
                match update {
                    LiveboardUpdate::Updated { added, removed } => {
                        let liveboard = &mut core.liveboard;

                        // Remove items that were removed or will be added.
                        liveboard.retain(|i| {
                            !(removed.contains(&i.player_id)
                                || added.iter().any(|a| a.player_id == i.player_id))
                        });

                        // Only inserting in sorted order, not updating in place.
                        // Invariant added cannot contain duplicate player ids.
                        for item in added.into_vec() {
                            // unwrap_err will never panic because player ids are unique because
                            // we searched for them with find.
                            let index = liveboard
                                .binary_search_by(|other| {
                                    // Put higher scores higher on leaderboard.
                                    // If scores are equal, ensure total ordering with player id.
                                    // NOTE: order of cmp is reversed compared to sort_by.
                                    item.score
                                        .cmp(&other.score)
                                        .then_with(|| other.player_id.cmp(&item.player_id))
                                })
                                .inspect(|_| debug_assert!(false))
                                .into_ok_or_err();

                            // Only inserting in correct position to maintain sorted order.
                            liveboard.insert(index, item.clone());
                        }
                    }
                }
            }
            Update::Player(update) => match update {
                PlayerUpdate::Updated {
                    added,
                    removed,
                    real_players,
                } => {
                    for player in added.into_vec() {
                        core.players.insert(player.player_id, player);
                    }
                    for player_id in removed.iter() {
                        core.players.remove(player_id);
                    }
                    core.real_players = real_players;
                }
                _ => {}
            },
            Update::System(update) => match update {
                SystemUpdate::Added(added) => {
                    for server in added.into_vec() {
                        core.servers.insert(server.server_id, server);
                    }
                }
                SystemUpdate::Removed(removed) => {
                    for server_id in removed.iter() {
                        core.servers.remove(server_id);
                    }
                }
            },
            Update::Team(update) => match update {
                TeamUpdate::Members(members) => {
                    core.members = members;
                }
                TeamUpdate::Joiners(joiners) => {
                    core.joiners = joiners;
                }
                TeamUpdate::Joins(joins) => {
                    core.joins = joins;
                }
                TeamUpdate::AddedOrUpdated(added_or_updated) => {
                    for team in added_or_updated.into_vec() {
                        core.teams.insert(team.team_id, team);
                    }
                }
                TeamUpdate::Removed(removed) => {
                    for team_id in removed.iter() {
                        core.teams.remove(team_id);
                    }
                }
                _ => {}
            },
        }
    }
}

impl<G: GameClient> Context<G> {
    pub(crate) fn new(
        mut browser_storages: BrowserStorages,
        mut common_settings: CommonSettings,
        settings: G::GameSettings,
        frontend: Box<dyn Frontend<G::UiProps> + 'static>,
    ) -> Self {
        let (host, server_id) = Self::compute_websocket_host(&common_settings, None, &*frontend);
        let socket = ReconnWebSocket::new(host, common_settings.protocol, None);
        common_settings.set_server_id(server_id, &mut browser_storages);

        Self {
            #[cfg(feature = "audio")]
            audio: AudioPlayer::default(),
            client: ClientState::default(),
            state: ServerState::default(),
            socket,
            keyboard: KeyboardState::default(),
            mouse: MouseState::default(),
            visibility: VisibilityState::default(),
            settings,
            common_settings,
            browser_storages,
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
        let host = frontend.get_real_host().unwrap_or_else(host);

        let ideal_host = ideal_server_id
            .filter(|_| !host.starts_with("localhost"))
            .map(|id| format!("{}.{}", id.0, domain_name_of(&host)))
            .unwrap_or(host);

        // crate::console_log!("override={:?} ideal server={:?}, host={:?}, ideal_host={:?}", override_server_id, ideal_server_id, host, ideal_host);

        let query = js_hooks::window().location().search().ok();
        let params = query.and_then(|query| UrlSearchParams::new_with_str(&query).ok());
        let oauth2_code = params.and_then(|params| params.get("code"));

        let web_socket_query = WebSocketQuery {
            protocol: Some(common_settings.protocol),
            arena_id: common_settings.arena_id,
            session_id: common_settings.session_id,
            invitation_id: invitation_id(),
            login_type: oauth2_code.is_some().then_some(LoginType::Discord),
            login_id: oauth2_code,
            referrer: frontend.get_real_referrer(),
        };

        let web_socket_query_url = serde_urlencoded::to_string(&web_socket_query).unwrap();

        (
            format!("{}://{}/ws?{}", scheme, ideal_host, web_socket_query_url),
            ideal_server_id,
        )
    }

    /// Whether the game websocket is closed or errored (not open, opening, or nonexistent).
    pub fn connection_lost(&self) -> bool {
        self.socket.is_terminated()
    }

    /// Send a game command on the socket.
    pub fn send_to_game(&mut self, request: G::GameRequest) {
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
    pub fn send_to_server(&mut self, request: Request<G::GameRequest>) {
        self.socket.send(request);
    }

    /// Set the props used to render the UI. Javascript must implement part of this.
    pub fn set_ui_props(&mut self, props: G::UiProps) {
        self.frontend.set_ui_props(props);
    }
}

#[derive(Clone)]
pub struct WeakCoreState(std::rc::Weak<CoreState>);

impl Default for WeakCoreState {
    fn default() -> Self {
        thread_local! {
            static DEFAULT_CORE_STATE: Rc<CoreState> = Rc::default();
        }
        DEFAULT_CORE_STATE.with(Self::new) // Only allocate zero value once to not cause a leak.
    }
}

impl PartialEq for WeakCoreState {
    fn eq(&self, _other: &Self) -> bool {
        // std::ptr::eq(self, _other)
        false // Can't implement Eq because not reflexive but probably doesn't matter...
    }
}

impl WeakCoreState {
    /// Borrow the core state immutably. Unused for now.
    pub fn as_strong(&self) -> StrongCoreState {
        StrongCoreState {
            inner: self.0.upgrade().unwrap(),
            _spooky: PhantomData,
        }
    }

    /// Like [`Self::as_strong`] but consumes self and has a static lifetime.
    pub fn into_strong(self) -> StrongCoreState<'static> {
        StrongCoreState {
            inner: self.0.upgrade().unwrap(),
            _spooky: PhantomData,
        }
    }

    /// Create a [`WeakCoreState`] from a [`Rc<CoreState>`].
    pub fn new(core: &Rc<CoreState>) -> Self {
        Self(Rc::downgrade(core))
    }
}

pub struct StrongCoreState<'a> {
    inner: Rc<CoreState>,
    _spooky: PhantomData<&'a ()>,
}

impl<'a> std::ops::Deref for StrongCoreState<'a> {
    type Target = CoreState;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
