use base64::Engine as _;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal,
};
use ratatui::{
    DefaultTerminal, Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::new()?.run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
pub struct App {
    current_dir: PathBuf,
    files: Vec<String>,
    selected: usize,
    scroll: usize,
    exit: bool,
}

impl App {
    pub fn new() -> std::io::Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let files = Self::read_dir(&home)?;
        Ok(Self {
            current_dir: home,
            files,
            selected: 0,
            scroll: 0,
            exit: false,
        })
    }

    pub fn read_dir(path: &PathBuf) -> std::io::Result<Vec<String>> {
        let mut entries = vec![];
        entries.push("..".into());
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type()?.is_dir() {
                entries.push(format!("{}/", file_name));
            } else {
                entries.push(file_name);
            }
        }
        entries.sort();
        Ok(entries)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_event()?;
        }
        Ok(())
    }

    pub fn handle_event(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),

            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    if self.selected < self.scroll as usize {
                        self.scroll = self.selected;
                    }
                }
            }

            KeyCode::Down => {
                if self.selected + 1 < self.files.len() {
                    self.selected += 1;
                    if self.selected >= self.scroll {
                        self.scroll = self.selected;
                    }
                }
            }

            KeyCode::Enter => {
                if let Some(name) = self.files.get(self.selected).cloned() {
                    if name == ".." {
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                        }
                    } else {
                        let candidate = self.current_dir.join(&name.trim_end_matches('/'));
                        if candidate.is_dir() {
                            self.current_dir = candidate;
                        }
                    }
                    if let Ok(new_files) = Self::read_dir(&self.current_dir) {
                        self.files = new_files;
                        self.selected = 0;
                        self.scroll = 0;
                    }
                }
            }

            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    pub fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("< Tmages - image converter TUI >".green().bold());
        let instructions = Line::from(vec![
            " Up/Down ".into(),
            "<↑/↓>".blue().bold(),
            " Enter ".into(),
            "<↵>".blue().bold(),
            " Quit ".into(),
            "<Q>".red().bold(),
        ]);

        let outer = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::EMPTY);

        let inner = outer.inner(area);
        outer.render(area, buf);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(inner);

        let list_rect = chunks[0];
        let preview_rect = chunks[1];

        let selected_path = self
            .current_dir
            .join(self.files[self.selected].trim_end_matches('/'));
        if selected_path.is_file() {
            if let Some(ext) = selected_path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                if ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&ext.as_str()) {
                    // Display image in the preview area
                    let _ = kitty_display_image(
                        selected_path.to_str().unwrap(),
                        preview_rect.x,
                        preview_rect.y,
                        preview_rect.width,
                        preview_rect.height,
                    );
                }
            }
        }

        Block::bordered()
            .title(" Preview ".blue().bold().into_right_aligned_line())
            .border_set(border::PLAIN)
            .render(preview_rect, buf);

        let max_visible = chunks[0].height.saturating_sub(2) as usize;

        let total = self.files.len();
        let mut scroll = self.scroll;

        if self.selected >= scroll + max_visible {
            scroll = self.selected + 1 - max_visible;
        }
        if self.selected < scroll {
            scroll = self.selected;
        }

        let start = self.scroll.min(total);
        let end = (start + max_visible).min(total);

        let file_lines: Vec<Line> = self.files[start as usize..end as usize]
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let absolute_index = start + i;
                if absolute_index == self.selected {
                    Line::from(name.clone()).style(
                        Style::default()
                            .bg(ratatui::style::Color::Blue)
                            .fg(ratatui::style::Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::from(name.clone())
                }
            })
            .collect();

        let file_paragraph = Paragraph::new(Text::from(file_lines)).block(
            Block::bordered()
                .title(format!(" Directory: {}", self.current_dir.display()).blue())
                .border_set(border::PLAIN),
        );
        file_paragraph.render(list_rect, buf);

        Block::bordered()
            .title(" Preview ".blue().bold().into_right_aligned_line())
            .border_set(border::PLAIN)
            .render(preview_rect, buf);
    }
}

fn kitty_display_image(path: &str, x: u16, y: u16, w: u16, h: u16) -> io::Result<()> {
    let data = fs::read(path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(data);
    print!("\x1b_Ga=d\x1b\\");
    let kitty_x = x + 2; // Add 1 for 1-indexing + 1 for border
    let kitty_y = y + 2; // Add 1 for 1-indexing + 1 for border
    let kitty_w = w.saturating_sub(2); // Subtract border width
    let kitty_h = h.saturating_sub(2); // Subtract border height
    print!(
        "\x1b_Gf=100,a=T,C=1,q=2,X={},Y={},c={},r={};{}\x1b\\",
        kitty_x, kitty_y, kitty_w, kitty_h, b64
    );
    io::stdout().flush()
}
