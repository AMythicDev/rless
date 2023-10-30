use std::{collections::BTreeMap, convert::TryInto, path::PathBuf, sync::Arc, vec::IntoIter};

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
    let bufsize = cl_args.buffers.unwrap_or(64);
    // TODO: Introduce proper error handling
    assert!(
        bufsize >= -1,
        "bufsize cannot take a value less than -1, {bufsize}",
        bufsize = bufsize,
    );

    let file_list = Arc::new(Mutex::new(FileList::default()));

    // Immidiately read the first file into buffer
    let mut buffer;
    if bufsize == -1 {
        // If buffer size is -1 we sent the capacity of buffer to a good amount
        // like 1GB to show that the nuffer size is infinite while still avoiding
        // much allocations
        buffer = Vec::with_capacity(1024 * 1024 * 1024);
    } else {
        buffer = vec![0u8; <isize as TryInto<usize>>::try_into(bufsize).unwrap() * 1024];
    }

    let first_filename = filenames.next().unwrap();
    let file = File::open(&first_filename).await?;
    let mut bufreader = BufReader::new(file);

    // Read the entire file if bufsize is -1 as the user explicitly asked for unlimited memory space
    // Otherwise read only `bufsize` amount of data from file and keep the rest for reading later
    #[allow(clippy::unused_io_amount)]
    if bufsize == -1 {
        bufreader.read_to_end(&mut buffer).await?;
    } else {
        bufreader.read(&mut buffer).await?;
    }

    let text = String::from_utf8_lossy(&buffer).into_owned();

    file_list.lock().await.push(first_filename, text);

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
            let mut buffer = Vec::with_capacity(64 * 1024);
            // Immidiately read the first file into buffer
            let mut file = File::open(&fnm).await?;
            file.read_to_end(&mut buffer).await?;
            let text = String::from_utf8_lossy(&buffer).into_owned();

            Ok((fnm, text))
        });
    }
    job_set
}

async fn start_pager(file_list: Arc<Mutex<FileList>>) -> Result<(), MinusError> {
    let mut fl_lock = file_list.lock().await;

    let (first_filename, first_file_data) = fl_lock.move_next().unwrap();

    // let input_register = minus::input::HashedEventRegister::default();
    // input_register.add_key_events(&["space"], |_, ps| {
    // if ps.upper_mark.saturating_add(ps.rows)
    // });

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
