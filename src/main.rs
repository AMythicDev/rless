use std::{
    collections::BTreeMap,
    convert::TryInto,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    vec::IntoIter,
};

use clap::Parser;
use minus::{input::InputEvent, MinusError, Pager};
use parking_lot::Mutex;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
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

    fn end(&self) -> bool {
        self.precheck();
        self.active == self.filenames.len()
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

    file_list.lock().push(first_filename, text);
    let file_list_clone = file_list.clone();

    let pager_run = tokio::task::spawn_blocking(move || start_pager(file_list_clone.clone()));
    tokio::spawn(async move {
        let mut job_set = read_files_in_parallel(filenames).await;

        while let Some(Ok(Ok((fnm, data)))) = job_set.join_next().await {
            let mut fd_lock = file_list.lock();
            fd_lock.push(fnm, data);
        }
    })
    .await?;

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

fn start_pager(file_list: Arc<Mutex<FileList>>) -> Result<(), MinusError> {
    let mut fl_lock = file_list.lock();
    let data = fl_lock.move_next().unwrap();

    let (first_filename, first_file_data) = (data.0.clone(), data.1.clone());
    drop(fl_lock);

    let pager = Pager::new();
    let mut input_register = minus::input::HashedEventRegister::default();
    let to_jump = AtomicBool::new(false);

    let fl_clone = file_list.clone();
    let pager_clone = pager.clone();
    input_register.add_key_events(&["space"], move |_, ps| {
        if ps.upper_mark.saturating_add(ps.rows) >= ps.screen.formatted_lines_count() {
            let to_jump_val = to_jump.load(std::sync::atomic::Ordering::SeqCst);

            if to_jump_val {
                to_jump.store(false, std::sync::atomic::Ordering::SeqCst);
                let mut guard = fl_clone.lock();
                if guard.end() {
                    return InputEvent::Exit;
                }
                let (filename, file_contents) = guard.move_next().unwrap();
                let _ = pager_clone.set_text(file_contents);
                let _ = pager_clone.set_prompt(filename.to_string_lossy());

                InputEvent::Ignore
            } else {
                to_jump.store(true, std::sync::atomic::Ordering::SeqCst);
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                let _ = pager_clone.send_message("EOF");
                InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
            }
        } else {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
            InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
        }
    });

    pager.set_text(first_file_data)?;
    pager.set_prompt(first_filename.to_string_lossy())?;
    pager.set_input_classifier(Box::new(input_register))?;

    minus::dynamic_paging(pager)
}

// async fn move_to_next_file(
//     file_list: Arc<Mutex<FileList>>,
//     pager: &minus::Pager,
// ) -> Result<(), MinusError> {
//     let mut fl_lock = file_list.lock().await;
//
//     let (filename, file_data) = fl_lock.move_next().unwrap();
//
//     let pager = Pager::new();
//     pager.set_text(file_data)?;
//     pager.set_prompt(filename.to_string_lossy())?;
//     drop(fl_lock);
//     Ok(())
// }
