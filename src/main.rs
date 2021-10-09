use rless::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use std::ffi::OsStr;
use std::path::Path;

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = set_prompt();
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let (filename, pager) = arg_parser()?;
    let extension = get_extension_from_filename(&filename).unwrap_or(".txt");
    let syntax = ps.find_syntax_by_extension(extension).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    read_file(filename, pager.finish(), &mut h, &ps).await
}
