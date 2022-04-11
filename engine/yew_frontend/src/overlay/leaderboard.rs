// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::section::{Section, TextAlign};
use crate::translation::t;
//use core_protocol::name::PlayerAlias;
use core_protocol::name::{PlayerAlias, TeamName};
use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(Default, PartialEq, Properties)]
pub struct LeaderboardProps {
    pub children: Children,
    pub content: Vec<LeaderboardTuple>,
    pub name: &'static str,
}

#[derive(PartialEq, Eq)]
pub struct LeaderboardTuple {
    pub name: PlayerAlias,
    pub score: u32,
    pub team: Option<TeamName>,
}

#[styled_component(LeaderboardOverlay)]
pub fn leaderboard_overlay(props: &LeaderboardProps) -> Html {
    let leaderboard_css_class = css!(
        r#"
        max-width: 25%;
        padding-right: 1rem;
        padding-top: 1rem;
        position: absolute;
        right: 0;
        text-align: right;
        top: 0;
    "#
    );

    let p_css_class = css!(
        r#"
        color: white;
        font-style: italic;
        margin-bottom: 1.4rem;
        margin-top: 0.5rem;
        text-align: center;
    "#
    );

    let table_css_class = css!(
        r#"
        color: white;
        width: 100%;

        td.name {
            font-weight: bold;
            text-align: left;
        }

        td.score {
            text-align: right;
        }
    "#
    );

    let footer = false; // TODO
    let leaderboard_index = 0; // TODO
    let leaderboard_name = get_leaderboard_name(leaderboard_index);

    let leaderboard_index = use_state(|| 0);

    let on_next_index = {
        let leaderboard_index = leaderboard_index.clone();
        Callback::from(move |_| {
            leaderboard_index.set(get_next_index(*leaderboard_index));
        })
    };

    // TODO: <Section ... bind:open={$leaderboardShown}>
    html! {
        <div id="leaderboard" class={leaderboard_css_class}>
            <Section name={leaderboard_name} header_align={TextAlign::Right} on_right_arrow={on_next_index}>
                <table class={table_css_class}>
                {
                    props.content.iter().map(|LeaderboardTuple{name, score, team}| {
                        html!{
                            <tr>
                                <td class="name">{team.map(|team_name| format!("[{}] {}", team_name, name)).unwrap_or(name.to_string())}</td>
                                <td class="score">{score}</td>
                            </tr>
                        }
                    }).collect::<Html>()
                }
                </table>
                if footer {
                    <p class={p_css_class}>{footer}</p>
                }
            </Section>
        </div>
    }
}

fn get_leaderboard_name(index: usize) -> &'static str {
    match index {
        1 => t().panel_leaderboard_day(),
        2 => t().panel_leaderboard_week(),
        3 => t().panel_leaderboard_all(),
        _ => t().panel_leaderboard_label(),
    }
}

fn get_next_index(index: usize) -> usize {
    if index < 3 {
        index + 1
    } else {
        0
    }
}
