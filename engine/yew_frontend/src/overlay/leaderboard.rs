// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::section::Section;
use crate::translation::t;
//use core_protocol::name::PlayerAlias;
use crate::Ctw;
use core_protocol::name::{PlayerAlias, TeamName};
use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(Default, PartialEq, Properties)]
pub struct LeaderboardProps {
    pub children: Option<Children>,
    pub name: &'static str,
}

#[derive(PartialEq, Eq)]
pub struct LeaderboardTuple {
    pub name: PlayerAlias,
    pub score: u32,
    pub team: Option<TeamName>,
}

#[styled_component(LeaderboardOverlay)]
pub fn leaderboard_overlay(_props: &LeaderboardProps) -> Html {
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

    let core_state = Ctw::use_core_state();

    let mut content = Vec::new();

    content.extend(core_state.liveboard.iter().filter_map(|dto| {
        core_state
            .player_or_bot(dto.player_id)
            .map(|player| LeaderboardTuple {
                name: player.alias,
                score: dto.score,
                team: dto
                    .team_id
                    .and_then(|team_id| core_state.teams.get(&team_id))
                    .map(|team_dto| team_dto.name),
            })
    }));

    // TODO: <Section ... bind:open={$leaderboardShown}>
    html! {
        <Section name={leaderboard_name} on_right_arrow={on_next_index}>
            <table class={table_css_class}>
            {
                content.iter().map(|LeaderboardTuple{name, score, team}| {
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
