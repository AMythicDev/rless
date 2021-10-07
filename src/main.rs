use rless::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let (filename, pager) = arg_parser()?;
    read_file(filename, pager.finish(), &mut h, &ps).await
}
