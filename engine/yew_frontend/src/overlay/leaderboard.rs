// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::section::Section;
use crate::translation::{t, Translation};
use crate::Ctw;
use core_protocol::id::PeriodId;
use std::ops::Deref;
use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct LeaderboardProps {
    pub children: Option<Children>,
    #[prop_or(LeaderboardProps::fmt_precise)]
    pub fmt_score: fn(u32) -> String,
}

impl LeaderboardProps {
    pub fn fmt_precise(score: u32) -> String {
        score.to_string()
    }

    pub fn fmt_abbreviated(score: u32) -> String {
        let power = score.max(1).log(1000);
        if power == 0 {
            score.to_string()
        } else {
            let units = ["", "k", "m", "b"];
            let power_of_1000 = 1000u32.pow(power);
            let unit = units[power as usize];
            let fraction = score as f32 / power_of_1000 as f32;
            format!("{:.1}{}", fraction, unit)
        }
    }
}

#[derive(Copy, Clone, Default)]
enum Mode {
    #[default]
    Liveboard,
    Leaderboard(PeriodId),
}

impl Mode {
    fn next(self) -> Self {
        match self {
            Self::Liveboard => Self::Leaderboard(PeriodId::Daily),
            Self::Leaderboard(period_id) => match period_id {
                PeriodId::Daily => Self::Leaderboard(PeriodId::Weekly),
                PeriodId::Weekly => Self::Leaderboard(PeriodId::AllTime),
                PeriodId::AllTime => Self::Liveboard,
            },
        }
    }
}

#[styled_component(LeaderboardOverlay)]
pub fn leaderboard_overlay(props: &LeaderboardProps) -> Html {
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

    let mode = use_state(Mode::default);

    let on_right_arrow = {
        let mode = mode.clone();
        Callback::from(move |_| {
            mode.set(mode.deref().next());
        })
    };

    let core_state = Ctw::use_core_state();

    let (name, items) = match *mode {
        Mode::Liveboard => {
            let name = t().liveboard_label();
            let items = core_state.liveboard.iter().filter_map(|dto| {
                core_state
                    .player_or_bot(dto.player_id)
                    .map(|player| {
                        let team_name = dto
                            .team_id
                            .and_then(|team_id| core_state.teams.get(&team_id))
                            .map(|team_dto| team_dto.name);
                        html_nested! {
                            <tr>
                                <td class="name">{team_name.map(|team_name| format!("[{}] {}", team_name, player.alias)).unwrap_or(player.alias.to_string())}</td>
                                <td class="score">{(props.fmt_score)(dto.score)}</td>
                            </tr>
                        }
                    })
            }).collect::<Html>();

            (name, items)
        }
        Mode::Leaderboard(period_id) => {
            let name = match period_id {
                PeriodId::AllTime => t().leaderboard_all_time_label(),
                PeriodId::Daily => t().leaderboard_daily_label(),
                PeriodId::Weekly => t().leaderboard_weekly_label(),
            };

            let items = core_state
                .leaderboard(period_id)
                .iter()
                .map(|dto| {
                    html_nested! {
                        <tr>
                            <td class="name">{dto.alias}</td>
                            <td class="score">{(props.fmt_score)(dto.score)}</td>
                        </tr>
                    }
                })
                .collect::<Html>();

            (name, items)
        }
    };

    // TODO: <Section ... bind:open={$leaderboardShown}>
    html! {
        <Section {name} {on_right_arrow}>
            <table class={table_css_class}>
                {items}
            </table>
            <p class={p_css_class}>
                if let Some(children) = props.children.as_ref() {
                    {children.clone()}
                } else {
                    {t().online(core_state.real_players)}
                }
            </p>
        </Section>
    }
}

/*
#[cfg(test)]
mod test {
    use crate::overlay::leaderboard::LeaderboardProps;

    #[test]
    fn fmt_abbreviated() {
        assert_eq!(LeaderboardProps::fmt_abbreviated(u32::MAX / 1000 / 1000), "");
        assert_eq!(LeaderboardProps::fmt_abbreviated(u32::MAX / 1000), "");
        assert_eq!(LeaderboardProps::fmt_abbreviated(u32::MAX), "");
    }
}
 */
