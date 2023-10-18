use std::{collections::BTreeMap, path::PathBuf, sync::Arc, vec::IntoIter};

use clap::Parser;
use minus::{MinusError, Pager};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
    sync::Mutex,
    task::JoinSet,
};

mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let cl_args = cli::CommandLineInterface::parse();

    let mut filenames = cl_args.filename.into_iter();

    let file_data = Arc::new(Mutex::new(BTreeMap::new()));

    let mut buffer = String::with_capacity(2048);

    let first_filename = filenames.next().unwrap();
    // Immidiately read the first file into buffer
    let file = File::open(&first_filename).await?;
    let mut bufreader = BufReader::new(file);
    bufreader.read_to_string(&mut buffer).await?;

    file_data.lock().await.insert(first_filename, buffer);

    tokio::join!(start_pager(file_data.clone()), async {
        let mut job_set = read_files_in_parallel(filenames).await;

        while let Some(Ok(Ok((fnm, data)))) = job_set.join_next().await {
            let mut fd_lock = file_data.lock().await;
            fd_lock.insert(fnm, data);
        }
    })
    .0?;

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

async fn start_pager(file_data: Arc<Mutex<BTreeMap<PathBuf, String>>>) -> Result<(), MinusError> {
    let fd_lock = file_data.lock().await;

    let first_file_data = fd_lock.first_key_value().unwrap().1;
    let first_file_prompt = fd_lock.first_key_value().unwrap().0;

    let pager = Pager::new();
    pager.set_text(first_file_data)?;
    pager.set_prompt(first_file_prompt.to_string_lossy())?;

    minus::dynamic_paging(pager)
}
