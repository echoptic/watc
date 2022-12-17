use std::{num::ParseIntError, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until, take_while},
    character::complete::{alpha1, alphanumeric1, char, digit1, one_of},
    combinator::{eof, map, map_res, opt, recognize, value},
    error::{FromExternalError, ParseError},
    multi::{many0, many0_count},
    number::complete::float,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use strum_macros::EnumString;

#[derive(Debug)]
pub enum Expr {
    Func(Func),
    Export(Export),
    Instr(Instr),
    Ident(String),
}

#[derive(Debug, EnumString, Clone, Copy)]
#[repr(u8)]
pub enum ValType {
    #[strum(serialize = "f64")]
    F64 = 0x7c,
    #[strum(serialize = "f32")]
    F32,
    #[strum(serialize = "i64")]
    I64,
    #[strum(serialize = "i32")]
    I32,
}

#[derive(Debug, EnumString, Clone, Copy)]
#[repr(u8)]
pub enum Instr {
    #[strum(serialize = "local.get")]
    LocalGet = 0x20,
    #[strum(serialize = "i32.add")]
    I32Add = 0x6a,
}

#[derive(Debug, Default)]
pub struct Func {
    pub name: String,
    pub params: Vec<(String, ValType)>,
    pub result: Option<ValType>,
    pub body: Vec<Expr>,
}

#[derive(Debug)]
pub struct Export {
    pub export_name: String,
    pub ident: String,
    pub ty: ExportType,
}

#[derive(Debug, EnumString, Clone, Copy)]
#[repr(u8)]
pub enum ExportType {
    #[strum(serialize = "func")]
    Func = 0,
}

#[derive(Debug, Default)]
pub struct Module {
    pub funcs: Vec<Func>,
    pub exports: Vec<Export>,
}

pub fn module<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, Module, E> {
    let mut module = Module::default();
    let (i, ()) = s_expr(preceded(
        ws(tag("module")),
        value(
            (),
            many0_count(map(ws(expr), |expr| match expr {
                Expr::Func(f) => module.funcs.push(f),
                Expr::Export(e) => module.exports.push(e),
                _ => unimplemented!(),
            })),
        ),
    ))(i)?;

    Ok((i, module))
}

fn expr<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, Expr, E> {
    alt((
        map(func, Expr::Func),
        map(export, Expr::Export),
        map(instr, Expr::Instr),
        map(identifier, |s| Expr::Ident(s.to_string())),
    ))(i)
}

fn func<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, Func, E> {
    map(
        s_expr(tuple((
            preceded(ws(tag("func")), identifier),
            many0(ws(param)),
            ws(opt(result)),
            many0(ws(expr)),
        ))),
        |(name, params, result, body)| Func {
            name: name.to_string(),
            params,
            result,
            body,
        },
    )(i)
}

fn valtype<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, ValType, E> {
    map_res(keyword, ValType::from_str)(i)
}

fn param<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, (String, ValType), E> {
    map(
        s_expr(tuple((preceded(ws(tag("param")), identifier), ws(valtype)))),
        |(name, ty)| (name.to_string(), ty),
    )(i)
}

fn result<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, ValType, E> {
    s_expr(preceded(ws(tag("result")), valtype))(i)
}

fn export<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, Export, E> {
    map(
        s_expr(tuple((
            (preceded(ws(tag("export")), ws(string))),
            s_expr(pair(map_res(ws(keyword), ExportType::from_str), identifier)),
        ))),
        |(name, (ty, ident))| Export {
            export_name: name.to_string(),
            ident: ident.to_string(),
            ty,
        },
    )(i)
}

fn s_expr<'a, F, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(ws(char('(')), inner, ws(char(')')))
}

fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(sp, inner, sp)
}

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E> {
    value(
        (),
        many0_count(alt((value((), one_of(" \u{09}\u{0A}\u{0D}")), comment))),
    )(i)
}

fn comment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E> {
    let linecomment = value(
        (),
        tuple((
            tag(";;"),
            many0_count(is_not("\u{0A}")),
            alt((tag("\u{0A}"), eof)),
        )),
    );
    let blockcomment = value((), tuple((tag("(;"), take_until(";)"), tag(";)"))));

    alt((linecomment, blockcomment))(i)
}

fn integer<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
    i: &'a str,
) -> IResult<&'a str, i32, E> {
    map_res(recognize(digit1), str::parse)(i)
}

fn hexfloat<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, f32, E> {
    preceded(tag("0x"), float)(i)
}

// TODO: Improve string parsing
fn string<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    delimited(char('"'), take_until("\""), char('"'))(i)
}

fn idchar<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c: char| c.is_ascii_alphanumeric() || "!#$%&′∗+−./‘:<=>?@\\^_ˋ∣~".contains(c))(
        i,
    )
}

fn keyword<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    recognize(pair(alpha1, idchar))(i)
}

fn identifier<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    preceded(char('$'), idchar)(i)
}

fn instr<'a, E: ParseError<&'a str> + FromExternalError<&'a str, strum::ParseError>>(
    i: &'a str,
) -> IResult<&'a str, Instr, E> {
    map_res(
        recognize(tuple((alphanumeric1, char('.'), alphanumeric1))),
        Instr::from_str,
    )(i)
}
