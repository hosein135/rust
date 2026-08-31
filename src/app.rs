//! Main IDE application (iced, CPU software renderer).

use crate::editor::{cursor_from_line_col, cursor_line_col};
use crate::project::{load_file, save_file, IdeProject, OpenFile, TreeNode};
use crate::templates::{self, counter_example};
use iced::highlighter;
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, text_editor,
    text_input, Space,
};
use iced::{Element, Fill, Font, Length, Task, Theme};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const SIDEBAR_WIDTH: f32 = 240.0;
const BOTTOM_HEIGHT: f32 = 180.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BottomTab {
    Console,
    Problems,
}

#[derive(Clone)]
enum Dialog {
    NewModule { name: String },
    NewTestbench { dut: String },
    Find { query: String },
    About,
}

pub struct VerilogIde {
    project: Option<IdeProject>,
    tree: Option<TreeNode>,
    expanded: HashSet<PathBuf>,
    open: Vec<OpenFile>,
    active: Option<usize>,
    editor_content: text_editor::Content,
    editor_theme: highlighter::Theme,
    console: String,
    problems: Vec<String>,
    bottom: BottomTab,
    status: String,
    dialog: Option<Dialog>,
    search_query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenProject,
    ProjectPicked(Result<Option<PathBuf>, DialogError>),
    CreateSample,
    SampleFolderPicked(Result<Option<PathBuf>, DialogError>),
    OpenFile(PathBuf),
    ToggleDir(PathBuf),
    SelectTab(usize),
    CloseTab(usize),
    EditorAction(text_editor::Action),
    Save,
    SaveAll,
    ShowNewModule,
    ShowNewTestbench,
    ShowFind,
    ShowAbout,
    DialogInput(String),
    DialogConfirm,
    DialogCancel,
    BottomTabSelected(BottomTab),
    ClearBottom,
}

#[derive(Debug, Clone)]
pub enum DialogError {
    Cancelled,
}

pub fn run() -> iced::Result {
    iced::application("Verilog IDE", VerilogIde::update, VerilogIde::view)
        .theme(|_| Theme::Dark)
        .default_font(Font::MONOSPACE)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(900.0, 560.0)),
            ..Default::default()
        })
        .run_with(VerilogIde::new)
}

