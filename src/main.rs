use rless::*;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (filename, pager) = arg_parser()?;
    read_file(filename, pager.finish()).await
}
