// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use yew::{html, Children, Properties};

#[derive(PartialEq, Properties)]
pub struct PositionerProps {
    pub children: Children,
    pub position: Position,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Position {
    BottomLeft { margin: &'static str },
    BottomMiddle { margin: &'static str },
    BottomRight { margin: &'static str },
    CenterLeft { margin: &'static str },
    Center,
    CenterRight { margin: &'static str },
    TopLeft { margin: &'static str },
    TopMiddle { margin: &'static str },
    TopRight { margin: &'static str },
}

impl Position {
    pub fn text_align(&self) -> &'static str {
        self.horizontal().text_align()
    }
}

impl ToString for Position {
    fn to_string(&self) -> String {
        let mut style = String::with_capacity(64);

        style += "position: absolute;";

        let (h_position, h_margin, h_translation) = match self.horizontal() {
            HorizontalPosition::Left { margin } => {
                ("left: 0;", format!("margin-left: {};", margin), 0)
            }
            HorizontalPosition::Middle => ("left: 50%;", String::new(), -50),
            HorizontalPosition::Right { margin } => {
                ("right: 0;", format!("margin-right: {};", margin), 0)
            }
        };

        style += h_position;
        style += self.horizontal().text_align();
        style += &h_margin;

        let (v_position, v_margin, v_translation) = match self.vertical() {
            VerticalPosition::Bottom { margin } => {
                ("bottom: 0;", format!("margin-bottom: {};", margin), 0)
            }
            VerticalPosition::Center => ("top: 50%;", String::new(), -50),
            VerticalPosition::Top { margin } => ("top: 0;", format!("margin-top: {};", margin), 0),
        };

        style += v_position;
        style += &v_margin;

        if h_translation != 0 || v_translation != 0 {
            style += &format!(
                "transform: translate({}%, {}%);",
                h_translation, v_translation
            );
        }

        style
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum HorizontalPosition {
    Left { margin: &'static str },
    Middle,
    Right { margin: &'static str },
}

impl HorizontalPosition {
    fn text_align(self) -> &'static str {
        match self {
            Self::Left { .. } => "text-align: left;",
            Self::Middle => "text-align: center;",
            Self::Right { .. } => "text-align: right;",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum VerticalPosition {
    Bottom { margin: &'static str },
    Center,
    Top { margin: &'static str },
}

impl Position {
    fn horizontal(self) -> HorizontalPosition {
        match self {
            Self::BottomLeft { margin }
            | Self::CenterLeft { margin }
            | Self::TopLeft { margin } => HorizontalPosition::Left { margin },
            Self::BottomMiddle { .. } | Self::Center | Self::TopMiddle { .. } => {
                HorizontalPosition::Middle
            }
            Self::BottomRight { margin }
            | Self::CenterRight { margin }
            | Self::TopRight { margin } => HorizontalPosition::Right { margin },
        }
    }

    fn vertical(self) -> VerticalPosition {
        match self {
            Self::BottomLeft { margin }
            | Self::BottomMiddle { margin }
            | Self::BottomRight { margin } => VerticalPosition::Bottom { margin },
            Self::CenterLeft { .. } | Self::Center | Self::CenterRight { .. } => {
                VerticalPosition::Center
            }
            Self::TopLeft { margin } | Self::TopMiddle { margin } | Self::TopRight { margin } => {
                VerticalPosition::Top { margin }
            }
        }
    }
}

#[styled_component(Positioner)]
pub fn positioner(props: &PositionerProps) -> Html {
    html! {
        <div style={props.position.to_string()}>
            {props.children.clone()}
        </div>
    }
}
