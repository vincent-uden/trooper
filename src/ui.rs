use std::{fs::DirEntry, io};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};

use crate::app::{ActiveMode, ActivePanel, Bookmark};

pub struct Ui {
    pub cursor_y: i32,
    pub scroll_y: i32,

    pub bookmark_y: i32,
    pub bookmark_scroll_y: i32,

    /* This position can be off screen */
    pub visual_intitial_y: i32,

    inside: Rect,

    layout: Layout,

    pub last_name: String,
    pub bookmark_width: u16,

    pub debug_msg: String,
}

impl Ui {
    pub(crate) fn new(start_dir: &str) -> Ui {
        Ui {
            cursor_y: 0,
            scroll_y: 0,

            bookmark_y: 0,
            bookmark_scroll_y: 0,

            visual_intitial_y: 0,

            inside: Rect::new(0, 0, 0, 0),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(15), Constraint::Min(20)]),
            last_name: String::from(start_dir),
            bookmark_width: 15,
            debug_msg: String::new(),
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
        active_panel: &ActivePanel,
        active_mode: &ActiveMode,
        selection_start: i32,
    ) -> io::Result<()> {
        term.draw(|f| {
            self.layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(self.bookmark_width), Constraint::Min(20)]);

            // Border
            let size = f.size();
            let block = Block::default()
                .title(Span::styled(
                    title.replace("\\", "/"),
                    Style::default().add_modifier(Modifier::BOLD),
                ))
                .title_alignment(tui::layout::Alignment::Center)
                .borders(Borders::ALL);

            // Layout
            self.inside = block.inner(size);
            self.inside.x = self.inside.x + 1;
            self.inside.width = self.inside.width - 2;

            let chunks = self.layout.split(self.inside);
            let main_block = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray));

            // Bookmarks
            let mut bookmarks_disp = vec![];
            let mut i = 0;
            for b in bookmarks {
                let mut s = Style::default();

                if i == self.bookmark_scroll_y + self.bookmark_y
                    && *active_panel == ActivePanel::Bookmarks
                {
                    s = s
                        .fg(Color::Black)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD);
                }

                bookmarks_disp.push(ListItem::new(b.name.clone()).style(s));

                i = i + 1;
            }
            let bookmark_list = List::new(bookmarks_disp);

            // File list
            let mut items = vec![];
            i = 0;
            for p in dir_contents {
                let mut s = Style::default();
                if p.file_type().unwrap().is_dir() {
                    s = s.fg(Color::Blue).add_modifier(Modifier::BOLD);

                    if ((i <= self.scroll_y + self.cursor_y && i >= selection_start)
                        || (i >= self.scroll_y + self.cursor_y && i <= selection_start))
                        && *active_panel == ActivePanel::Main
                    {
                        s = s
                            .fg(Color::Black)
                            .bg(Color::Blue)
                            .add_modifier(Modifier::BOLD);
                    }
                } else {
                    if ((i <= self.scroll_y + self.cursor_y && i >= selection_start)
                        || (i >= self.scroll_y + self.cursor_y && i <= selection_start))
                        && *active_panel == ActivePanel::Main
                    {
                        s = s.fg(Color::Black).bg(Color::Blue);
                    }
                }

                if i >= self.scroll_y && i - self.scroll_y < self.inside.height as i32 {
                    items.push(ListItem::new(p.file_name().into_string().unwrap()).style(s));
                }
                i = i + 1;
            }
            let item_list = List::new(items);

            // Command mode
            let cmd_text = Span::styled(format!(":{}", command_buffer), Style::default());
            let cmd_line = Paragraph::new(cmd_text)
                .block(Block::default())
                .wrap(Wrap { trim: true });

            let inner_main_block = main_block.inner(chunks[1]);
            f.render_widget(block, size);
            f.render_widget(bookmark_list.clone(), chunks[0]);
            f.render_widget(main_block, chunks[1]);
            f.render_widget(item_list.clone(), inner_main_block);

            let debug_text = Span::styled(&self.debug_msg, Style::default());
            let debug_line = Paragraph::new(debug_text);
            f.render_widget(
                debug_line,
                Rect {
                    x: ((size.width as usize) - self.debug_msg.len() - 2) as u16,
                    y: 2,
                    width: self.debug_msg.len() as u16,
                    height: 1,
                },
            );

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

            let mode_style = Style::default().add_modifier(Modifier::BOLD).fg(match active_mode {
                ActiveMode::Normal => Color::Green,
                ActiveMode::Command => Color::Magenta,
                ActiveMode::Visual => Color::Blue,
            });
            let active_mode_text = Span::styled(format!("{}", active_mode), mode_style);
            let active_mode_line = Paragraph::new(active_mode_text)
                .block(Block::default())
                .wrap(Wrap { trim: true });
            f.render_widget(
                active_mode_line,
                Rect {
                    x: 2,
                    y: size.height - 3,
                    width: size.width - 2,
                    height: 1,
                },
            )
        })?;

        Ok(())
    }

    pub(crate) fn scroll(&mut self, y: i32, max: i32, active_panel: &ActivePanel) {
        match active_panel {
            ActivePanel::Main => {
                self.cursor_y = std::cmp::min(self.cursor_y + y, max - 1);

                if self.cursor_y < 0 {
                    self.cursor_y = 0;
                    self.scroll_y = self.scroll_y + y;

                    if self.scroll_y < 0 {
                        self.scroll_y = 0;
                    }
                } else if self.cursor_y >= self.inside.height as i32 {
                    self.cursor_y = self.inside.height as i32 - 1;
                    self.scroll_y =
                        std::cmp::min(self.scroll_y + y, max - self.inside.height as i32);
                }
            }
            ActivePanel::Bookmarks => {
                self.bookmark_y = std::cmp::min(self.bookmark_y + y, max - 1);

                if self.bookmark_y < 0 {
                    self.bookmark_y = 0;
                    self.bookmark_scroll_y = self.bookmark_y + y;

                    if self.bookmark_scroll_y < 0 {
                        self.bookmark_scroll_y = 0;
                    }
                } else if self.bookmark_y >= self.inside.height as i32 {
                    self.bookmark_y = self.inside.height as i32 - 1;
                    self.bookmark_scroll_y =
                        std::cmp::min(self.bookmark_scroll_y + y, max - self.inside.height as i32);
                }
            }
        }
    }

    pub(crate) fn scroll_abs(&mut self, y: i32, max: i32, active_panel: &ActivePanel) {
        self.cursor_y = 0;
        self.scroll_y = 0;
        self.scroll(y, max, active_panel);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::ActivePanel;

    use super::Ui;

    #[test]
    fn scroll_past_end() {
        let mut ui = Ui::new(".");
        ui.inside.height = 30;

        ui.scroll_abs(60, 60, &ActivePanel::Main);
        assert!(
            ui.scroll_y + ui.cursor_y == 59,
            "Scrolled to index {}",
            ui.scroll_y + ui.cursor_y
        );

        ui.scroll(1, 60, &ActivePanel::Main);
        assert!(
            ui.scroll_y + ui.cursor_y == 59,
            "Scrolled to index {}",
            ui.scroll_y + ui.cursor_y
        );
    }
}
