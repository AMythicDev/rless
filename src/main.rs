use std::{collections::BTreeMap, path::PathBuf, vec::IntoIter};

use clap::Parser;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
    task::JoinSet,
};

mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let cl_args = cli::CommandLineInterface::parse();

    let mut filenames = cl_args.filename.into_iter();

    let mut file_data = BTreeMap::new();

    let mut buffer = String::with_capacity(2048);

    let first_filename = filenames.next().unwrap();
    // Immidiately read the first file into buffer
    let file = File::open(&first_filename).await?;
    let mut bufreader = BufReader::new(file);
    bufreader.read_to_string(&mut buffer).await?;

    file_data.insert(first_filename, buffer);

    let mut job_set = read_files_in_parallel(filenames).await;

    while let Some(Ok(Ok((fnm, data)))) = job_set.join_next().await {
        file_data.insert(fnm, data);
    }

    Ok(())
}

async fn read_files_in_parallel(
    filenames: IntoIter<PathBuf>,
) -> JoinSet<Result<(PathBuf, String), std::io::Error>> {
    let mut job_set = JoinSet::new();

    for fnm in filenames {
        job_set.spawn(async move {
            let mut buffer = String::with_capacity(2048);
            // Immidiately read the first file into buffer
            let file = File::open(&fnm).await?;
            let mut bufreader = BufReader::new(file);
            bufreader.read_to_string(&mut buffer).await?;

            Ok((fnm, buffer))
        });
    }
    job_set
}
