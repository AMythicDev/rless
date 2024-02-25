use std::{
    collections::{linked_list::IntoIter, BTreeMap},
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

type SyncedfileList = Arc<Mutex<Vec<PathBuf>>>;

#[tokio::main]
async fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let cl_args = cli::CommandLineInterface::parse();

    let mut filenames = Arc::new(Mutex::new(cl_args.filename));
    let bufsize = cl_args.buffers.unwrap_or(64);
    // TODO: Introduce proper error handling
    assert!(
        bufsize >= -1,
        "bufsize cannot take a value less than -1, {bufsize}",
        bufsize = bufsize,
    );

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

    let pager = configure_pager(&&cl_args, filenames)?;

    tokio::task::spawn_blocking(move || start_pager(pager, file_list_clone.clone()));

    Ok(())
}

fn configure_pager(
    cl_args: &cli::CommandLineInterface,
    filenames: SyncedfileList,
) -> Result<Pager, MinusError> {
    let pager = Pager::new();
    let mut input_register = minus::input::HashedEventRegister::default();
    let to_jump = AtomicBool::new(false);
    let fl_clone = filenames.clone();
    let pager_clone = pager.clone();

    let quit_on_eof = cl_args.quit_on_eof;
    let incsearch = cl_args.incsearch;

    input_register.add_key_events(&["space"], move |_, ps| {
        if ps.upper_mark.saturating_add(ps.rows) >= ps.screen.formatted_lines_count() {
            let to_jump_val = to_jump.load(std::sync::atomic::Ordering::SeqCst);

            if !to_jump_val {
                to_jump.store(true, std::sync::atomic::Ordering::SeqCst);
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                let _ = pager_clone.send_message("EOF");
                return InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position));
            }

            to_jump.store(false, std::sync::atomic::Ordering::SeqCst);
            let mut guard = fl_clone.lock();
            if guard.is_empty() && quit_on_eof {
                return InputEvent::Exit;
            } else {
                return InputEvent::Ignore;
            }
            move_to_next_file(filenames, &pager);
            InputEvent::Ignore
        } else {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
            InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
        }
    });

    pager.set_input_classifier(Box::new(input_register))?;
    pager.set_incremental_search_condition(Box::new(move |_| incsearch))?;
    Ok(pager)
}

fn start_pager(pager: Pager, file_list: SyncedfileList) -> Result<(), MinusError> {
    let mut fl_lock = file_list.lock();
    let data = fl_lock.move_next().unwrap();

    let (first_filename, first_file_data) = (data.0.clone(), data.1.clone());
    drop(fl_lock);

    pager.set_text(first_file_data)?;
    pager.set_prompt(first_filename.to_string_lossy())?;

    minus::dynamic_paging(pager)
}

async fn move_to_next_file(
    file_list: SyncedfileList,
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