impl VerilogIde {
    fn new() -> (Self, Task<Message>) {
        let mut ide = Self {
            project: None,
            tree: None,
            expanded: HashSet::new(),
            open: Vec::new(),
            active: None,
            editor_content: text_editor::Content::with_text(
                "Open a Verilog file to start editing...\n",
            ),
            editor_theme: highlighter::Theme::Base16Ocean,
            console: "Verilog IDE ready.\nOpen a folder or create a sample project.\n".into(),
            problems: Vec::new(),
            bottom: BottomTab::Console,
            status: "Ready".into(),
            dialog: None,
            search_query: String::new(),
        };

        for candidate in [
            PathBuf::from("samples"),
            PathBuf::from("examples"),
            std::env::current_dir()
                .ok()
                .map(|p| p.join("samples"))
                .unwrap_or_default(),
        ] {
            if candidate.is_dir() {
                ide.open_project(candidate);
                break;
            }
        }

        (ide, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenProject => {
                Task::perform(pick_folder("Open project folder"), Message::ProjectPicked)
            }
            Message::ProjectPicked(Ok(Some(path))) => {
                self.open_project(path);
                Task::none()
            }
            Message::ProjectPicked(_) => Task::none(),
            Message::CreateSample => Task::perform(
                pick_folder("Choose parent folder for sample project"),
                Message::SampleFolderPicked,
            ),
            Message::SampleFolderPicked(Ok(Some(parent))) => {
                let root = parent.join("verilog-sample");
                if let Err(e) = std::fs::create_dir_all(&root) {
                    self.log_err(&e.to_string());
                } else {
                    let (n1, c1, n2, c2) = counter_example();
                    let _ = std::fs::write(root.join(n1), c1);
                    let _ = std::fs::write(root.join(n2), c2);
                    self.open_project(root);
                    self.log("Created sample counter + testbench project.\n");
                }
                Task::none()
            }
            Message::SampleFolderPicked(_) => Task::none(),
            Message::OpenFile(path) => {
                self.open_path(&path);
                Task::none()
            }
            Message::ToggleDir(path) => {
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                Task::none()
            }
            Message::SelectTab(idx) => {
                self.sync_editor_to_active();
                self.active = Some(idx);
                self.load_active_into_editor();
                Task::none()
            }
            Message::CloseTab(idx) => {
                self.sync_editor_to_active();
                if self.open.get(idx).map(|f| f.dirty).unwrap_or(false) {
                    let _ = save_file(&mut self.open[idx]);
                }
                self.open.remove(idx);
                self.active = if self.open.is_empty() {
                    None
                } else {
                    Some(idx.min(self.open.len() - 1))
                };
                self.load_active_into_editor();
                Task::none()
            }
            Message::EditorAction(action) => {
                if action.is_edit() {
                    if let Some(i) = self.active {
                        if let Some(file) = self.open.get_mut(i) {
                            file.dirty = true;
                        }
                    }
                }
                self.editor_content.perform(action);
                self.sync_editor_to_active();
                Task::none()
            }
            Message::Save => {
                self.sync_editor_to_active();
                if let Some(i) = self.active {
                    match save_file(&mut self.open[i]) {
                        Ok(()) => {
                            let path = self.open[i].path.display().to_string();
                            self.status = format!("Saved {path}");
                            self.log(&format!("Saved {path}\n"));
                        }
                        Err(e) => self.log_err(&e),
                    }
                }
                Task::none()
            }
            Message::SaveAll => {
                self.sync_editor_to_active();
                let mut errors = Vec::new();
                for f in &mut self.open {
                    if f.dirty {
                        if let Err(e) = save_file(f) {
                            errors.push(e);
                        }
                    }
                }
                for e in errors {
                    self.problems.push(e.clone());
                    self.log(&format!("ERROR: {e}\n"));
                }
                self.status = "Saved all".into();
                Task::none()
            }
            Message::ShowNewModule => {
                self.dialog = Some(Dialog::NewModule {
                    name: String::new(),
                });
                Task::none()
            }
            Message::ShowNewTestbench => {
                self.dialog = Some(Dialog::NewTestbench { dut: String::new() });
                Task::none()
            }
            Message::ShowFind => {
                self.dialog = Some(Dialog::Find {
                    query: self.search_query.clone(),
                });
                Task::none()
            }
            Message::ShowAbout => {
                self.dialog = Some(Dialog::About);
                Task::none()
            }
            Message::DialogInput(value) => {
                match &mut self.dialog {
                    Some(Dialog::NewModule { name }) => *name = value,
                    Some(Dialog::NewTestbench { dut }) => *dut = value,
                    Some(Dialog::Find { query }) => *query = value,
                    _ => {}
                }
                Task::none()
            }
            Message::DialogConfirm => {
                let dialog = self.dialog.take();
                if let Some(dialog) = dialog {
                    match dialog {
                        Dialog::NewModule { name } => self.create_module(&name),
                        Dialog::NewTestbench { dut } => self.create_testbench(&dut),
                        Dialog::Find { query } => {
                            self.search_query = query;
                            self.find_next();
                        }
                        Dialog::About => {}
                    }
                }
                Task::none()
            }
            Message::DialogCancel => {
                self.dialog = None;
                Task::none()
            }
            Message::BottomTabSelected(tab) => {
                self.bottom = tab;
                Task::none()
            }
            Message::ClearBottom => {
                match self.bottom {
                    BottomTab::Console => self.console.clear(),
                    BottomTab::Problems => self.problems.clear(),
                }
                Task::none()
            }
        }
    }

    fn open_project(&mut self, root: PathBuf) {
        let project = IdeProject::new(root);
        self.log(&format!("Opened project: {}\n", project.root.display()));
        self.tree = Some(project.build_tree());
        self.expanded.insert(project.root.clone());
        self.project = Some(project);
        self.open.clear();
        self.active = None;
        self.status = "Project opened".into();

        if let Some(p) = self.project.as_ref() {
            if let Some(first) = p.list_verilog_files().into_iter().next() {
                self.open_path(&first);
            }
        }
    }

    fn open_path(&mut self, path: &Path) {
        self.sync_editor_to_active();

        if let Some(idx) = self.open.iter().position(|f| f.path == path) {
            self.active = Some(idx);
            self.load_active_into_editor();
            return;
        }

        match load_file(path) {
            Ok(file) => {
                self.open.push(file);
                self.active = Some(self.open.len() - 1);
                self.status = format!("Opened {}", path.display());
                self.load_active_into_editor();
            }
            Err(e) => {
                self.log_err(&e);
                self.problems.push(e);
            }
        }
    }

    fn sync_editor_to_active(&mut self) {
        if let Some(i) = self.active {
            if let Some(file) = self.open.get_mut(i) {
                file.content = self.editor_content.text();
                let (line, col) = self.editor_content.cursor_position();
                file.cursor = cursor_from_line_col(&file.content, line, col);
            }
        }
    }

    fn load_active_into_editor(&mut self) {
        if let Some(i) = self.active {
            if let Some(file) = self.open.get(i) {
                self.editor_content = text_editor::Content::with_text(&file.content);
            }
        } else {
            self.editor_content = text_editor::Content::with_text(
                "Open a Verilog file to start editing...\n",
            );
        }
    }

    fn log(&mut self, msg: &str) {
        self.console.push_str(msg);
    }

