use std::{
    collections::HashMap,
    fs::{self, DirEntry, FileType},
    io,
    path::{Path, PathBuf}, ffi::OsStr,
};

use tui::{backend::Backend, Terminal};

use crate::ui::Ui;

enum AppActions {
    MoveDown,
    MoveUp,
    MoveUpDir,
    EnterDir,
    Quit,
    MoveToTop,
    MoveToBottom,
    CopyFiles,
    CutFiles,
    PasteFiles,
}

enum YankMode {
    Copying,
    Cutting,
}

pub struct Bookmark<'a> {
    pub name: String,
    pub path: &'a Path,
}

pub struct App<'a> {
    pub title: String,

    pub should_quit: bool,
    pub current_dir: Box<PathBuf>,

    pub dir_contents: Vec<DirEntry>,

    pub bookmarks: Vec<Bookmark<'a>>,

    ui: Ui,

    // Vim Controls
    last_char: char,
    key_chord: Vec<char>,
    bindings: HashMap<Vec<char>, AppActions>,
    // ---

    yank_reg: Box<PathBuf>,
    yank_mode: Option<YankMode>,
}

impl App<'_> {
    pub fn new(title: String, current_dir: &Path) -> App {
        let mut bindings = HashMap::new();
        bindings.insert(str_to_char_arr("j"),  AppActions::MoveDown);
        bindings.insert(str_to_char_arr("k"),  AppActions::MoveUp);
        bindings.insert(str_to_char_arr("h"),  AppActions::MoveUpDir);
        bindings.insert(str_to_char_arr("l"),  AppActions::EnterDir);
        bindings.insert(str_to_char_arr("q"),  AppActions::Quit);
        bindings.insert(str_to_char_arr("gg"), AppActions::MoveToTop);
        bindings.insert(str_to_char_arr("G"),  AppActions::MoveToBottom);
        bindings.insert(str_to_char_arr("yy"), AppActions::CopyFiles);
        bindings.insert(str_to_char_arr("dd"), AppActions::CutFiles);
        bindings.insert(str_to_char_arr("p"),  AppActions::PasteFiles);

        App {
            title,
            should_quit: false,
            current_dir: Box::<PathBuf>::new(current_dir.to_path_buf().clone()),
            dir_contents: fs::read_dir(current_dir)
                .unwrap()
                .map(|x| x.unwrap())
                .collect(),
            bookmarks: vec![
                Bookmark {
                    name: String::from("Nextcloud"),
                    path: Path::new("/home/vincent/Nextcloud"),
                },
                Bookmark {
                    name: String::from("Obsidian"),
                    path: Path::new("/home/vincent/Nextcloud/chalmers/Obsidian"),
                },
            ],
            ui: Ui::new(current_dir.to_str().unwrap()),
            last_char: ' ',
            key_chord: Vec::new(),
            bindings,
            yank_reg: Box::<PathBuf>::new("/tmp/rust_fm_yank.txt".into()),
            yank_mode: None,
        }
    }

    pub fn on_key(&mut self, c: char) {
        self.last_char = c;

        self.key_chord.push(c);
        let mut matched = true;

        let selected_paths = self.get_selected_entries().iter().map(|d| d.path()).collect();

        match self.bindings.get(&self.key_chord) {
            Some(action) => match action {
                AppActions::MoveDown => self.ui.scroll(1, self.dir_contents.len() as i32),
                AppActions::MoveUp => self.ui.scroll(-1, self.dir_contents.len() as i32),
                AppActions::MoveUpDir => {
                    self.move_up_dir();
                    self.ui
                        .scroll_abs(self.find_name(self.ui.last_name.clone()).unwrap_or(0), self.dir_contents.len() as i32);
                    self.ui.last_name = self
                        .current_dir
                        .file_name()
                        .unwrap_or(OsStr::new(""))
                        .to_str()
                        .unwrap()
                        .to_string();
                }
                AppActions::EnterDir => {
                    if self.dir_contents[(self.ui.cursor_y + self.ui.scroll_y) as usize]
                        .file_type()
                        .unwrap()
                        .is_dir()
                    {
                        let path =
                            &self.dir_contents[(self.ui.cursor_y + self.ui.scroll_y) as usize];
                        self.ui.last_name =
                            path.file_name().to_owned().to_str().unwrap().to_string();
                        self.enter_dir(&path.path());
                        self.ui.scroll_abs(0, self.dir_contents.len() as i32);
                    }
                }
                AppActions::Quit => {
                    self.should_quit = true;
                },
                AppActions::MoveToTop => self.ui.scroll_abs(0, self.dir_contents.len() as i32),
                AppActions::MoveToBottom => self.ui.scroll_abs(self.dir_contents.len() as i32 - 1, self.dir_contents.len() as i32),
                AppActions::CopyFiles => self.copy_files(selected_paths),
                AppActions::CutFiles => self.cut_files(selected_paths),
                AppActions::PasteFiles => self.paste_yanked_files(),
            },
            None => matched = false,
        }

        if matched {
            self.key_chord.clear();
        } else {
            let mut starting = false;
            let chord_len = self.key_chord.len();

            for chord in self.bindings.keys() {
                if chord.len() >= chord_len {
                    if chord[0..chord_len] == self.key_chord[..] {
                        starting = true;
                    }
                }
            }

            if !starting {
                self.key_chord.clear();
            }
        }
    }

    pub(crate) fn on_tick(&self) {
        return;
    }

    pub(crate) fn enter_dir(&mut self, dir: &Path) {
        self.current_dir = Box::new(dir.to_path_buf());
        self.dir_contents = fs::read_dir(dir).unwrap().map(|x| x.unwrap()).collect();
    }

    pub(crate) fn move_up_dir(&mut self) {
        let parent = self.current_dir.parent().unwrap().to_path_buf();
        self.dir_contents = fs::read_dir(&parent).unwrap().map(|x| x.unwrap()).collect();
        self.current_dir = Box::new(parent);
    }

    pub(crate) fn draw<B: Backend>(&mut self, term: &mut Terminal<B>) -> io::Result<()> {
        self.ui.draw_app(
            term,
            self.current_dir.to_str().unwrap(),
            &self.bookmarks,
            &self.dir_contents,
        )
    }

    fn find_name(&self, name: String) -> Option<i32> {
        for (j, d) in self.dir_contents.iter().enumerate() {
            if d.file_name().into_string().unwrap() == name {
                return Some(i32::try_from(j).unwrap());
            }
        }

        return None;
    }

    fn copy_files(&mut self, paths: Vec<PathBuf>) {
        let mut output = String::new();
        for p in paths {
            output.push_str(p.as_path().to_str().unwrap());
            output.push('\n');
        }
        fs::write(self.yank_reg.as_path(), output).unwrap();

        self.yank_mode = Some(YankMode::Copying);
    }

    fn cut_files(&mut self, paths: Vec<PathBuf>) {
        let mut output = String::new();
        for p in paths {
            output.push_str(p.as_path().to_str().unwrap());
            output.push('\n');
        }
        fs::write(self.yank_reg.as_path(), output).unwrap();

        self.yank_mode = Some(YankMode::Cutting);
    }

    fn get_selected_entries(&self) -> Vec<&DirEntry> {
        vec![&self.dir_contents[(self.ui.cursor_y + self.ui.scroll_y) as usize]]
    }

    fn paste_yanked_files(&mut self) {
        let contents = fs::read_to_string(self.yank_reg.as_path()).unwrap();
        let lines = contents.split("\n");

        let dest_dir = self.current_dir.clone();

        for l in lines {
            if l.len() > 0 {
                let p = Path::new(l);
                let dest = dest_dir.join(p.file_name().unwrap());

                let copy_success = fs::copy(&p, dest);

                if let Ok(_) = copy_success {
                    if let Some(_) = &self.yank_mode {
                        fs::remove_file(&p).unwrap();
                    }
                }

            }
        }

        self.update_dir_contents();
    }

    fn update_dir_contents(&mut self) {
        self.dir_contents = fs::read_dir(self.current_dir.as_path()).unwrap().map(|x| x.unwrap()).collect();
    }
}

fn str_to_char_arr(s: &str) -> Vec<char> {
    let mut output = Vec::with_capacity(s.len());
    for c in s.chars() {
        output.push(c);
    }
    output
}
