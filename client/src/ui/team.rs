// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::settings::Mk48Settings;
use crate::ui::UiEvent;
use common::protocol::{TeamDto, TeamRequest};
use itertools::Itertools;
use kodiak_client::{
    event_target, use_core_state, use_gctw, use_translator, use_ui_event_callback, BrowserStorages,
    PlayerDto, PlayerId, Position, Section, TeamId, TeamName, Translator,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use stylist::yew::styled_component;
use web_sys::{HtmlInputElement, InputEvent, SubmitEvent};
use yew::virtual_dom::AttrValue;
use yew::{classes, html, html_nested, use_node_ref, use_state, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct TeamOverlayProps {
    pub position: Position,
    #[prop_or(None)]
    pub style: Option<AttrValue>,
    /// Override the default label.
    #[prop_or(Translator::team_label)]
    pub label: fn(&Translator) -> String,
    /// Override the default placeholder.
    #[prop_or(Translator::team_name_placeholder)]
    pub name_placeholder: fn(&Translator) -> String,
    #[prop_or_default]
    pub team_proximity: HashMap<TeamId, f32>,
    pub teams: HashMap<TeamId, TeamDto>,
    pub members: Box<[PlayerId]>,
    pub joiners: Box<[PlayerId]>,
    pub joins: Box<[TeamId]>,
}

#[styled_component(TeamOverlay)]
pub fn team_overlay(props: &TeamOverlayProps) -> Html {
    let button_css_class = css!(
        r#"
        border-radius: 0.25rem;
        box-sizing: border-box;
        color: white;
        cursor: pointer;
        font-size: 1rem;
        margin-top: 0.25rem;
        text-decoration: none;
        white-space: nowrap;
        background-color: transparent;
        border: 0;
        width: min-content;
        padding: 0.1rem 0.5rem;

        :disabled {
            opacity: 0.6;
            cursor: initial;
        }

        :hover:not(:disabled) {
            background-color: #00000025;
        }
        "#
    );

    let hidden_css_class = css!(
        r#"
        visibility: hidden;
        "#
    );

    let disabled_css_class = css!(
        r#"
        opacity: 0.6;
        cursor: initial;
        "#
    );

    let input_css_class = css!(
        r#"
        border-radius: 0.25rem;
        box-sizing: border-box;
        cursor: pointer;
        font-size: 1rem;
        font-weight: bold;
        outline: 0;
        padding: 0.5rem;
        pointer-events: all;
        white-space: nowrap;
        margin-top: 0.25rem;
        background-color: #00000025;
        border: 0;
        color: white;
        width: 9rem;

        @media (max-width: 600px) {
            width: 5rem;
        }
        "#
    );

    let table_css_class = css!(
        r#"
        color: white;
        width: 100%;
        "#
    );

    let tr_css_class = css!(
        r#"
        margin-top: 0.25rem;
        margin-bottom: 0.25rem;
        "#
    );

    let name_css_class = css!(
        r#"
        color: white;
        cursor: pointer;
        font-weight: bold;
        white-space: nowrap;
    "#
    );

    let name_pending_css_class = css!(
        r#"
        filter: brightness(0.7);
    "#
    );

    let owner_css_class = css!(
        r#"
        text-decoration: underline;
    "#
    );

    let gctw = use_gctw::<Mk48Game>();
    let t = use_translator();
    let core_state = use_core_state();
    let team_id = core_state.team_id();
    let team = team_id.and_then(|team_id| props.teams.get(&team_id));
    let team_name = team.map(|t| t.name);
    let i_am_team_captain = props.members.get(0) == core_state.player_id.as_ref();
    let team_full = team.map(|t| t.full).unwrap_or(false);
    let team_request_callback = use_ui_event_callback::<Mk48Game>().reform(UiEvent::Team);
    let input_ref = use_node_ref();

    // KLUDGE: Avoid use state eq so input triggers team_name_team_id to be reconsidered.
    let team_name_empty = use_state(|| true);
    let team_name_team_id = if let Some(input) = input_ref.cast::<HtmlInputElement>() {
        let team_name = input.value();
        props
            .teams
            .iter()
            .find(|(_, dto)| dto.name.as_str() == &team_name)
            .map(|(id, _)| *id)
    } else {
        None
    };

    let on_open_changed = gctw.change_settings_callback.reform(|open| {
        Box::new(
            move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                settings.set_team_dialog_shown(open, browser_storages);
            },
        )
    });

    let on_new_team_name_change = {
        let team_name_empty = team_name_empty.clone();
        move |event: InputEvent| {
            if !event.is_composing() {
                let input: HtmlInputElement = event_target(&event);
                team_name_empty.set(input.value().is_empty());
            }
        }
    };

    let on_accept_join_team = {
        let cb = team_request_callback.clone();
        move |player_id: PlayerId| {
            cb.emit(TeamRequest::Accept(player_id));
        }
    };

    let on_create_team = {
        let cb = team_request_callback.clone();
        let input_ref = input_ref.clone();
        move || {
            if let Some(team_name_team_id) = team_name_team_id {
                cb.emit(TeamRequest::Join(team_name_team_id));
            } else if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                let new_team_name = input.value();
                if !new_team_name.is_empty() {
                    cb.emit(TeamRequest::Create(TeamName::new_input_sanitized(
                        &new_team_name,
                    )));
                }
            }
        }
    };

    let on_kick_from_team = {
        let cb = team_request_callback.clone();
        move |player_id: PlayerId| {
            cb.emit(TeamRequest::Kick(player_id));
        }
    };

    let on_leave_team = {
        let cb = team_request_callback.clone();
        move || cb.emit(TeamRequest::Leave)
    };

    let on_reject_join_team = {
        let cb = team_request_callback.clone();
        move |player_id: PlayerId| {
            cb.emit(TeamRequest::Reject(player_id));
        }
    };

    let on_request_join_team = {
        let cb = team_request_callback;
        move |team_id: TeamId| {
            cb.emit(TeamRequest::Join(team_id));
        }
    };

    let seed = core_state
        .player_id
        .map(|player_id| player_id.0.get())
        .unwrap_or(0);
    let cmp_teams =
        |&(a, team_a): &(&TeamId, &TeamDto), &(b, team_b): &(&TeamId, &TeamDto)| -> Ordering {
            team_a
                .closed
                .cmp(&team_b.closed)
                .then(team_a.full.cmp(&team_b.full))
                .then_with(|| {
                    props
                        .team_proximity
                        .get(a)
                        .unwrap_or(&f32::INFINITY)
                        .partial_cmp(props.team_proximity.get(b).unwrap_or(&f32::INFINITY))
                        .unwrap_or_else(|| {
                            debug_assert!(false, "NaN team proximity");
                            Ordering::Equal
                        })
                        .then_with(|| {
                            // Use a seed so different players see a different set of options.
                            (a.0.get() ^ seed).cmp(&(b.0.get() ^ seed))
                        })
                })
        };

    const CHECK_MARK: &str = "✔";
    const X_MARK: &str = "✘";

    html! {
        <Section
            id="team"
            name={team_name.map(|n| AttrValue::Rc(n.to_string().into())).unwrap_or(AttrValue::Rc((props.label)(&t).into()))}
            position={props.position}
            style={props.style.clone()}
            open={gctw.settings_cache.team_dialog_shown}
            {on_open_changed}
        >
            if team_name.is_some() {
                <table class={table_css_class}>
                    {props.members.iter().enumerate().filter_map(|(i, player_id)| core_state.player_or_bot(*player_id).map(|p| (i, p))).map(|(i, PlayerDto{alias, player_id, ..})| {
                        let team_captain = i == 0;
                        let on_kick_from_team = on_kick_from_team.clone();

                        html_nested!{
                            <tr class={tr_css_class.clone()}>
                                <td class={classes!(name_css_class.clone(), team_captain.then(|| owner_css_class.clone()))}>{alias.to_string()}</td>
                                if i_am_team_captain {
                                    <td><button class={classes!(button_css_class.clone(), hidden_css_class.clone())}>{CHECK_MARK}</button></td>
                                    <td><button class={classes!(button_css_class.clone(), team_captain.then(|| hidden_css_class.clone()))} onclick={move |_| on_kick_from_team(player_id)} title={t.team_kick_hint()}>{X_MARK}</button></td>
                                }
                            </tr>
                        }
                    }).collect::<Html>()}
                    {props.joiners.iter().filter_map(|player_id| core_state.player_or_bot(*player_id)).map(|PlayerDto{alias, player_id, ..}| {
                        let on_accept_join_team = on_accept_join_team.clone();
                        let on_reject_join_team = on_reject_join_team.clone();
                        html_nested!{
                            <tr class={tr_css_class.clone()}>
                                <td class={classes!(name_css_class.clone(), name_pending_css_class.clone())}>{alias.to_string()}</td>
                                <td><button class={classes!(button_css_class.clone(), team_full.then(|| disabled_css_class.clone()))} onclick={move |_| on_accept_join_team(player_id)} title={t.team_accept_hint()}>{CHECK_MARK}</button></td>
                                <td><button class={button_css_class.clone()} onclick={move |_| on_reject_join_team(player_id)} title={t.team_deny_hint()}>{X_MARK}</button></td>
                            </tr>
                        }
                    }).collect::<Html>()}
                </table>
                <button onclick={move |_| on_leave_team()} class={button_css_class}>{t.team_leave_hint()}</button>
            } else {
                <form onsubmit={move |e: SubmitEvent| {e.prevent_default(); on_create_team();}}>
                    <table>
                        {props.teams.iter().sorted_by(cmp_teams).take(5).map(|(_, &TeamDto{closed, name, team_id, ..})| {
                            let on_request_join_team = on_request_join_team.clone();
                            let unavailable = closed || props.joins.contains(&team_id);

                            html_nested!{
                                <tr>
                                    <td class={name_css_class.clone()}>{name.to_string()}</td>
                                    <td>
                                        <button
                                            type="button"
                                            class={classes!(
                                                button_css_class.clone(),
                                                unavailable.then(|| hidden_css_class.clone())
                                            )}
                                            onclick={move |_| on_request_join_team(team_id)}
                                        >
                                            {t.team_request_hint()}
                                        </button>
                                    </td>
                                </tr>
                            }
                        }).collect::<Html>()}
                        <tr>
                            <td>
                                <input
                                    ref={input_ref}
                                    type="text"
                                    minlength="1"
                                    maxlength="6"
                                    placeholder={(props.name_placeholder)(&t)}
                                    oninput={on_new_team_name_change}
                                    class={input_css_class}
                                />
                            </td>
                            <td>
                                <button
                                    disabled={*team_name_empty}
                                    class={button_css_class}
                                >
                                    {if team_name_team_id.is_some() {
                                        t.team_request_hint()
                                    } else {
                                        t.team_create_hint()
                                    }}
                                </button>
                            </td>
                        </tr>
                    </table>
                </form>
            }
        </Section>
    }
}
