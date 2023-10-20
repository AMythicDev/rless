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

#[derive(Default)]
struct FileList {
    filenames: Vec<PathBuf>,
    file_data: Vec<String>,
    active: usize,
}

impl FileList {
    fn push(&mut self, fnm: PathBuf, data: String) {
        self.filenames.push(fnm);
        self.file_data.push(data);
    }

    fn precheck(&self) {
        assert_eq!(
            self.filenames.len(),
            self.file_data.len(),
            "Length of filenames vector not equal to file_data vectors. This is most likely a bug.\
            Please report to the project maintainers"
        )
    }

    fn move_next(&mut self) -> Option<(&PathBuf, &String)> {
        self.precheck();
        if self.active >= self.filenames.len() {
            return None;
        }
        let filename = self.filenames.get(self.active).unwrap();
        let file_data = self.file_data.get(self.active).unwrap();
        self.active = self.active.saturating_add(1);

        Some((filename, file_data))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let cl_args = cli::CommandLineInterface::parse();

    let mut filenames = cl_args.filename.into_iter();

    let file_list = Arc::new(Mutex::new(FileList::default()));

    let mut buffer = String::with_capacity(2048);

    let first_filename = filenames.next().unwrap();
    // Immidiately read the first file into buffer
    let file = File::open(&first_filename).await?;
    let mut bufreader = BufReader::new(file);
    bufreader.read_to_string(&mut buffer).await?;

    file_list.lock().await.push(first_filename, buffer);

    tokio::join!(start_pager(file_list.clone()), async {
        let mut job_set = read_files_in_parallel(filenames).await;

        while let Some(Ok(Ok((fnm, data)))) = job_set.join_next().await {
            let mut fd_lock = file_list.lock().await;
            fd_lock.push(fnm, data);
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

async fn start_pager(file_list: Arc<Mutex<FileList>>) -> Result<(), MinusError> {
    let mut fl_lock = file_list.lock().await;

    let (first_filename, first_file_data) = fl_lock.move_next().unwrap();

    let input_register = minus::input::HashedEventRegister::default();
    input_register.add_key_events(&["space"], |_, ps| {
        // if ps.upper_mark.saturating_add(ps.rows)
    });

    let pager = Pager::new();
    pager.set_text(first_file_data)?;
    pager.set_prompt(first_filename.to_string_lossy())?;
    drop(fl_lock);

    minus::dynamic_paging(pager)
}

async fn move_to_next_file(
    file_list: Arc<Mutex<FileList>>,
    pager: &minus::Pager,
) -> Result<(), MinusError> {
    let mut fl_lock = file_list.lock().await;

    let (filename, file_data) = fl_lock.move_next().unwrap();

    let pager = Pager::new();
    pager.set_text(file_data)?;
    pager.set_prompt(filename.to_string_lossy())?;
    drop(fl_lock);
    Ok(())
}
