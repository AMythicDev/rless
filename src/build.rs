use man::prelude::*;
use std::fs::File;
use std::io::{Error, Write};

fn main() -> Result<(), Error> {
    let path = "rless.1";
    let mut output = File::create(path)?;

    let msg = Manual::new("rless")
        .about("A pager in rust.")
        .arg(Arg::new("path"))
        .custom(Section::new("commands").paragraph(
            r#"
k      Up. Scroll up one line by default, otherwise, if a number is specified, scroll up N lines.

j      Down. Scroll down one line by default, otherwise, if a number is specified, scroll down N lines.

u      Undo. Go back to the location before the previous command.

r      Redo. Apply the last command that was undone."#,
        ))
        .example(
            Example::new()
                .text("Running the program")
                .command("rless numbers.txt")
                .output("TODO"),
        )
        .author(Author::new("Takashi I").email("mail@takashiidobe.com"))
        .render();

    write!(output, "{}", msg)
}