    fn log_err(&mut self, msg: &str) {
        self.console.push_str(&format!("ERROR: {msg}\n"));
        self.status = msg.to_string();
        self.problems.push(msg.to_string());
    }

    fn create_module(&mut self, name: &str) {
        if name.trim().is_empty() {
            return;
        }
        let Some(project) = self.project.as_ref() else {
            self.log_err("Open a project first.");
            return;
        };
        let path = project.root.join(format!("{}.v", name.trim()));
        let body = templates::module_template(name.trim());
        if let Err(e) = std::fs::write(&path, body) {
            self.log_err(&e.to_string());
            return;
        }
        self.tree = Some(project.build_tree());
        self.open_path(&path);
        self.log(&format!("Created module {}\n", path.display()));
    }

    fn create_testbench(&mut self, dut: &str) {
        if dut.trim().is_empty() {
            return;
        }
        let Some(project) = self.project.as_ref() else {
            self.log_err("Open a project first.");
            return;
        };
        let path = project.root.join(format!("{}_tb.v", dut.trim()));
        let body = templates::testbench_template(dut.trim());
        if let Err(e) = std::fs::write(&path, body) {
            self.log_err(&e.to_string());
            return;
        }
        self.tree = Some(project.build_tree());
        self.open_path(&path);
        self.log(&format!("Created testbench {}\n", path.display()));
    }

    fn find_next(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        self.sync_editor_to_active();
        let Some(i) = self.active else { return };
        let content = self.open[i].content.clone();
        let start = self.open[i].cursor.saturating_add(1).min(content.len());
        let found = content[start..]
            .find(&self.search_query)
            .map(|o| start + o)
            .or_else(|| content.find(&self.search_query));
        if let Some(pos) = found {
            self.open[i].cursor = pos;
            self.status = format!("Found at {pos}");
            self.load_active_into_editor();
        } else {
            self.status = "Not found".into();
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            self.view_toolbar(),
            row![
                container(self.view_sidebar())
                    .width(Length::Fixed(SIDEBAR_WIDTH))
                    .height(Fill)
                    .padding(0),
                column![
                    container(self.view_editor_area()).height(Fill),
                    container(self.view_bottom())
                        .height(Length::Fixed(BOTTOM_HEIGHT))
                        .width(Fill),
                ]
                .width(Fill)
                .spacing(0),
            ]
            .spacing(0)
            .height(Fill),
            self.view_status_bar(),
        ]
        .spacing(0)
        .padding(0)
        .into()
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        row![
            button("Open").on_press(Message::OpenProject),
            button("Save")
                .on_press_maybe(self.active.map(|_| Message::Save)),
            button("Save All")
                .on_press_maybe(if self.open.is_empty() {
                    None
                } else {
                    Some(Message::SaveAll)
                }),
            button("+ Module").on_press(Message::ShowNewModule),
            button("+ Testbench").on_press(Message::ShowNewTestbench),
            button("Sample").on_press(Message::CreateSample),
            button("Find").on_press(Message::ShowFind),
            button("About").on_press(Message::ShowAbout),
            horizontal_space(),
            text(
                self.project
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "No project".into())
            )
            .size(14),
        ]
        .spacing(8)
        .padding(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let header = text("Explorer").size(14);
        let body: Element<'_, Message> = if let Some(tree) = &self.tree {
            scrollable(
                column(self.render_tree_nodes(tree, 0))
                    .spacing(2)
                    .padding(4),
            )
            .height(Fill)
            .into()
        } else {
            column![
                text("No project open.").size(13),
                button("Open Project...").on_press(Message::OpenProject),
                button("Create Sample...").on_press(Message::CreateSample),
            ]
            .spacing(8)
            .padding(8)
            .into()
        };

