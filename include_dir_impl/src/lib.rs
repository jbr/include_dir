//! Implementation crate for the [include_dir!()] macro.
//!
//! [include_dir!()]: https://github.com/Michael-F-Bryan/include_dir

use crate::dir::Dir;
use proc_macro::{TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::{
    borrow::Cow,
    env,
    path::{Path, PathBuf, StripPrefixError},
    time::{SystemTime, UNIX_EPOCH},
};

mod dir;
mod file;

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Str(&'static str),
    StripPrefix(StripPrefixError),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(i) => f.write_fmt(format_args!("{}", i)),
            Error::Str(s) => f.write_str(s),
            Error::StripPrefix(s) => f.write_fmt(format_args!("{}", s)),
        }
    }
}

impl From<StripPrefixError> for Error {
    fn from(s: StripPrefixError) -> Self {
        Self::StripPrefix(s)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Self::Str(s)
    }
}

#[proc_macro]
pub fn include_dir(input: TokenStream) -> TokenStream {
    let path = tokens_to_path(input).unwrap();

    Dir::from_disk(&path, &path)
        .expect("Couldn't load the directory")
        .to_token_stream()
        .into()
}

fn unwrap_string_literal(lit: &proc_macro::Literal) -> Result<String, &'static str> {
    let mut repr = lit.to_string();
    if !repr.starts_with('"') || !repr.ends_with('"') {
        return Err("This macro only accepts a single, non-empty string argument");
    }

    repr.remove(0);
    repr.pop();

    Ok(repr)
}

fn match_path(tokens: Vec<TokenTree>) -> Result<String, &'static str> {
    match tokens.as_slice() {
        [TokenTree::Literal(lit)] => unwrap_string_literal(lit),

        [TokenTree::Group(group)] => match_path(group.stream().into_iter().collect()),

        _ => return Err("This macro only accepts a single, non-empty string argument".into()),
    }
}

fn tokens_to_path(input: TokenStream) -> Result<PathBuf, Cow<'static, str>> {
    let path = match_path(input.into_iter().collect())?;
    let crate_root =
        env::var("CARGO_MANIFEST_DIR").map_err(|_| "cannot find cargo manifest dir")?;
    let path = PathBuf::from(crate_root).join(path);

    if !path.exists() {
        return Err(format!("\"{}\" doesn't exist", path.display()).into());
    }

    Ok(path
        .canonicalize()
        .map_err(|_| "Can't normalize the path")?)
}

#[proc_macro]
pub fn try_include_dir(input: TokenStream) -> TokenStream {
    let path = match tokens_to_path(input) {
        Ok(path) => path,
        Err(e) => return quote! { Result::<Dir, &'static str>::Err(#e) }.into(),
    };

    TokenStream::from(match load_dir(&path) {
        Ok(dir) => quote! { Result::<Dir, &'static str>::Ok(#dir) },
        Err(err) => quote! { Result::<Dir, &'static str>::Err(#err) },
    })
}

fn load_dir(path: impl AsRef<Path>) -> Result<Dir, String> {
    let path = path.as_ref();

    Dir::from_disk(&path, &path)
        .map_err(|e| format!("Couldn't load the directory: {}", e.to_string()))
}

pub(crate) fn timestamp_to_tokenstream(
    time: std::io::Result<SystemTime>,
) -> proc_macro2::TokenStream {
    time.ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs_f64())
        .map(|secs| quote! { Some(#secs) }.to_token_stream())
        .unwrap_or_else(|| quote! { None }.to_token_stream())
}
