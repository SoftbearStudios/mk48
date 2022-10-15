// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::section::Section;
use crate::event::event_target;
use crate::translation::{t, Translation};
use crate::Ctw;
use core_protocol::dto::{PlayerDto, TeamDto};
use core_protocol::id::{LanguageId, PlayerId, TeamId};
use core_protocol::name::TeamName;
use core_protocol::rpc::TeamRequest;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::HashMap;
use stylist::yew::styled_component;
use web_sys::{FocusEvent, HtmlInputElement, InputEvent};
use yew::{classes, html, html_nested, use_state, virtual_dom::AttrValue, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct TeamOverlayProps {
    /// Override the default label.
    #[prop_or(LanguageId::team_label)]
    pub label: fn(LanguageId) -> &'static str,
    /// Override the default placeholder.
    #[prop_or(LanguageId::team_name_placeholder)]
    pub name_placeholder: fn(LanguageId) -> &'static str,
    #[prop_or_default]
    pub team_proximity: HashMap<TeamId, f32>,
}

/// TODO: Rename to TeamOverlay.
#[styled_component(TeamsOverlay)]
pub fn team_overlay(props: &TeamOverlayProps) -> Html {
    let button_css_class = css!(
        r#"
        border-radius: 0.25em;
        box-sizing: border-box;
        color: white;
        cursor: pointer;
        font-size: 1em;
        margin-top: 0.5em;
        text-decoration: none;
        white-space: nowrap;
        background-color: transparent;
        border: 0;
        width: min-content;
        padding: 0.1em 0.5em;

        :disabled {
            opacity: 0.6;
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
        "#
    );

    let input_css_class = css!(
        r#"
        border-radius: 0.25em;
        box-sizing: border-box;
        cursor: pointer;
        font-size: 1em;
        font-weight: bold;
        outline: 0;
        padding: 0.5em;
        pointer-events: all;
        white-space: nowrap;
        margin-top: 0.25em;
        background-color: #00000025;
        border: 0;
        color: white;
        width: 9em;
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
        margin-top: 0.25em;
        margin-bottom: 0.25em;
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

    let t = t();
    let ctw = Ctw::use_ctw();
    let core_state = Ctw::use_core_state();
    let team_id = core_state.team_id();
    let team = team_id.and_then(|team_id| core_state.teams.get(&team_id));
    let team_name = team.map(|t| t.name);
    let i_am_team_captain = core_state.player().map(|p| p.team_captain).unwrap_or(false);
    let team_request_callback = Ctw::use_team_request_callback();
    let new_team_name = use_state(|| TeamName::new_unsanitized(""));

    let on_new_team_name_change = {
        let new_team_name = new_team_name.clone();
        move |event: InputEvent| {
            let input: HtmlInputElement = event_target(&event);
            new_team_name.set(TeamName::new_input_sanitized(&input.value()));
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
        let new_team_name = new_team_name.clone();
        move || {
            if !new_team_name.is_empty() {
                cb.emit(TeamRequest::Create(*new_team_name));
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
        let cb = team_request_callback.clone();
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

    const CHECK_MARK: &'static str = "✔";
    const X_MARK: &'static str = "✘";

    // TODO (use settings): on_open_changed={|o| ctw.dialogs.teams = o}}
    html! {
        <Section name={team_name.map(|n| AttrValue::Owned(n.to_string())).unwrap_or(AttrValue::Static((props.label)(t)))} open={ctw.setting_cache.team_dialog_shown}>
            if team_name.is_some() {
                <table class={table_css_class}>
                    {core_state.members.iter().filter_map(|player_id| core_state.player_or_bot(*player_id)).map(|PlayerDto{alias, player_id, team_captain, ..}| {
                        let on_kick_from_team = on_kick_from_team.clone();

                        html_nested!{
                            <tr class={tr_css_class.clone()}>
                                <td class={classes!(name_css_class.clone(), team_captain.then(|| owner_css_class.clone()))}>{alias}</td>
                                if i_am_team_captain {
                                    <td><button class={classes!(button_css_class.clone(), hidden_css_class.clone())}>{CHECK_MARK}</button></td>
                                    <td><button class={classes!(button_css_class.clone(), team_captain.then(|| hidden_css_class.clone()))} onclick={move |_| on_kick_from_team(player_id)} title={t.team_kick_hint()}>{X_MARK}</button></td>
                                }
                            </tr>
                        }
                    }).collect::<Html>()}
                    {core_state.joiners.iter().filter_map(|player_id| core_state.player_or_bot(*player_id)).map(|PlayerDto{alias, player_id, ..}| {
                        let on_accept_join_team = on_accept_join_team.clone();
                        let on_reject_join_team = on_reject_join_team.clone();
                        html_nested!{
                            <tr class={tr_css_class.clone()}>
                                <td class={classes!(name_css_class.clone(), name_pending_css_class.clone())}>{alias}</td>
                                <td><button class={classes!(button_css_class.clone(), false.then(|| disabled_css_class.clone()))} onclick={move |_| on_accept_join_team(player_id)} title={t.team_accept_hint()}>{CHECK_MARK}</button></td>
                                <td><button class={button_css_class.clone()} onclick={move |_| on_reject_join_team(player_id)} title={t.team_deny_hint()}>{X_MARK}</button></td>
                            </tr>
                        }
                    }).collect::<Html>()}
                </table>
                <button onclick={move |_| on_leave_team()} class={button_css_class}>{t.team_leave_hint()}</button>
            } else {
                <form onsubmit={move |e: FocusEvent| {e.prevent_default(); on_create_team();}}>
                    <table>
                        {core_state.teams.iter().sorted_by(cmp_teams).take(5).map(|(_, &TeamDto{closed, name, team_id, ..})| {
                            let on_request_join_team = on_request_join_team.clone();
                            let unavailable = closed || core_state.joins.contains(&team_id);

                            html_nested!{
                                <tr>
                                    <td class={name_css_class.clone()}>{name}</td>
                                    <td>
                                        <button type="button" class={classes!(button_css_class.clone(), unavailable.then(|| hidden_css_class.clone()))} onclick={move |_| on_request_join_team(team_id)}>{t.team_request_hint()}</button>
                                    </td>
                                </tr>
                            }
                        }).collect::<Html>()}
                        <tr>
                            <td>
                                <input type="text" placeholder={(props.name_placeholder)(t)} oninput={on_new_team_name_change} value={new_team_name.to_string()} class={input_css_class}/>
                            </td>
                            <td>
                                <button disabled={new_team_name.is_empty()} class={button_css_class}>{t.team_create_hint()}</button>
                            </td>
                        </tr>
                    </table>
                </form>
            }
        </Section>
    }
}