        column![header, body].spacing(4).padding(4).into()
    }

    fn render_tree_nodes<'a>(
        &'a self,
        node: &'a TreeNode,
        depth: usize,
    ) -> Vec<Element<'a, Message>> {
        let mut items = Vec::new();
        if depth > 0 {
            let indent = depth as f32 * 14.0;
            if node.is_dir {
                let expanded = self.expanded.contains(&node.path);
                let label = if expanded { "▾" } else { "▸" };
                items.push(
                    row![
                        Space::new(Length::Fixed(indent), Length::Shrink),
                        button(text(format!("{label} {}", node.name)))
                            .on_press(Message::ToggleDir(node.path.clone()))
                            .padding([2, 4]),
                    ]
                    .into(),
                );
            } else {
                items.push(
                    row![
                        Space::new(Length::Fixed(indent), Length::Shrink),
                        button(text(format!("  {}", node.name)))
                            .on_press(Message::OpenFile(node.path.clone()))
                            .padding([2, 4]),
                    ]
                    .into(),
                );
            }
        }

        if node.is_dir && (depth == 0 || self.expanded.contains(&node.path)) {
            for child in &node.children {
                items.extend(self.render_tree_nodes(child, depth + 1));
            }
        }

        items
    }

    fn view_editor_area(&self) -> Element<'_, Message> {
        if self.open.is_empty() {
            return container(
                column![
                    text("Verilog IDE").size(28),
                    text("Edit Verilog modules and testbenches.").size(14),
                    button("Open Project Folder").on_press(Message::OpenProject),
                    button("Create Sample Counter Project").on_press(Message::CreateSample),
                ]
                .spacing(12)
                .align_x(iced::Alignment::Center),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into();
        }

        let tabs = row(
            self.open.iter().enumerate().map(|(i, f)| {
                let name = f
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled");
                let title = if f.dirty {
                    format!("* {name}")
                } else {
                    name.to_string()
                };
                row![
                    button(text(title)).on_press(Message::SelectTab(i)),
                    button("×")
                        .on_press(Message::CloseTab(i))
                        .padding([2, 6]),
                ]
                .spacing(2)
                .into()
            }),
        )
        .spacing(4)
        .padding([4, 8]);

        let editor = text_editor(&self.editor_content)
            .height(Fill)
            .on_action(Message::EditorAction)
            .highlight("v", self.editor_theme);

        let overlay = self.view_dialog();

        column![tabs, editor, overlay]
            .spacing(0)
            .height(Fill)
            .into()
    }

    fn view_bottom(&self) -> Element<'_, Message> {
        let tabs = row![
            button("Console").on_press(Message::BottomTabSelected(BottomTab::Console)),
            button(text(format!("Problems ({})", self.problems.len())))
                .on_press(Message::BottomTabSelected(BottomTab::Problems)),
            horizontal_space(),
            button("Clear").on_press(Message::ClearBottom),
        ]
        .spacing(8)
        .padding([4, 8]);

        let body: Element<'_, Message> = match self.bottom {
            BottomTab::Console => scrollable(
                text(self.console.as_str()).size(13).font(Font::MONOSPACE),
            )
            .height(Fill)
            .into(),
            BottomTab::Problems => {
                if self.problems.is_empty() {
                    text("No problems.").size(13).into()
                } else {
                    scrollable(
                        column(
                            self.problems
                                .iter()
                                .enumerate()
                                .map(|(i, p)| {
                                    text(format!("{}. {p}", i + 1)).size(13).into()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .spacing(4)
                        .padding(8),
                    )
                    .height(Fill)
                    .into()
                }
            }
        };

        column![tabs, body].spacing(0).height(Fill).into()
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        let detail = if let Some(i) = self.active {
            if let Some(f) = self.open.get(i) {
                let (line, col) = cursor_line_col(&f.content, f.cursor);
                let dirty = if f.dirty { " *" } else { "" };
                format!(
                    "Ln {line}, Col {col}{dirty}  |  {}",
                    f.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                )
            } else {
                self.status.clone()
            }
        } else {
            self.status.clone()
        };

        row![
            text(&self.status).size(12),
            horizontal_space(),
            text(detail).size(12),
        ]
        .padding([4, 8])
        .into()
    }

    fn view_dialog(&self) -> Element<'_, Message> {
        let Some(dialog) = &self.dialog else {
            return Space::new(Length::Shrink, Length::Fixed(0.0)).into();
        };

        let (title, input, show_input): (String, String, bool) = match dialog {
            Dialog::NewModule { name } => ("New Verilog Module".into(), name.clone(), true),
            Dialog::NewTestbench { dut } => ("New Testbench".into(), dut.clone(), true),
            Dialog::Find { query } => ("Find".into(), query.clone(), true),
            Dialog::About => ("About Verilog IDE".into(), String::new(), false),
        };

        let content: Element<'_, Message> = if show_input {
            column![
                text(title).size(16),
                text_input("…", &input).on_input(Message::DialogInput),
                row![
                    button("OK").on_press(Message::DialogConfirm),
                    button("Cancel").on_press(Message::DialogCancel),
                ]
                .spacing(8),
            ]
            .spacing(8)
            .into()
        } else {
            column![
                text("About Verilog IDE").size(16),
                text("Desktop IDE for Verilog HDL and testbenches."),
                text("Built with Rust + iced (software renderer)."),
                button("OK").on_press(Message::DialogCancel),
            ]
            .spacing(8)
            .into()
        };

        container(content)
            .padding(16)
            .style(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.palette().background)),
                border: iced::Border {
                    color: theme.palette().text,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

async fn pick_folder(title: &str) -> Result<Option<PathBuf>, DialogError> {
    let picked = rfd::AsyncFileDialog::new()
        .set_title(title)
        .pick_folder()
        .await
        .ok_or(DialogError::Cancelled)?;
    Ok(Some(picked.path().to_path_buf()))
}
