use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, DirEntry, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
};

use configparser::ini::Ini;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fs_extra::dir::CopyOptions;
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tui::{backend::Backend, Terminal};

use crate::ui::Ui;

#[derive(Debug, Clone, Copy, EnumString, PartialEq, Eq)]
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
    OpenCommandMode,
    DeleteFile,
    CreateBookmark,
    DeleteBookmark,
    ToggleBookmark,
    MoveToLeftPanel,
    MoveToRightPanel,
    MoveEntry,
    ToggleHiddenFiles,
    CreateDir,
    ToggleVisualMode,
}

#[derive(PartialEq, Clone, Copy)]
enum YankMode {
    Copying,
    Cutting,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Bookmark {
    pub name: String,
    pub path: Box<PathBuf>,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ActivePanel {
    Main,
    Bookmarks,
}

#[derive(PartialEq, Clone, Copy)]
enum ActiveMode {
    Normal,
    Command,
    Visual,
}

pub struct App {
    pub title: String,

    pub should_quit: bool,
    pub current_dir: Box<PathBuf>,

    pub dir_contents: Vec<DirEntry>,

    pub bookmarks: Vec<Bookmark>,

    ui: Ui,

    // Vim Controls
    last_key: KeyEvent,
    key_chord: Vec<KeyEvent>,
    bindings: HashMap<Vec<KeyEvent>, AppActions>,
    visual_bindings: HashMap<Vec<KeyEvent>, AppActions>,
    commands: HashMap<String, AppActions>,
    active_panel: ActivePanel,
    active_mode: ActiveMode,
    // ---
    yank_reg: Box<PathBuf>,
    yank_mode: Option<YankMode>,

    bookmark_store: Box<PathBuf>,

    command_buffer: String,
    command_buffer_tmp: String,
    command_history: Vec<String>,
    command_index: i32,

    show_hidden_files: bool,
}

impl App {
    pub fn new(title: String, current_dir: &Path) -> App {
        let config_path = home::home_dir().unwrap().join(".config/trooper/config.ini");
        let (bindings, visual_bindings) = read_config(&config_path).unwrap();

        let mut commands = HashMap::new();
        commands.insert(String::from("delete"), AppActions::DeleteFile);
        commands.insert(String::from("up"), AppActions::MoveUp);
        commands.insert(String::from("bookmark"), AppActions::CreateBookmark);
        commands.insert(String::from("del_bookmark"), AppActions::DeleteBookmark);
        commands.insert(String::from("bm"), AppActions::CreateBookmark);
        commands.insert(String::from("dbm"), AppActions::DeleteBookmark);
        commands.insert(String::from("mv"), AppActions::MoveEntry);
        commands.insert(String::from("mkdir"), AppActions::CreateDir);

        App {
            title,
            should_quit: false,
            current_dir: Box::<PathBuf>::new(current_dir.to_path_buf().clone()),
            dir_contents: Vec::new(),
            bookmarks: vec![],
            ui: Ui::new(current_dir.to_str().unwrap()),
            last_key: KeyEvent::new(KeyCode::Null, KeyModifiers::empty()),
            key_chord: Vec::new(),
            bindings,
            visual_bindings,
            commands,
            active_panel: ActivePanel::Main,
            active_mode: ActiveMode::Normal,
            yank_reg: Box::<PathBuf>::new("/tmp/rust_fm_yank.txt".into()),
            yank_mode: None,
            bookmark_store: Box::<PathBuf>::new(
                dirs::home_dir()
                    .unwrap_or(Path::new("/tmp/").to_path_buf())
                    .join(".trooper/bookmarks.txt"),
            ),
            command_buffer: String::from(""),
            command_buffer_tmp: String::from(""),
            command_history: Vec::new(),
            command_index: -1,
            show_hidden_files: false,
        }
    }

    pub fn init(&mut self) {
        self.enter_dir(&self.current_dir.to_owned());
        fs::create_dir_all(self.bookmark_store.parent().unwrap()).unwrap();

        if !Path::new(self.bookmark_store.as_path()).exists() {
            fs::write(self.bookmark_store.as_path(), "[]").unwrap();
        }

        let f = File::open(self.bookmark_store.as_path()).unwrap();
        let bookmark_file = BufReader::new(f);
        self.bookmarks = serde_json::from_reader(bookmark_file).unwrap_or(vec![]);

        self.update_bookmark_width();
    }

    pub fn tear_down(&mut self) {
        fs::write(
            self.bookmark_store.as_path(),
            serde_json::to_string(&self.bookmarks).unwrap(),
        )
        .unwrap();
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        self.last_key = key;
        /*
        if mods.intersects(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }
        */

        self.key_chord.push(key);
        let mut matched = true;

        match self.active_mode {
            ActiveMode::Normal => {
                // Figure out some way to do this shit with borrowing
                let maybe_action = self.get_binding();
                match maybe_action {
                    Some(action) => {
                        self.normal_handle_action(action, vec![]);
                    }
                    None => matched = false,
                }
            }
            ActiveMode::Visual => {
                let maybe_action = self.get_binding();
                match maybe_action {
                    Some(action) => {
                        self.visual_handle_action(action, vec![]);
                    }
                    None => matched = false,
                }
            }
            ActiveMode::Command => match key.code {
                KeyCode::Char(c) => self.command_buffer.push(c),
                _ => {}
            },
        }

        // TODO: How does this work when in visual mode
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

    fn get_binding(&mut self) -> Option<AppActions> {
        match self.active_mode {
            ActiveMode::Normal => match self.bindings.get(&self.key_chord) {
                Some(a) => Some(a.clone()),
                None => None,
            },
            ActiveMode::Visual => match self.visual_bindings.get(&self.key_chord) {
                Some(a) => Some(a.clone()),
                None => None,
            },
            ActiveMode::Command => None
        }
    }

    pub(crate) fn on_tick(&self) {
        return;
    }

    pub(crate) fn enter_dir(&mut self, dir: &Path) {
        self.current_dir = Box::new(dir.to_path_buf());
        self.dir_contents = self.read_dir_sorted(dir);
    }

    pub(crate) fn move_up_dir(&mut self) {
        let parent = self.current_dir.parent().unwrap().to_path_buf();
        self.dir_contents = self.read_dir_sorted(&parent);
        self.current_dir = Box::new(parent);
    }

    pub(crate) fn draw<B: Backend>(&mut self, term: &mut Terminal<B>) -> io::Result<()> {
        self.ui.draw_app(
            term,
            self.current_dir.to_str().unwrap(),
            &self.bookmarks,
            &self.dir_contents,
            self.active_mode == ActiveMode::Command,
            &self.command_buffer,
            &self.active_panel,
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

    fn delete_files(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            let md = fs::metadata(&p).unwrap();
            if md.is_dir() {
                fs::remove_dir_all(&p).unwrap();
            } else if md.is_file() {
                fs::remove_file(&p).unwrap();
            }
        }

        self.update_dir_contents();
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
        if !&self.dir_contents.is_empty() {
            vec![&self.dir_contents[(self.ui.cursor_y + self.ui.scroll_y) as usize]]
        } else {
            Vec::new()
        }
    }

    fn get_selected_bookmark(&self) -> Option<&Bookmark> {
        self.bookmarks
            .get((self.ui.bookmark_y + self.ui.bookmark_scroll_y) as usize)
    }

    fn paste_yanked_files(&mut self) {
        let contents = fs::read_to_string(self.yank_reg.as_path()).unwrap();
        let lines = contents.split("\n");

        let dest_dir = self.current_dir.clone();

        for l in lines {
            if l.len() > 0 {
                let p = Path::new(l);
                let mut dest = dest_dir.join(p.file_name().unwrap());
                let md = fs::metadata(&p).unwrap();

                while dest.exists() {
                    dest.set_file_name(format!(
                        "{} (Copy).{}",
                        dest.file_stem().unwrap().to_str().unwrap(),
                        dest.extension().unwrap().to_str().unwrap()
                    ));
                }

                if md.is_dir() {
                    let copy_options = CopyOptions::new();
                    let copy_success = fs_extra::dir::copy(&p, dest, &copy_options);

                    if let Ok(_) = copy_success {
                        if let Some(ym) = self.yank_mode {
                            if ym == YankMode::Cutting {
                                fs::remove_dir_all(&p).unwrap();
                            }
                        }
                    }
                } else if md.is_file() {
                    let copy_success = fs::copy(&p, dest);

                    if let Ok(_) = copy_success {
                        if let Some(ym) = self.yank_mode {
                            if ym == YankMode::Cutting {
                                fs::remove_file(&p).unwrap();
                            }
                        }
                    }
                }
            }
        }

        self.update_dir_contents();
    }

    fn update_dir_contents(&mut self) {
        self.dir_contents = self.read_dir_sorted(self.current_dir.as_path());

        self.ui.scroll_abs(
            self.ui.cursor_y + self.ui.scroll_y,
            self.dir_contents.len() as i32,
            &self.active_panel,
        );
    }

    fn normal_handle_action(&mut self, action: AppActions, args: Vec<String>) {
        let selected_paths = self
            .get_selected_entries()
            .iter()
            .map(|d| d.path())
            .collect();
        match self.active_panel {
            ActivePanel::Main => match action {
                AppActions::MoveDown => {
                    self.ui
                        .scroll(1, self.dir_contents.len() as i32, &self.active_panel)
                }
                AppActions::MoveUp => {
                    self.ui
                        .scroll(-1, self.dir_contents.len() as i32, &self.active_panel)
                }
                AppActions::MoveUpDir => {
                    self.move_up_dir();
                    self.ui.scroll_abs(
                        self.find_name(self.ui.last_name.clone()).unwrap_or(0),
                        self.dir_contents.len() as i32,
                        &self.active_panel,
                    );
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
                        self.ui
                            .scroll_abs(0, self.dir_contents.len() as i32, &self.active_panel);
                    }
                }
                AppActions::Quit => {
                    self.should_quit = true;
                }
                AppActions::MoveToTop => {
                    self.ui
                        .scroll_abs(0, self.dir_contents.len() as i32, &self.active_panel)
                }
                AppActions::MoveToBottom => self.ui.scroll_abs(
                    self.dir_contents.len() as i32 - 1,
                    self.dir_contents.len() as i32,
                    &self.active_panel,
                ),
                AppActions::CopyFiles => self.copy_files(selected_paths),
                AppActions::CutFiles => self.cut_files(selected_paths),
                AppActions::PasteFiles => self.paste_yanked_files(),
                AppActions::OpenCommandMode => {
                    self.command_buffer = String::from("");
                    self.active_mode = ActiveMode::Command;
                }
                AppActions::DeleteFile => self.delete_files(selected_paths),
                AppActions::CreateBookmark => self.create_bookmark(),
                AppActions::DeleteBookmark => {}
                AppActions::ToggleBookmark => {
                    self.active_panel = ActivePanel::Bookmarks;
                }
                AppActions::MoveToLeftPanel => {
                    self.active_panel = ActivePanel::Bookmarks;
                }
                AppActions::MoveEntry => {
                    if args.len() > 0 && selected_paths.len() == 1 {
                        self.mv_entry(&selected_paths[0], &args[0]);
                    }
                }
                AppActions::ToggleHiddenFiles => {
                    self.show_hidden_files = !self.show_hidden_files;
                    self.update_dir_contents();
                }
                AppActions::ToggleVisualMode => {
                    self.active_mode = ActiveMode::Visual;
                }
                _ => {}
            },
            ActivePanel::Bookmarks => match action {
                AppActions::MoveDown => {
                    self.ui
                        .scroll(1, self.bookmarks.len() as i32, &self.active_panel)
                }
                AppActions::MoveUp => {
                    self.ui
                        .scroll(-1, self.bookmarks.len() as i32, &self.active_panel)
                }
                AppActions::EnterDir => {
                    if let Some(b) = self.get_selected_bookmark() {
                        let path = b.path.clone();
                        self.enter_dir(&path);
                    }
                    self.active_panel = ActivePanel::Main;
                    self.ui
                        .scroll_abs(0, self.dir_contents.len() as i32, &self.active_panel);
                }
                AppActions::Quit => self.should_quit = true,
                AppActions::DeleteBookmark => self.delete_bookmark(),
                AppActions::ToggleBookmark => match self.active_panel {
                    ActivePanel::Main => self.active_panel = ActivePanel::Bookmarks,
                    ActivePanel::Bookmarks => self.active_panel = ActivePanel::Main,
                },
                AppActions::OpenCommandMode => {
                    self.command_buffer = String::from("");
                    self.active_mode = ActiveMode::Command;
                }
                AppActions::MoveToRightPanel => {
                    self.active_panel = ActivePanel::Main;
                }
                _ => {}
            },
        }

        match action {
            AppActions::CreateDir => {
                for arg in &args {
                    self.create_dir(arg);
                }
                self.update_dir_contents();
            }
            _ => {}
        }
    }

    fn visual_handle_action(&mut self, action: AppActions, args: Vec<String>) {
        let selected_paths = self
            .get_selected_entries()
            .iter()
            .map(|d| d.path())
            .collect();
        /* There's no need to check the active panel since we can only enter
         * visual mode in the main panel
         */
        match action {
            AppActions::MoveDown => {
                self.ui
                    .scroll(1, self.dir_contents.len() as i32, &self.active_panel)
            }
            AppActions::MoveUp => {
                self.ui
                    .scroll(-1, self.dir_contents.len() as i32, &self.active_panel)
            }
            AppActions::Quit => {
                self.should_quit = true;
            }
            AppActions::MoveToTop => {
                self.ui
                    .scroll_abs(0, self.dir_contents.len() as i32, &self.active_panel)
            }
            AppActions::MoveToBottom => self.ui.scroll_abs(
                self.dir_contents.len() as i32 - 1,
                self.dir_contents.len() as i32,
                &self.active_panel,
            ),
            AppActions::CopyFiles => self.copy_files(selected_paths),
            AppActions::CutFiles => self.cut_files(selected_paths),
            AppActions::PasteFiles => self.paste_yanked_files(),
            AppActions::OpenCommandMode => {
                self.command_buffer = String::from("");
                self.active_mode = ActiveMode::Command;
            }
            AppActions::DeleteFile => self.delete_files(selected_paths),
            AppActions::ToggleVisualMode => {
                self.active_mode = ActiveMode::Normal;
            }
            _ => {}
        }
    }

    pub(crate) fn on_esc(&mut self) {
        match self.active_mode {
            ActiveMode::Command => {
                self.active_mode = ActiveMode::Normal;
                self.command_buffer.clear();
            }
            _ => {}
        }
    }

    pub(crate) fn on_enter(&mut self) {
        match self.active_mode {
            ActiveMode::Command => {
                let words: Vec<&str> = self.command_buffer.split(" ").collect();

                if let Some(cmd) = words.get(0) {
                    match self.commands.get(*cmd) {
                        Some(action) => {
                            let args = words[1..].into_iter().map(|x| String::from(*x)).collect();
                            self.normal_handle_action(*action, args);
                        }
                        None => (),
                    }

                    self.command_history.push(self.command_buffer.clone());
                    self.on_esc();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn on_backspace(&mut self) {
        match self.active_mode {
            ActiveMode::Command => {
                if self.command_buffer.len() > 0 {
                    self.command_buffer.pop();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn on_down(&mut self) {
        match self.active_mode {
            ActiveMode::Command => {
                if self.command_index > 0 {
                    self.command_index = self.command_index - 1;
                    self.command_buffer = self.command_history
                        [(self.command_history.len() as i32 - self.command_index - 1) as usize]
                        .clone();
                } else if self.command_index == 0 {
                    self.command_index = -1;
                    self.command_buffer = self.command_buffer_tmp.clone();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn on_up(&mut self) {
        match self.active_mode {
            ActiveMode::Command => {
                if self.command_index + 1 < self.command_history.len() as i32 {
                    if self.command_index == -1 {
                        self.command_buffer_tmp = self.command_buffer.clone();
                    }
                    self.command_index = self.command_index + 1;

                    self.command_buffer = self.command_history
                        [(self.command_history.len() as i32 - self.command_index - 1) as usize]
                        .clone();
                }
            }
            _ => {}
        }
    }

    fn create_bookmark(&mut self) {
        self.bookmarks.push(Bookmark {
            name: String::from(
                self.current_dir
                    .file_name()
                    .unwrap_or(&OsStr::new("No file name"))
                    .to_str()
                    .unwrap_or("No file name"),
            ),
            path: self.current_dir.to_owned(),
        });

        self.update_bookmark_width();
    }

    fn delete_bookmark(&mut self) {
        let i = (self.ui.bookmark_scroll_y + self.ui.bookmark_y) as usize;
        if i < self.bookmarks.len() {
            self.bookmarks.remove(i);
        }

        self.update_bookmark_width();
    }

    fn update_bookmark_width(&mut self) {
        let mut max_len: u16 = 15;
        for b in &self.bookmarks {
            if b.name.len() > max_len.into() {
                max_len = b.name.len() as u16;
            }
        }
        self.ui.bookmark_width = max_len + 1;
    }

    fn mv_entry(&mut self, src: &Path, dest: &str) {
        let new_name = src.parent().unwrap().join(dest);
        fs::rename(src, new_name).unwrap();
        self.update_dir_contents();
    }

    fn read_dir_sorted<P: AsRef<Path>>(&self, path: P) -> Vec<DirEntry> {
        let mut contents: Vec<DirEntry> = fs::read_dir(path).unwrap().map(|x| x.unwrap()).collect();
        contents.sort_unstable_by_key(|item| {
            (
                item.metadata().unwrap().is_file(),
                item.path().as_path().to_str().unwrap().to_lowercase(),
            )
        });
        contents = contents
            .into_iter()
            .filter(|item| {
                if item
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".")
                {
                    self.show_hidden_files
                } else {
                    true
                }
            })
            .collect();

        return contents;
    }

    fn create_dir(&self, name: &str) {
        match PathBuf::from_str(name) {
            Ok(_) => {
                let new_path = self.current_dir.join(name);
                fs::create_dir_all(new_path);
            }
            Err(_) => {}
        }
    }
}

fn str_to_char_arr(s: &str) -> Vec<char> {
    let mut output = Vec::with_capacity(s.len());
    for c in s.chars() {
        output.push(c);
    }
    return output;
}

fn str_to_key_events(s: &str) -> Vec<KeyEvent> {
    let mut output = Vec::with_capacity(s.len());

    let re = Regex::new(r"<[.|[^<>]]+>|.").unwrap();

    for cap in re.captures_iter(s) {
        let symbol = &cap[0];

        if symbol.len() == 1 {
            output.push(KeyEvent::new(
                KeyCode::Char(symbol.chars().next().unwrap()),
                KeyModifiers::empty(),
            ));
        } else if symbol == "<lt>" {
            output.push(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::empty()));
        } else if symbol == "<gt>" {
            output.push(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::empty()));
        } else if symbol == "<Space>" {
            output.push(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
        } else if symbol.len() == 5 {
            if symbol.chars().nth(1).unwrap() == 'C' || symbol.chars().nth(1).unwrap() == 'c' {
                output.push(KeyEvent::new(
                    KeyCode::Char(symbol.chars().nth(3).unwrap()),
                    KeyModifiers::CONTROL,
                ));
            }
        }
    }

    return output;
}

fn read_config(
    p: &Path,
) -> Result<
    (
        HashMap<Vec<KeyEvent>, AppActions>,
        HashMap<Vec<KeyEvent>, AppActions>,
    ),
    io::Error,
> {
    let mut normal_bindings = HashMap::new();
    let mut visual_bindings = HashMap::new();

    let mut config = Ini::new();
    let mut default = config.defaults();
    default.delimiters = vec!['='];
    default.case_sensitive = true;
    config.load_defaults(default);

    let user_map = if p.exists() {
        match config.read(fs::read_to_string(p)?) {
            Err(msg) => return Err(io::Error::new(io::ErrorKind::Other, msg)),
            Ok(inner) => inner,
        }
    } else {
        HashMap::new()
    };

    let default_map = match config.read(String::from(include_str!("../assets/default_config.ini")))
    {
        Err(msg) => return Err(io::Error::new(io::ErrorKind::Other, msg)),
        Ok(inner) => inner,
    };

    for (k, v) in default_map["normal"]
        .iter()
        .chain(user_map.get("normal").unwrap_or(&HashMap::new()).iter())
    {
        if let Some(v_str) = v {
            if let Ok(action) = AppActions::from_str(v_str) {
                normal_bindings.insert(str_to_key_events(&k), action);
            }
        }
    }

    for (k, v) in default_map["visual"]
        .iter()
        .chain(user_map.get("visual").unwrap_or(&HashMap::new()).iter())
    {
        if let Some(v_str) = v {
            if let Ok(action) = AppActions::from_str(v_str) {
                visual_bindings.insert(str_to_key_events(&k), action);
            }
        }
    }

    return Ok((normal_bindings, visual_bindings));
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, str::FromStr};

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{read_config, str_to_key_events, AppActions};

    #[test]
    fn reading_default_config_gives_default_bindings() {
        let mut bindings = HashMap::new();
        bindings.insert(str_to_key_events("j"), AppActions::MoveDown);
        bindings.insert(str_to_key_events("k"), AppActions::MoveUp);
        bindings.insert(str_to_key_events("h"), AppActions::MoveUpDir);
        bindings.insert(str_to_key_events("l"), AppActions::EnterDir);
        bindings.insert(str_to_key_events("q"), AppActions::Quit);
        bindings.insert(str_to_key_events("gg"), AppActions::MoveToTop);
        bindings.insert(str_to_key_events("G"), AppActions::MoveToBottom);
        bindings.insert(str_to_key_events("yy"), AppActions::CopyFiles);
        bindings.insert(str_to_key_events("dd"), AppActions::CutFiles);
        bindings.insert(str_to_key_events("p"), AppActions::PasteFiles);
        bindings.insert(str_to_key_events(":"), AppActions::OpenCommandMode);
        bindings.insert(str_to_key_events("b"), AppActions::ToggleBookmark);
        bindings.insert(
            vec![
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
            ],
            AppActions::MoveToLeftPanel,
        );
        bindings.insert(
            vec![
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            ],
            AppActions::MoveToRightPanel,
        );
        bindings.insert(
            vec![KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL)],
            AppActions::MoveToLeftPanel,
        );
        bindings.insert(
            vec![KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL)],
            AppActions::MoveToRightPanel,
        );
        bindings.insert(str_to_key_events("z"), AppActions::ToggleHiddenFiles);

        let config_path = PathBuf::from_str("./assets/default_config.ini").unwrap();
        let generated_bindings = match read_config(&config_path) {
            Ok(x) => x,
            Err(msg) => panic!("{}", msg),
        };

        for (k, v) in generated_bindings.iter() {
            assert!(bindings.contains_key(k), "{:?}", k);

            assert!(bindings.get(k).unwrap() == v);
        }
    }
}
