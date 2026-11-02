use command_manager::{model::Library, store::Store};
use std::{path::PathBuf, sync::mpsc};

pub struct Update {
    pub library: Option<Library>,
    pub import: Option<Library>,
    pub message: String,
    pub close_editor: bool,
}
impl Update {
    pub fn saved(library: Library, message: &str) -> Self {
        Self {
            library: Some(library),
            import: None,
            message: message.into(),
            close_editor: true,
        }
    }
}
pub type Job = Box<dyn FnOnce(&Store) -> anyhow::Result<Update> + Send>;

pub struct Worker {
    pub tx: mpsc::Sender<Job>,
    pub rx: mpsc::Receiver<anyhow::Result<Update>>,
}
impl Worker {
    pub fn new(path: PathBuf) -> Self {
        let (tx, jobs) = mpsc::channel::<Job>();
        let (results, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let store = Store::new(path);
            while let Ok(job) = jobs.recv() {
                if results.send(job(&store)).is_err() {
                    break;
                }
            }
        });
        Self { tx, rx }
    }
}
