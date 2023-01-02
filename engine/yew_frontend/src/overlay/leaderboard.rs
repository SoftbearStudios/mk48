// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::positioner::Position;
use crate::component::section::{Section, SectionArrow};
use crate::frontend::{use_core_state, use_ctw};
use crate::translation::Translation;
use client_util::browser_storage::BrowserStorages;
use client_util::setting::CommonSettings;
use core_protocol::dto::LiveboardDto;
use core_protocol::id::{LanguageId, PeriodId};
use std::ops::Deref;
use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct LeaderboardProps {
    #[prop_or(None)]
    pub position: Option<Position>,
    #[prop_or(None)]
    pub style: Option<AttrValue>,
    /// Override the default liveboard label.
    #[prop_or(LanguageId::liveboard_label)]
    pub liveboard_label: fn(LanguageId) -> &'static str,
    /// Override the default leaderboard label.
    #[prop_or(LanguageId::leaderboard_label)]
    pub leaderboard_label: fn(LanguageId, PeriodId) -> &'static str,
    /// If Some, this score will be attributed to the local player.
    pub show_my_score: Option<u32>,
    #[prop_or(true)]
    pub mode_arrow: bool,
    pub children: Option<Children>,
    #[prop_or(LeaderboardProps::fmt_precise)]
    pub fmt_score: fn(u32) -> String,
}

impl LeaderboardProps {
    pub fn fmt_precise(score: u32) -> String {
        score.to_string()
    }

    pub fn fmt_abbreviated(score: u32) -> String {
        let power = score.max(1).ilog(1000);
        if power == 0 {
            score.to_string()
        } else {
            let units = ["", "k", "m", "b"];
            let power_of_1000 = 1000u32.pow(power);
            let unit = units[power as usize];
            // TODO: Round down not up.
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
        margin-bottom: 0rem;
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

    let fake_style = css!(
        r#"
        opacity: 0.6;
        "#
    );

    let ctw = use_ctw();
    let on_open_changed = ctw.change_common_settings_callback.reform(|open| {
        Box::new(
            move |common_settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                common_settings.set_leaderboard_dialog_shown(open, browser_storages);
            },
        )
    });

    let mode = use_state(Mode::default);

    let right_arrow = if props.mode_arrow {
        let mode = mode.clone();
        SectionArrow::always(Callback::from(move |_| {
            mode.set(mode.deref().next());
        }))
    } else {
        SectionArrow::None
    };

    let t = ctw.setting_cache.language;
    let core_state = use_core_state();

    let (name, items) = match *mode {
        Mode::Liveboard => {
            let name = (props.liveboard_label)(t);
            let extra = props
                .show_my_score
                .zip(core_state.player().filter(|player| {
                    core_state
                        .liveboard
                        .iter()
                        .all(|dto| dto.player_id != player.player_id)
                }))
                .map(|(score, player)| {
                    (
                        LiveboardDto {
                            player_id: player.player_id,
                            score,
                            team_captain: player.team_captain,
                            team_id: player.team_id,
                        },
                        true,
                    )
                });
            let items = core_state.liveboard.iter().map(|dto| (dto.clone(), false)).chain(extra).filter_map(|(dto, fake)| {
                core_state
                    .player_or_bot(dto.player_id)
                    .map(|player| {
                        let team_name = dto
                            .team_id
                            .and_then(|team_id| core_state.teams.get(&team_id))
                            .map(|team_dto| team_dto.name);
                        html_nested! {
                            <tr class={fake.then(|| fake_style.clone())}>
                                <td class="name">{team_name.map(|team_name| format!("[{}] {}", team_name, player.alias)).unwrap_or(player.alias.to_string())}</td>
                                <td class="score">{(props.fmt_score)(dto.score)}</td>
                            </tr>
                        }
                    })
            }).collect::<Html>();

            (name, items)
        }
        Mode::Leaderboard(period_id) => {
            let name = (props.leaderboard_label)(t, period_id);

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
        <Section
            id="leaderboard"
            {name}
            position={props.position}
            style={props.style.clone()}
            {right_arrow}
            open={ctw.setting_cache.leaderboard_dialog_shown}
            {on_open_changed}
        >
            <table class={table_css_class}>
                {items}
            </table>
            <p class={p_css_class}>
                if let Some(children) = props.children.as_ref() {
                    {children.clone()}
                } else {
                    {t.online(core_state.real_players)}
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
