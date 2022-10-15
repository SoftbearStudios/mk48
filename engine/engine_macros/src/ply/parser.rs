// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use nom::branch::alt;
use nom::bytes::complete::take_while;
use nom::character::complete::{digit1, not_line_ending, space1};
use nom::combinator::{all_consuming, map};
use nom::error::{Error, ErrorKind, ParseError};
use nom::multi::{many0, many1, many_m_n, separated_list1};
use nom::sequence::{delimited, terminated};
use nom::{bytes::complete::tag, character::complete::line_ending, IResult};
use std::str::FromStr;

#[derive(Debug)]
pub(crate) struct Ply {
    #[allow(unused)]
    pub format: Format,
    #[allow(unused)]
    pub comments: Vec<Comment>,
    pub elements: Vec<Element>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Format {
    Ascii,
    Binary,
}

impl Format {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            map(tag("ascii"), |_| Self::Ascii),
            map(tag("binary"), |_| Self::Binary),
        ))(i)
    }
}

#[derive(Debug)]
pub(crate) struct Comment(pub String);

impl Comment {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, text) = delimited(tag("comment "), not_line_ending, many1(line_ending))(i)?;
        Ok((i, Self(String::from(text))))
    }
}

#[derive(Debug)]
pub(crate) struct Element {
    pub _type: ElementType,
    pub count: usize,
    pub properties: Vec<Property>,
    pub data: Vec<Vec<f64>>,
}

impl Element {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, _) = terminated(tag("element"), space1)(i)?;
        let (i, _type) = terminated(ElementType::parse, space1)(i)?;
        let (i, count_str) = terminated(digit1, many1(line_ending))(i)?;
        let count = usize::from_str(count_str).map_err(|_| {
            nom::Err::Error(nom::error::Error::from_error_kind(i, ErrorKind::Digit))
        })?;
        let (i, properties) = many1(Property::parse)(i)?;

        Ok((
            i,
            Self {
                _type,
                count,
                properties,
                data: Default::default(),
            },
        ))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ElementType {
    Face,
    Vertex,
}

impl ElementType {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            map(tag("face"), |_| Self::Face),
            map(tag("vertex"), |_| Self::Vertex),
        ))(i)
    }
}

#[derive(Debug)]
pub(crate) struct Property {
    pub _type: PropertyType,
    pub name: String,
}

impl Property {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, _) = terminated(tag("property"), space1)(i)?;
        let (i, _type) = terminated(PropertyType::parse, space1)(i)?;
        let (i, name) = terminated(
            take_while(|c: char| c.is_alphanumeric() || matches!(c, '_')),
            many1(line_ending),
        )(i)?;

        Ok((
            i,
            Self {
                _type,
                name: String::from(name),
            },
        ))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum PropertyType {
    List(PropertyListType),
    Scalar(PropertyScalarType),
}

impl PropertyType {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            map(PropertyListType::parse, Self::List),
            map(PropertyScalarType::parse, Self::Scalar),
        ))(i)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct PropertyListType {
    pub index: PropertyScalarType,
    pub _type: PropertyScalarType,
}

impl PropertyListType {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, _) = terminated(tag("list"), space1)(i)?;
        let (i, index) = PropertyScalarType::parse(i)?;
        let (i, _) = space1(i)?;
        let (i, _type) = PropertyScalarType::parse(i)?;

        Ok((i, Self { index, _type }))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum PropertyScalarType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl PropertyScalarType {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            map(alt((tag("int8"), tag("char"))), |_| Self::Int8),
            map(alt((tag("uint8"), tag("uchar"))), |_| Self::Uint8),
            map(alt((tag("int16"), tag("short"))), |_| Self::Int16),
            map(alt((tag("uint16"), tag("ushort"))), |_| Self::Uint16),
            map(alt((tag("int32"), tag("int"))), |_| Self::Int32),
            map(alt((tag("uint32"), tag("uint"))), |_| Self::Uint32),
            map(alt((tag("float32"), tag("float"))), |_| Self::Float32),
            map(alt((tag("float64"), tag("double"))), |_| Self::Float64),
        ))(i)
    }
}

impl Ply {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, _) = terminated(tag("ply"), many1(line_ending))(i)?;
        let (i, format) = terminated(
            delimited(tag("format "), Format::parse, tag(" 1.0")),
            many1(line_ending),
        )(i)?;

        enum CommentOrElement {
            Comment(Comment),
            Element(Element),
        }
        let (i, comments_and_elements) = many0(alt((
            map(Comment::parse, CommentOrElement::Comment),
            map(Element::parse, CommentOrElement::Element),
        )))(i)?;

        let mut comments = Vec::new();
        let mut elements = Vec::new();

        for comment_or_element in comments_and_elements {
            match comment_or_element {
                CommentOrElement::Comment(comment) => comments.push(comment),
                CommentOrElement::Element(element) => elements.push(element),
            }
        }

        let (mut i, _) = terminated(tag("end_header"), many1(line_ending))(i)?;

        match format {
            Format::Ascii => {
                for element in &mut elements {
                    let (new_i, mut data) = many_m_n(
                        element.count,
                        element.count,
                        terminated(
                            separated_list1(space1, nom::number::complete::double),
                            line_ending,
                        ),
                    )(i)?;
                    i = new_i;
                    for item in &mut data {
                        match &element.properties[0]._type {
                            PropertyType::List(_list) => {
                                let length = item.remove(0);
                                let expected_length = length as usize;
                                if length.fract() != 0.0 || item.len() != expected_length {
                                    return Err(nom::Err::Error(Error::from_error_kind(
                                        i,
                                        ErrorKind::LengthValue,
                                    )));
                                }
                            }
                            PropertyType::Scalar(_scalar) => {
                                let scalars = element.properties.len();
                                if item.len() != scalars {
                                    return Err(nom::Err::Error(Error::from_error_kind(
                                        i,
                                        ErrorKind::LengthValue,
                                    )));
                                }
                            }
                        };
                    }
                    element.data = data;
                }
            }
            Format::Binary => {
                return Err(nom::Err::Error(Error::from_error_kind(
                    i,
                    ErrorKind::NoneOf,
                )));
            }
        }

        Ok((
            i,
            Self {
                format,
                comments,
                elements,
            },
        ))
    }
}

impl FromStr for Ply {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        all_consuming(Ply::parse)(s)
            .map(|(_, ply)| ply)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Ply;
    use std::str::FromStr;

    #[test]
    fn ply() {
        let src = r#"ply
format ascii 1.0
comment made by anonymous
comment this file is a cube
element vertex 8
property float32 x
property float32 y
property float32 z
element face 6
property list uint8 int32 vertex_index
end_header
0 0 0
0 0 1
0 1 1
0 1 0
1 0 0
1 0 1
1 1 1
1 1 0
4 0 1 2 3
4 7 6 5 4
4 0 4 5 1
4 1 5 6 2
4 2 6 7 3
4 3 7 4 0
"#;

        println!("{:?}", Ply::from_str(src));
    }
}
