use std::collections::HashMap;

use strum::EnumString;

use crate::app::AppActions;

#[derive(Debug, PartialEq, Clone, Copy, EnumString)]
enum CompletionTypes {
    None,
    Path,
}

pub(crate) struct CommandMode {
    commands: HashMap<String, AppActions>,
}

impl CommandMode {
    pub(crate) fn new() -> CommandMode {
        let mut commands = HashMap::new();
        commands.insert(String::from("delete"), AppActions::DeleteFile);
        commands.insert(String::from("up"), AppActions::MoveUp);
        commands.insert(String::from("bookmark"), AppActions::CreateBookmark);
        commands.insert(String::from("del_bookmark"), AppActions::DeleteBookmark);
        commands.insert(String::from("bm"), AppActions::CreateBookmark);
        commands.insert(String::from("dbm"), AppActions::DeleteBookmark);
        commands.insert(String::from("mv"), AppActions::MoveEntry);
        commands.insert(String::from("mkdir"), AppActions::CreateDir);

        CommandMode { commands }
    }
}
