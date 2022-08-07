// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use yew::virtual_dom::AttrValue;
use yew::{html, Children, Classes, Properties};

#[derive(PartialEq, Properties)]
pub struct PositionerProps {
    pub id: Option<AttrValue>,
    pub children: Children,
    pub position: Position,
    /// Override default alignment (horizontal position).
    pub align: Option<Align>,
    /// Use flex layout.
    pub flex: Option<Flex>,
    pub min_width: Option<AttrValue>,
    pub max_width: Option<AttrValue>,
    #[prop_or_default]
    pub class: Classes,
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
    pub fn default_text_align(&self) -> Align {
        self.horizontal().default_text_align()
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
            HorizontalPosition::Right { margin, .. } => {
                ("right: 0;", format!("margin-right: {};", margin), 0)
            }
        };

        style += h_position;
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
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    fn as_css(self) -> &'static str {
        match self {
            Align::Left => "text-align: left;",
            Align::Center => "text-align: center;",
            Align::Right => "text-align: right;",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Flex {
    Column,
    Row,
}

impl Flex {
    fn as_css(self) -> &'static str {
        match self {
            Flex::Column => "display: flex; flex-direction: column; gap: 0.5rem;",
            Flex::Row => "display: flex; flex-direction: row; gap: 0.75rem;",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum HorizontalPosition {
    Left { margin: &'static str },
    Middle,
    Right { margin: &'static str },
}

impl HorizontalPosition {
    fn default_text_align(self) -> Align {
        match self {
            Self::Left { .. } => Align::Left,
            Self::Middle => Align::Center,
            Self::Right { .. } => Align::Right,
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
            | Self::BottomRight { margin, .. } => VerticalPosition::Bottom { margin },
            Self::CenterLeft { .. } | Self::Center | Self::CenterRight { .. } => {
                VerticalPosition::Center
            }
            Self::TopLeft { margin }
            | Self::TopMiddle { margin }
            | Self::TopRight { margin, .. } => VerticalPosition::Top { margin },
        }
    }
}

#[styled_component(Positioner)]
pub fn positioner(props: &PositionerProps) -> Html {
    let mut style = props.position.to_string();

    if let Some(min_width) = props.min_width.as_ref() {
        style += &format!("min-width: {};", min_width);
    }

    if let Some(max_width) = props.max_width.as_ref() {
        style += &format!("max-width: {};", max_width);
    }

    if let Some(flex) = props.flex {
        style += flex.as_css();
    }

    style += props
        .align
        .unwrap_or(props.position.default_text_align())
        .as_css();

    html! {
        <div id={props.id.clone()} style={style} class={props.class.clone()}>
            {props.children.clone()}
        </div>
    }
}
