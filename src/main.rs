use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
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
                }
            }

            KeyCode::Down => {
                if self.selected + 1 < self.files.len() {
                    self.selected += 1;
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

        let file_lines: Vec<Line> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, name)| {
                if i == self.selected {
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
        file_paragraph.render(chunks[0], buf);

        let preview_paragraph = Paragraph::new(Text::from("placeholder")).block(
            Block::bordered()
                .title(" Preview ".blue().bold().into_right_aligned_line())
                .border_set(border::PLAIN),
        );
        preview_paragraph.render(chunks[1], buf);
    }
}
