use rless::*;
use atty::Stream;
use std::fs::read_to_string;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (filename, pager) = arg_parser()?;
    if atty::is(Stream::Stdout) {
    	read_file(filename, pager.finish()).await;
	std::process::exit(0);
    } 
	let res = read_to_string(filename);	
	println!("{}", res.unwrap());
	Ok(())
}
