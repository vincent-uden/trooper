use std::{fs::DirEntry, io};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};

use crate::app::Bookmark;

pub struct Ui {
    pub cursor_y: i32,
    pub scroll_y: i32,

    inside: Rect,

    layout: Layout,

    pub last_name: String,
}

impl Ui {
    pub(crate) fn new(start_dir: &str) -> Ui {
        Ui {
            cursor_y: 0,
            scroll_y: 0,

            inside: Rect::new(0, 0, 0, 0),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(15),
                    Constraint::Min(20),
                    Constraint::Max(40),
                ]),
            last_name: String::from(start_dir),
        }
    }

    pub(crate) fn draw_app<B: Backend>(
        &mut self,
        term: &mut Terminal<B>,
        title: &str,
        bookmarks: &Vec<Bookmark>,
        dir_contents: &Vec<DirEntry>,
        command_mode: bool,
        command_buffer: &str,
    ) -> io::Result<()> {
        term.draw(|f| {
            // Border
            let size = f.size();
            let block = Block::default()
                .title(Span::styled(
                    title.replace("\\", "/"),
                    Style::default().add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL);

            // Layout
            self.inside = block.inner(size);
            self.inside.x = self.inside.x + 1;
            self.inside.width = self.inside.width - 2;

            let chunks = self.layout.split(self.inside);

            // Bookmarks
            let mut bookmarks_disp = vec![];
            for b in bookmarks {
                let s = Style::default();
                bookmarks_disp.push(ListItem::new(b.name.clone()).style(s));
            }
            let bookmark_list = List::new(bookmarks_disp);

            // File list
            let mut items = vec![];
            let mut i = 0;
            for p in dir_contents {
                let mut s = Style::default();
                if p.file_type().unwrap().is_dir() {
                    s = s.fg(Color::Blue).add_modifier(Modifier::BOLD);

                    if i == self.scroll_y + self.cursor_y {
                        s = s
                            .fg(Color::Black)
                            .bg(Color::Blue)
                            .add_modifier(Modifier::BOLD);
                    }
                } else {
                    if i == self.scroll_y + self.cursor_y {
                        s = s.fg(Color::Black).bg(Color::Blue);
                    }
                }

                if i >= self.scroll_y && i - self.scroll_y < self.inside.height as i32 {
                    items.push(ListItem::new(p.file_name().into_string().unwrap()).style(s));
                }
                i = i + 1;
            }
            let item_list = List::new(items);

            // Debug info
            let debug = Block::default().title(format!(
                "Cursor: {} Scroll: {}",
                self.cursor_y, self.scroll_y
            ));

            // Command mode
            let cmd_text = Span::styled(format!(":{}", command_buffer), Style::default());
            let cmd_line = Paragraph::new(cmd_text)
                .block(Block::default())
                .wrap(Wrap { trim: true });

            f.render_widget(block, size);
            f.render_widget(bookmark_list.clone(), chunks[0]);
            f.render_widget(item_list.clone(), chunks[1]);
            f.render_widget(debug, chunks[2]);

            if command_mode {
            f.render_widget(
                cmd_line,
                Rect {
                    x: 1,
                    y: size.height - 2,
                    width: size.width - 1,
                    height: 1,
                },
            );
            }
        })?;

        Ok(())
    }

    pub(crate) fn scroll(&mut self, y: i32, max: i32) {
        self.cursor_y = std::cmp::min(self.cursor_y + y, max - 1);

        if self.cursor_y < 0 {
            self.cursor_y = 0;
            self.scroll_y = self.scroll_y + y;

            if self.scroll_y < 0 {
                self.scroll_y = 0;
            }
        } else if self.cursor_y >= self.inside.height as i32 {
            self.cursor_y = self.inside.height as i32 - 1;
            self.scroll_y = self.scroll_y + y;
        }
    }

    pub(crate) fn scroll_abs(&mut self, y: i32, max: i32) {
        self.cursor_y = 0;
        self.scroll_y = 0;
        self.scroll(y, max);
    }
}
