//! Main IDE application (iced, VS Code–inspired layout).

use crate::editor::{
    self, cursor_from_line_col, cursor_line_col, EDITOR_FONT_SIZE, EDITOR_LINE_HEIGHT,
    EDITOR_PADDING,
};
use crate::verilog_highlighter::{
    self, Settings as VerilogHighlightSettings, VerilogHighlighter,
};
use crate::project::{
    collect_dir_paths, find_first_verilog, load_file, locate_samples_dir, save_file, IdeProject,
    OpenFile, TreeNode,
};
use crate::sim::{self, SimResult};
use crate::templates::{self, counter_example};
use iced::keyboard::{self, Key};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, mouse_area, row, scrollable,
    stack, text, text_editor, text_input, Column, Space,
};
use iced::{
    Alignment, Border, Color, Element, Fill, Font, Length, Padding, Point, Shadow, Subscription,
    Task, Theme,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const ACTIVITY_WIDTH: f32 = 48.0;
const SIDEBAR_WIDTH: f32 = 260.0;
const BOTTOM_HEIGHT: f32 = 160.0;
const MENU_HEIGHT: f32 = 36.0;
const TAB_HEIGHT: f32 = 36.0;
const STATUS_HEIGHT: f32 = 24.0;

// VS Code dark palette (approximate)
const BG_EDITOR: Color = Color::from_rgb(0.12, 0.12, 0.12);
const BG_SIDEBAR: Color = Color::from_rgb(0.15, 0.15, 0.16);
const BG_ACTIVITY: Color = Color::from_rgb(0.20, 0.20, 0.20);
const BG_TABS: Color = Color::from_rgb(0.18, 0.18, 0.18);
const BG_TAB_ACTIVE: Color = Color::from_rgb(0.12, 0.12, 0.12);
const BG_STATUS: Color = Color::from_rgb(0.0, 0.47, 0.80);
const BG_MENU: Color = Color::from_rgb(0.22, 0.22, 0.22);
const FG_MUTED: Color = Color::from_rgb(0.55, 0.55, 0.55);
const FG_TEXT: Color = Color::from_rgb(0.85, 0.85, 0.85);
const BORDER: Color = Color::from_rgb(0.28, 0.28, 0.28);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BottomTab {
    Console,
    Problems,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TopMenu {
    File,
    Edit,
    Run,
    Help,
}

#[derive(Clone)]
enum Dialog {
    NewFile { name: String },
    NewFolder { name: String },
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
    editor_scroll_y: f32,
    editor_view_h: f32,
    console: String,
    problems: Vec<String>,
    bottom: BottomTab,
    bottom_visible: bool,
    status: String,
    dialog: Option<Dialog>,
    menu_open: Option<TopMenu>,
    search_query: String,
    sim_running: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenProject,
    ProjectPicked(Result<Option<PathBuf>, DialogError>),
    OpenFilePicker,
    FilePicked(Result<Option<PathBuf>, DialogError>),
    CreateSample,
    SampleFolderPicked(Result<Option<PathBuf>, DialogError>),
    OpenFile(PathBuf),
    ToggleDir(PathBuf),
    SelectTab(usize),
    CloseTab(usize),
    EditorAction(text_editor::Action),
    EditorScrolled { offset_y: f32, view_h: f32 },
    EditorPage(i32),
    Save,
    SaveAll,
    CloseProject,
    RefreshExplorer,
    ShowNewFile,
    ShowNewFolder,
    ShowFind,
    ShowAbout,
    DialogInput(String),
    DialogConfirm,
    DialogCancel,
    BottomTabSelected(BottomTab),
    ToggleBottomPanel,
    ClearBottom,
    MenuToggle(TopMenu),
    MenuClose,
    RunSim,
    SimFinished(SimResult),
}

#[derive(Debug, Clone)]
pub enum DialogError {
    Cancelled,
}

pub fn run() -> iced::Result {
    iced::application("Verilog IDE", VerilogIde::update, VerilogIde::view)
        .theme(|_| Theme::Dark)
        .subscription(VerilogIde::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(900.0, 560.0)),
            ..Default::default()
        })
        .run_with(VerilogIde::new)
}

impl VerilogIde {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                project: None,
                tree: None,
                expanded: HashSet::new(),
                open: Vec::new(),
                active: None,
                editor_content: text_editor::Content::new(),
                editor_scroll_y: 0.0,
                editor_view_h: 480.0,
                console: "Verilog IDE ready.\n".into(),
                problems: Vec::new(),
                bottom: BottomTab::Console,
                bottom_visible: true,
                status: "Ready".into(),
                dialog: None,
                menu_open: None,
                search_query: String::new(),
                sim_running: false,
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, _modifiers| {
            if matches!(key, Key::Named(keyboard::key::Named::F5)) {
                Some(Message::RunSim)
            } else {
                None
            }
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuToggle(menu) => {
                self.menu_open = if self.menu_open == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
                Task::none()
            }
            Message::MenuClose => {
                self.menu_open = None;
                Task::none()
            }
            Message::OpenProject => {
                self.menu_open = None;
                Task::perform(pick_folder("Open Folder"), Message::ProjectPicked)
            }
            Message::ProjectPicked(Ok(Some(path))) => {
                self.open_folder_path(path);
                editor::scroll_to_y(0.0)
            }
            Message::ProjectPicked(_) => Task::none(),
            Message::OpenFilePicker => {
                self.menu_open = None;
                Task::perform(pick_file("Open File"), Message::FilePicked)
            }
            Message::FilePicked(Ok(Some(path))) => {
                if path.is_dir() {
                    self.open_folder_path(path);
                } else {
                    if self.project.is_none() {
                        if let Some(parent) = path.parent() {
                            self.open_project(parent.to_path_buf());
                        }
                    }
                    self.open_path(&path);
                }
                editor::scroll_to_y(0.0)
            }
            Message::FilePicked(_) => Task::none(),
            Message::CreateSample => {
                self.menu_open = None;
                if let Some(samples) = locate_samples_dir() {
                    self.open_project(samples);
                    self.log("Opened bundled sample project (samples/).\n");
                    editor::scroll_to_y(0.0)
                } else {
                    Task::perform(
                        pick_folder("Choose parent folder for sample project"),
                        Message::SampleFolderPicked,
                    )
                }
            }
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
                editor::scroll_to_y(0.0)
            }
            Message::SampleFolderPicked(_) => Task::none(),
            Message::CloseProject => {
                self.menu_open = None;
                self.sync_editor_to_active();
                self.project = None;
                self.tree = None;
                self.expanded.clear();
                self.open.clear();
                self.active = None;
                self.status = "Closed folder".into();
                self.reload_editor()
            }
            Message::RefreshExplorer => {
                if let Some(project) = self.project.as_ref() {
                    self.tree = Some(project.refresh_tree());
                }
                Task::none()
            }
            Message::OpenFile(path) => {
                self.open_path(&path);
                editor::scroll_to_y(0.0)
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
                self.reload_editor()
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
                self.reload_editor()
            }
            Message::EditorScrolled { offset_y, view_h } => {
                self.editor_scroll_y = offset_y.max(0.0);
                if view_h > 1.0 {
                    self.editor_view_h = view_h;
                }
                Task::none()
            }
            Message::EditorPage(direction) => self.page_cursor(direction),
            Message::EditorAction(action) => {
                if let text_editor::Action::Scroll { lines } = action {
                    return editor::scroll_by_lines(lines);
                }
                if action.is_edit() {
                    if let Some(i) = self.active {
                        if let Some(file) = self.open.get_mut(i) {
                            file.dirty = true;
                        }
                    }
                }
                let follow = matches!(
                    action,
                    text_editor::Action::Move(_)
                        | text_editor::Action::Select(_)
                        | text_editor::Action::Click(_)
                        | text_editor::Action::Edit(_)
                );
                self.editor_content.perform(action);
                self.sync_editor_to_active();
                if follow {
                    self.ensure_cursor_visible()
                } else {
                    Task::none()
                }
            }
            Message::Save => {
                self.menu_open = None;
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
                self.menu_open = None;
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
            Message::ShowNewFile => {
                self.menu_open = None;
                self.dialog = Some(Dialog::NewFile {
                    name: "untitled.v".into(),
                });
                Task::none()
            }
            Message::ShowNewFolder => {
                self.menu_open = None;
                self.dialog = Some(Dialog::NewFolder {
                    name: "new_folder".into(),
                });
                Task::none()
            }
            Message::ShowFind => {
                self.menu_open = None;
                self.dialog = Some(Dialog::Find {
                    query: self.search_query.clone(),
                });
                Task::none()
            }
            Message::ShowAbout => {
                self.menu_open = None;
                self.dialog = Some(Dialog::About);
                Task::none()
            }
            Message::DialogInput(value) => {
                match &mut self.dialog {
                    Some(Dialog::NewFile { name }) => *name = value,
                    Some(Dialog::NewFolder { name }) => *name = value,
                    Some(Dialog::Find { query }) => *query = value,
                    _ => {}
                }
                Task::none()
            }
            Message::DialogConfirm => {
                let dialog = self.dialog.take();
                if let Some(dialog) = dialog {
                    match dialog {
                        Dialog::NewFile { name } => {
                            self.create_file(&name);
                            return editor::scroll_to_y(0.0);
                        }
                        Dialog::NewFolder { name } => self.create_folder(&name),
                        Dialog::Find { query } => {
                            self.search_query = query;
                            self.find_next();
                            return self.ensure_cursor_visible();
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
            Message::ToggleBottomPanel => {
                self.bottom_visible = !self.bottom_visible;
                Task::none()
            }
            Message::ClearBottom => {
                match self.bottom {
                    BottomTab::Console => self.console.clear(),
                    BottomTab::Problems => self.problems.clear(),
                }
                Task::none()
            }
            Message::RunSim => self.start_simulation(),
            Message::SimFinished(result) => {
                self.sim_running = false;
                self.bottom = BottomTab::Console;
                self.bottom_visible = true;
                if !result.log.is_empty() {
                    if !self.console.ends_with('\n') && !self.console.is_empty() {
                        self.console.push('\n');
                    }
                    self.console.push_str(&result.log);
                    if !result.log.ends_with('\n') {
                        self.console.push('\n');
                    }
                }
                if !result.ok {
                    self.problems
                        .push("xezim simulation failed. See OUTPUT.".into());
                    self.status = "Simulation failed".into();
                } else if let Some(vcd) = result.vcd {
                    self.status = format!("Wrote {}", vcd.display());
                    self.refresh_tree();
                } else {
                    self.status = "Simulation finished (no VCD)".into();
                    self.problems.push(
                        "Simulation finished but no .vcd was produced. Add $dumpfile / $dumpvars to the testbench."
                            .into(),
                    );
                }
                Task::none()
            }
        }
    }

    fn start_simulation(&mut self) -> Task<Message> {
        self.menu_open = None;
        if self.sim_running {
            return Task::none();
        }
        if self.dialog.is_some() {
            return Task::none();
        }
        self.sync_editor_to_active();
        let mut errors = Vec::new();
        for f in &mut self.open {
            if f.dirty {
                if let Err(e) = save_file(f) {
                    errors.push(e);
                }
            }
        }
        if !errors.is_empty() {
            for e in errors {
                self.problems.push(e.clone());
                self.log(&format!("ERROR: {e}\n"));
            }
            self.log_err("Save failed; simulation not started.");
            return Task::none();
        }

        let Some(root) = self.project_root() else {
            self.log_err("Open a folder first (File → Open Folder), then click Run.");
            return Task::none();
        };
        let active = self.active_path().cloned();
        match sim::prepare_job(&root, active.as_deref(), &self.open) {
            Ok(job) => {
                self.sim_running = true;
                self.bottom = BottomTab::Console;
                self.bottom_visible = true;
                self.status = "Running xezim…".into();
                self.log("Simulating with xezim (--wave) to generate a VCD…\n");
                self.log(&format!("{}\n", job.command_preview()));
                Task::perform(sim::run_job_async(job), Message::SimFinished)
            }
            Err(e) => {
                self.log_err(&e);
                Task::none()
            }
        }
    }

    fn refresh_tree(&mut self) {
        if let Some(project) = self.project.as_ref() {
            self.tree = Some(project.refresh_tree());
        }
    }

    fn open_folder_path(&mut self, path: PathBuf) {
        let folder = if path.is_dir() {
            path
        } else if let Some(parent) = path.parent() {
            self.log(&format!(
                "Selected file {}; opening parent folder {}.\n",
                path.display(),
                parent.display()
            ));
            parent.to_path_buf()
        } else {
            self.log_err(&format!("Not a folder: {}", path.display()));
            return;
        };

        if !folder.is_dir() {
            self.log_err(&format!("Not a folder: {}", folder.display()));
            return;
        }

        self.open_project(folder);
    }

    fn open_project(&mut self, root: PathBuf) {
        let project = IdeProject::new(root);
        self.log(&format!("Opened folder: {}\n", project.root.display()));
        let tree = project.build_tree();
        self.expanded.clear();
        self.expanded.insert(project.root.clone());
        let mut dirs = Vec::new();
        collect_dir_paths(&tree, &mut dirs);
        self.expanded.extend(dirs);
        self.tree = Some(tree);
        self.project = Some(project);
        self.open.clear();
        self.active = None;
        self.status = format!(
            "Opened folder: {}",
            self.project.as_ref().unwrap().root.display()
        );
        self.load_active_into_editor();

        if let Some(root) = self.project.as_ref().map(|p| p.root.clone()) {
            if let Some(first) = find_first_verilog(&root) {
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
        self.editor_scroll_y = 0.0;
        if let Some(i) = self.active {
            if let Some(file) = self.open.get(i) {
                let cursor = file.cursor;
                let content = file.content.clone();
                self.editor_content = text_editor::Content::with_text(&content);
                let (line, _) = cursor_line_col(&content, cursor);
                let y = (line.saturating_sub(1)) as f32 * EDITOR_LINE_HEIGHT
                    + EDITOR_LINE_HEIGHT * 0.4;
                self.editor_content
                    .perform(text_editor::Action::Click(Point::new(2.0, y)));
            }
        } else {
            self.editor_content = text_editor::Content::new();
        }
    }

    fn reload_editor(&mut self) -> Task<Message> {
        self.load_active_into_editor();
        let (line, _) = self.editor_content.cursor_position();
        let y = (editor::line_top(line) - self.editor_view_h * 0.25).max(0.0);
        editor::scroll_to_y(y)
    }

    fn ensure_cursor_visible(&self) -> Task<Message> {
        let view_h = self.editor_view_h;
        if view_h <= EDITOR_LINE_HEIGHT {
            return Task::none();
        }
        let (line, _) = self.editor_content.cursor_position();
        let line_top = editor::line_top(line);
        let line_bot = line_top + EDITOR_LINE_HEIGHT;
        let view_top = self.editor_scroll_y;
        let view_bot = view_top + view_h;
        let margin = EDITOR_LINE_HEIGHT;
        let y = if line_top < view_top + margin {
            (line_top - margin).max(0.0)
        } else if line_bot + margin > view_bot {
            (line_bot + margin - view_h).max(0.0)
        } else {
            return Task::none();
        };
        editor::scroll_to_y(y)
    }

    fn page_cursor(&mut self, direction: i32) -> Task<Message> {
        let (line, _) = self.editor_content.cursor_position();
        let last = self.editor_content.line_count().saturating_sub(1);
        let page = ((self.editor_view_h / EDITOR_LINE_HEIGHT).floor() as i32 - 2).max(1);
        let target = (line as i32 + direction * page).clamp(0, last as i32) as usize;
        let y = target as f32 * EDITOR_LINE_HEIGHT + EDITOR_LINE_HEIGHT * 0.4;
        self.editor_content
            .perform(text_editor::Action::Click(Point::new(2.0, y)));
        self.sync_editor_to_active();
        self.ensure_cursor_visible()
    }

    fn log(&mut self, msg: &str) {
        self.console.push_str(msg);
    }

    fn log_err(&mut self, msg: &str) {
        self.console.push_str(&format!("ERROR: {msg}\n"));
        self.status = msg.to_string();
        self.problems.push(msg.to_string());
    }

    fn project_root(&self) -> Option<PathBuf> {
        self.project.as_ref().map(|p| p.root.clone())
    }

    fn create_file(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let Some(root) = self.project_root() else {
            self.log_err("Open a folder first (File → Open Folder).");
            return;
        };
        let path = root.join(name);
        if path.exists() {
            self.log_err(&format!("Already exists: {}", path.display()));
            return;
        }
        let stem = name
            .trim_end_matches(".sv")
            .trim_end_matches(".v")
            .trim_end_matches(".SV")
            .trim_end_matches(".V");
        let body = if templates::is_testbench_filename(name) {
            templates::testbench_template(stem)
        } else if is_verilog_name(name) {
            templates::module_template(stem)
        } else {
            String::new()
        };
        if let Err(e) = std::fs::write(&path, body) {
            self.log_err(&e.to_string());
            return;
        }
        self.refresh_tree();
        self.open_path(&path);
        self.log(&format!("Created file {}\n", path.display()));
    }

    fn create_folder(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let Some(root) = self.project_root() else {
            self.log_err("Open a folder first (File → Open Folder).");
            return;
        };
        let path = root.join(name);
        if let Err(e) = std::fs::create_dir_all(&path) {
            self.log_err(&e.to_string());
            return;
        }
        self.expanded.insert(path.clone());
        self.refresh_tree();
        self.log(&format!("Created folder {}\n", path.display()));
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

    fn active_path(&self) -> Option<&PathBuf> {
        self.active
            .and_then(|i| self.open.get(i))
            .map(|f| &f.path)
    }

    fn view(&self) -> Element<'_, Message> {
        let main = row![
            self.view_activity_bar(),
            self.view_sidebar(),
            column![
                self.view_editor_column(),
                if self.bottom_visible {
                    Element::from(self.view_bottom_panel())
                } else {
                    Space::new(Length::Fill, Length::Fixed(0.0)).into()
                },
            ]
            .width(Fill)
            .spacing(0),
        ]
        .height(Fill)
        .spacing(0);

        let content = column![
            container(main)
                .width(Fill)
                .height(Fill)
                .style(|_| panel_style(BG_EDITOR)),
            self.view_status_bar(),
        ]
        .height(Fill)
        .spacing(0);

        let content_area: Element<'_, Message> = if let Some(menu) = self.menu_open {
            stack![
                content,
                mouse_area(Space::new(Fill, Fill)).on_press(Message::MenuClose),
                container(
                    row![
                        Space::new(Length::Fixed(menu_dropdown_left(menu)), Length::Shrink),
                        self.view_menu_dropdown(menu),
                    ]
                    .align_y(Alignment::Start),
                )
                .width(Fill)
                .height(Length::Shrink)
                .align_x(Alignment::Start)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: 2.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
            ]
            .height(Fill)
            .into()
        } else {
            content.into()
        };

        column![
            self.view_menu_bar(),
            content_area,
        ]
        .height(Fill)
        .spacing(0)
        .into()
    }

    fn view_menu_dropdown(&self, menu: TopMenu) -> Element<'_, Message> {
        match menu {
            TopMenu::File => menu_dropdown(column![
                menu_item("Open Folder…", Message::OpenProject),
                menu_item("Open File…", Message::OpenFilePicker),
                menu_item("New File…", Message::ShowNewFile),
                menu_item("New Folder…", Message::ShowNewFolder),
                menu_item("Save", Message::Save),
                menu_item("Save All", Message::SaveAll),
                menu_item("Close Folder", Message::CloseProject),
                menu_item("Open Sample Folder", Message::CreateSample),
            ]),
            TopMenu::Edit => menu_dropdown(column![
                menu_item("Find…", Message::ShowFind),
                menu_item("Refresh Explorer", Message::RefreshExplorer),
            ]),
            TopMenu::Run => menu_dropdown(column![
                menu_item("Run Simulation (F5)", Message::RunSim),
            ]),
            TopMenu::Help => {
                menu_dropdown(column![menu_item("About Verilog IDE", Message::ShowAbout)])
            }
        }
    }

    fn view_menu_bar(&self) -> Element<'_, Message> {
        let file_active = self.menu_open == Some(TopMenu::File);
        let edit_active = self.menu_open == Some(TopMenu::Edit);
        let run_active = self.menu_open == Some(TopMenu::Run);
        let help_active = self.menu_open == Some(TopMenu::Help);

        container(
            row![
                menu_label("File", file_active, Message::MenuToggle(TopMenu::File)),
                menu_label("Edit", edit_active, Message::MenuToggle(TopMenu::Edit)),
                menu_label("Run", run_active, Message::MenuToggle(TopMenu::Run)),
                menu_label("Help", help_active, Message::MenuToggle(TopMenu::Help)),
                horizontal_space(),
                run_toolbar_button(self.sim_running),
                text("Verilog IDE").size(12).color(FG_MUTED),
            ]
            .height(Fill)
            .spacing(2)
            .padding([0, 8])
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(MENU_HEIGHT))
        .style(|_| panel_style(BG_MENU))
        .into()
    }

    fn view_activity_bar(&self) -> Element<'_, Message> {
        container(
            column![
                activity_icon("EX", true),
                Space::new(Length::Fill, Length::Shrink),
            ]
            .height(Fill)
            .padding(4),
        )
        .width(Length::Fixed(ACTIVITY_WIDTH))
        .height(Fill)
        .style(|_| panel_style(BG_ACTIVITY))
        .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let title = row![
            text("EXPLORER").size(11).color(FG_MUTED),
            horizontal_space(),
            sidebar_action("+", "New File", Message::ShowNewFile),
            sidebar_action("Fld", "New Folder", Message::ShowNewFolder),
            sidebar_action("Ref", "Refresh", Message::RefreshExplorer),
        ]
        .align_y(Alignment::Center)
        .padding([6, 8]);

        let project_header: Element<'_, Message> = if let Some(p) = self.project.as_ref() {
            row![
                text("v").size(12).color(FG_MUTED),
                text(p.name.as_str()).size(13).color(FG_TEXT),
            ]
            .spacing(4)
            .padding([2, 8])
            .into()
        } else {
            Space::new(Length::Fill, Length::Fixed(0.0)).into()
        };

        let tree_body: Element<'_, Message> = if let Some(tree) = &self.tree {
            scrollable(
                column(self.render_tree_nodes(tree, 0))
                    .spacing(0)
                    .padding([0, 4]),
            )
            .height(Fill)
            .into()
        } else {
            scrollable(
                column![
                    text("No folder opened").size(13).color(FG_MUTED),
                    Space::new(Length::Fill, Length::Fixed(8.0)),
                    sidebar_link("Open Folder…", Message::OpenProject),
                    sidebar_link("Open File…", Message::OpenFilePicker),
                    sidebar_link("Open Sample Folder", Message::CreateSample),
                ]
                .padding(12)
                .spacing(6),
            )
            .height(Fill)
            .into()
        };

        container(
            column![title, horizontal_rule(1), project_header, tree_body]
                .spacing(0)
                .height(Fill),
        )
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Fill)
        .style(|_| panel_style(BG_SIDEBAR))
        .into()
    }

    fn render_tree_nodes(
        &self,
        node: &TreeNode,
        depth: usize,
    ) -> Vec<Element<'static, Message>> {
        let mut items = Vec::new();
        let indent = depth as f32 * 16.0;
        let is_active = self.active_path().is_some_and(|p| p == &node.path);

        if depth > 0 {
            if node.is_dir {
                let expanded = self.expanded.contains(&node.path);
                let chevron = if expanded { "v" } else { ">" };
                items.push(tree_row(
                    indent,
                    format!("{chevron} [D] {}", node.name),
                    is_active,
                    Message::ToggleDir(node.path.clone()),
                ));
            } else {
                items.push(tree_row(
                    indent,
                    format!("    {}", node.name),
                    is_active,
                    Message::OpenFile(node.path.clone()),
                ));
            }
        }

        if node.is_dir && (depth == 0 || self.expanded.contains(&node.path)) {
            for child in &node.children {
                items.extend(self.render_tree_nodes(child, depth + 1));
            }
        }

        items
    }

    fn view_editor_column(&self) -> Element<'_, Message> {
        let editor = column![
            self.view_tab_bar(),
            container(self.view_editor_body())
                .width(Fill)
                .height(Fill)
                .style(|_| panel_style(BG_EDITOR)),
        ]
        .spacing(0)
        .height(Fill);

        if self.dialog.is_some() {
            column![editor, self.view_dialog_bar()].spacing(0).height(Fill).into()
        } else {
            editor.into()
        }
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        if self.open.is_empty() {
            return container(Space::new(Length::Fill, Length::Fixed(TAB_HEIGHT - 4.0)))
                .width(Fill)
                .style(|_| panel_style(BG_TABS))
                .into();
        }

        let tabs = scrollable(
            row(
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
                    let selected = self.active == Some(i);
                    row![
                        tab_button(title, selected, Message::SelectTab(i)),
                        button(text("×"))
                            .on_press(Message::CloseTab(i))
                            .padding([2, 6])
                            .style(|_, _| tab_close_style()),
                    ]
                    .spacing(0)
                    .into()
                }),
            )
            .spacing(0)
            .padding([0, 4]),
        )
        .direction(iced::widget::scrollable::Direction::Horizontal(
            Default::default(),
        ));

        container(tabs)
            .width(Fill)
            .height(Length::Fixed(TAB_HEIGHT))
            .style(|_| panel_style(BG_TABS))
            .into()
    }

    fn view_editor_body(&self) -> Element<'_, Message> {
        if self.open.is_empty() {
            return container(
                column![
                    text("Verilog IDE").size(32).color(FG_TEXT),
                    text("Open a folder, then ▶ Run (F5) to simulate with xezim and write a .vcd.")
                        .size(14)
                        .color(FG_MUTED),
                    Space::new(Length::Shrink, Length::Fixed(16.0)),
                    row![
                        welcome_button("Open Folder", Message::OpenProject),
                        welcome_button("Open File", Message::OpenFilePicker),
                        welcome_button("Open Sample", Message::CreateSample),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    Space::new(Length::Shrink, Length::Fixed(24.0)),
                    text("File → Open Folder    Run → Run Simulation (F5)")
                        .size(12)
                        .color(FG_MUTED),
                ]
                .align_x(Alignment::Center)
                .spacing(8),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into();
        }

        let line_count = self.editor_content.line_count().max(1);
        let current_line = self.editor_content.cursor_position().0 + 1;
        let content_h = editor::content_height(line_count, self.editor_view_h);

        let editor = text_editor(&self.editor_content)
            .font(Font::MONOSPACE)
            .size(EDITOR_FONT_SIZE)
            .line_height(editor::line_height())
            .padding(EDITOR_PADDING)
            .wrapping(Wrapping::None)
            .height(Length::Fixed(content_h))
            .on_action(Message::EditorAction)
            .key_binding(|press| match &press.key {
                Key::Named(keyboard::key::Named::PageDown) => {
                    Some(text_editor::Binding::Custom(Message::EditorPage(1)))
                }
                Key::Named(keyboard::key::Named::PageUp) => {
                    Some(text_editor::Binding::Custom(Message::EditorPage(-1)))
                }
                Key::Named(keyboard::key::Named::F5) => {
                    Some(text_editor::Binding::Custom(Message::RunSim))
                }
                _ => text_editor::Binding::from_key_press(press),
            })
            .highlight_with::<VerilogHighlighter>(
                VerilogHighlightSettings {
                    enabled: self
                        .active_path()
                        .map(|path| verilog_highlighter::syntax_enabled_for_path(path.as_path()))
                        .unwrap_or(true),
                },
                verilog_highlighter::format_highlight,
            );

        let body = row![
            editor::line_gutter(line_count, current_line, self.editor_view_h),
            container(editor)
                .width(Fill)
                .height(Length::Fixed(content_h)),
        ]
        .spacing(0)
        .width(Fill)
        .height(Length::Fixed(content_h));

        container(
            scrollable(body)
                .id(editor::scroll_id())
                .width(Fill)
                .height(Fill)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new(),
                ))
                .on_scroll(|viewport| Message::EditorScrolled {
                    offset_y: viewport.absolute_offset().y,
                    view_h: viewport.bounds().height,
                }),
        )
        .width(Fill)
        .height(Fill)
        .style(|_| panel_style(BG_EDITOR))
        .into()
    }

    fn view_bottom_panel(&self) -> Element<'_, Message> {
        let problems_label = format!("PROBLEMS ({})", self.problems.len());
        let panel_tabs = row![
            panel_tab(
                "OUTPUT".into(),
                self.bottom == BottomTab::Console,
                Message::BottomTabSelected(BottomTab::Console),
            ),
            panel_tab(
                problems_label,
                self.bottom == BottomTab::Problems,
                Message::BottomTabSelected(BottomTab::Problems),
            ),
            horizontal_space(),
            button(text("^")).on_press(Message::ToggleBottomPanel),
            button(text("Clear")).on_press(Message::ClearBottom),
        ]
        .spacing(4)
        .padding([4, 8])
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match self.bottom {
            BottomTab::Console => scrollable(
                text(self.console.as_str())
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(FG_TEXT),
            )
            .height(Fill)
            .into(),
            BottomTab::Problems => {
                if self.problems.is_empty() {
                    text("No problems detected.").size(12).color(FG_MUTED).into()
                } else {
                    scrollable(
                        column(
                            self.problems
                                .iter()
                                .enumerate()
                                .map(|(i, p)| {
                                    text(format!("{}. {p}", i + 1))
                                        .size(12)
                                        .color(FG_TEXT)
                                        .into()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .spacing(2)
                        .padding(8),
                    )
                    .height(Fill)
                    .into()
                }
            }
        };

        container(column![panel_tabs, horizontal_rule(1), body].spacing(0).height(Fill))
            .width(Fill)
            .height(Length::Fixed(BOTTOM_HEIGHT))
            .style(|_| panel_style(BG_SIDEBAR))
            .into()
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        let branch = self
            .project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "No Folder".into());

        let detail = if let Some(i) = self.active {
            if let Some(f) = self.open.get(i) {
                let (line, col) = cursor_line_col(&f.content, f.cursor);
                let dirty = if f.dirty { " *" } else { "" };
                format!(
                    "Ln {line}, Col {col}{dirty}   {}",
                    f.path.display()
                )
            } else {
                self.status.clone()
            }
        } else {
            self.status.clone()
        };

        container(
            row![
                text(branch).size(12),
                horizontal_space(),
                text(detail).size(12),
            ]
            .padding([2, 10])
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(STATUS_HEIGHT))
        .style(|_| panel_style(BG_STATUS))
        .into()
    }

    fn view_dialog_bar(&self) -> Element<'_, Message> {
        let Some(dialog) = &self.dialog else {
            return Space::new(Length::Shrink, Length::Fixed(0.0)).into();
        };

        let (title, input, show_input) = match dialog {
            Dialog::NewFile { name } => ("New File", name.clone(), true),
            Dialog::NewFolder { name } => ("New Folder", name.clone(), true),
            Dialog::Find { query } => ("Find", query.clone(), true),
            Dialog::About => ("About", String::new(), false),
        };

        let body: Element<'_, Message> = if show_input {
            column![
                text(title).size(14).color(FG_TEXT),
                text_input("Name…", &input).on_input(Message::DialogInput),
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
                text("About Verilog IDE").size(14),
                text("Desktop IDE for Verilog HDL and testbenches."),
                text("Run ▶ uses the bundled xezim simulator to write a .vcd waveform."),
                button("Close").on_press(Message::DialogCancel),
            ]
            .spacing(6)
            .into()
        };

        container(body)
            .padding(12)
            .width(Fill)
            .style(|_| dialog_style())
            .into()
    }
}

fn is_verilog_name(name: &str) -> bool {
    name.ends_with(".v") || name.ends_with(".sv")
}

fn panel_style(bg: Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn dialog_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.2, 0.22))),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

fn tab_close_style() -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: FG_MUTED,
        ..Default::default()
    }
}

fn menu_label(label: &'static str, active: bool, msg: Message) -> Element<'static, Message> {
    button(
        text(label)
            .size(13)
            .color(FG_TEXT),
    )
    .on_press(msg)
    .padding([6, 12])
    .style(move |_, status| {
        let hovered = matches!(status, iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed);
        iced::widget::button::Style {
            background: if active {
                Some(iced::Background::Color(Color::from_rgb(0.30, 0.30, 0.32)))
            } else if hovered {
                Some(iced::Background::Color(Color::from_rgb(0.26, 0.26, 0.28)))
            } else {
                None
            },
            text_color: FG_TEXT,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn menu_item(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(text(label).size(13))
        .on_press(msg)
        .width(Fill)
        .padding([6, 12])
        .style(|_, _| iced::widget::button::Style {
            background: None,
            text_color: FG_TEXT,
            ..Default::default()
        })
        .into()
}

fn menu_dropdown_left(menu: TopMenu) -> f32 {
    match menu {
        TopMenu::File => 8.0,
        TopMenu::Edit => 52.0,
        TopMenu::Run => 96.0,
        TopMenu::Help => 140.0,
    }
}

fn run_toolbar_button(running: bool) -> Element<'static, Message> {
    let label = if running { "Running…" } else { "▶ Run" };
    let mut btn = button(text(label).size(13)).padding([4, 14]).style(move |_, status| {
        let hovered = matches!(
            status,
            iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
        );
        iced::widget::button::Style {
            background: Some(iced::Background::Color(if running {
                Color::from_rgb(0.18, 0.42, 0.22)
            } else if hovered {
                Color::from_rgb(0.16, 0.58, 0.28)
            } else {
                Color::from_rgb(0.12, 0.52, 0.24)
            })),
            text_color: Color::WHITE,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });
    if !running {
        btn = btn.on_press(Message::RunSim);
    }
    btn.into()
}

fn menu_dropdown(items: Column<'_, Message>) -> Element<'_, Message> {
    container(items.padding(4))
        .width(Length::Fixed(240.0))
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.24, 0.24, 0.26))),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        })
        .into()
}

fn sidebar_action(icon: &'static str, _tip: &str, msg: Message) -> Element<'static, Message> {
    button(text(icon).size(12))
        .on_press(msg)
        .padding([2, 4])
        .style(|_, _| iced::widget::button::Style {
            background: None,
            text_color: FG_MUTED,
            ..Default::default()
        })
        .into()
}

fn sidebar_link(label: &str, msg: Message) -> Element<'_, Message> {
    button(text(label).size(13))
        .on_press(msg)
        .padding([4, 0])
        .style(|_, _| iced::widget::button::Style {
            background: None,
            text_color: Color::from_rgb(0.4, 0.65, 1.0),
            ..Default::default()
        })
        .into()
}

fn activity_icon(label: &str, _active: bool) -> Element<'_, Message> {
    container(text(label).size(16).color(FG_TEXT))
        .padding(8)
        .center_x(Fill)
        .into()
}

fn tree_row(indent: f32, label: String, active: bool, msg: Message) -> Element<'static, Message> {
    row![
        Space::new(Length::Fixed(indent), Length::Shrink),
        button(text(label).size(12).color(if active {
            Color::WHITE
        } else {
            FG_TEXT
        }))
        .on_press(msg)
        .width(Fill)
        .padding([3, 4])
        .style(move |_, _| iced::widget::button::Style {
            background: if active {
                Some(iced::Background::Color(Color::from_rgb(0.09, 0.38, 0.65)))
            } else {
                None
            },
            text_color: FG_TEXT,
            ..Default::default()
        }),
    ]
    .width(Fill)
    .into()
}

fn tab_button(title: String, selected: bool, msg: Message) -> Element<'static, Message> {
    button(text(title).size(12))
        .on_press(msg)
        .padding([8, 14])
        .style(move |_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(if selected {
                BG_TAB_ACTIVE
            } else {
                BG_TABS
            })),
            text_color: if selected { FG_TEXT } else { FG_MUTED },
            ..Default::default()
        })
        .into()
}

fn welcome_button(label: &str, msg: Message) -> Element<'_, Message> {
    button(text(label).size(13))
        .on_press(msg)
        .padding([8, 16])
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.09, 0.38, 0.65))),
            text_color: Color::WHITE,
            ..Default::default()
        })
        .into()
}

fn panel_tab(label: String, selected: bool, msg: Message) -> Element<'static, Message> {
    button(text(label).size(11))
        .on_press(msg)
        .padding([4, 8])
        .style(move |_, _| iced::widget::button::Style {
            background: if selected {
                Some(iced::Background::Color(BG_EDITOR))
            } else {
                None
            },
            text_color: if selected { FG_TEXT } else { FG_MUTED },
            ..Default::default()
        })
        .into()
}

async fn pick_folder(title: &str) -> Result<Option<PathBuf>, DialogError> {
    let title = title.to_owned();

    #[cfg(target_os = "linux")]
    if zenity_available() {
        return tokio::task::spawn_blocking(move || pick_folder_zenity(&title))
            .await
            .map_err(|_| DialogError::Cancelled)?;
    }

    let picked = rfd::AsyncFileDialog::new()
        .set_title(&title)
        .pick_folder()
        .await
        .map(|handle| handle.path().to_path_buf());
    Ok(picked)
}

async fn pick_file(title: &str) -> Result<Option<PathBuf>, DialogError> {
    let title = title.to_owned();

    #[cfg(target_os = "linux")]
    if zenity_available() {
        return tokio::task::spawn_blocking(move || pick_file_zenity(&title))
            .await
            .map_err(|_| DialogError::Cancelled)?;
    }

    let picked = rfd::AsyncFileDialog::new()
        .set_title(&title)
        .add_filter("Verilog / text", &["v", "sv", "vh", "svh", "txt", "md"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf());
    Ok(picked)
}

#[cfg(target_os = "linux")]
fn zenity_available() -> bool {
    std::process::Command::new("zenity")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(target_os = "linux"))]
fn zenity_available() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn pick_folder_zenity(title: &str) -> Result<Option<PathBuf>, DialogError> {
    let output = std::process::Command::new("zenity")
        .args(["--file-selection", "--directory", "--title", title])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| DialogError::Cancelled)?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!path.is_empty()).then(|| PathBuf::from(path)))
    } else if output.status.code() == Some(1) {
        Ok(None)
    } else {
        Err(DialogError::Cancelled)
    }
}

#[cfg(target_os = "linux")]
fn pick_file_zenity(title: &str) -> Result<Option<PathBuf>, DialogError> {
    let output = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--title",
            title,
            "--file-filter",
            "Verilog | *.v *.sv *.vh *.svh",
            "--file-filter",
            "Text | *.txt *.md",
            "--file-filter",
            "All | *",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| DialogError::Cancelled)?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!path.is_empty()).then(|| PathBuf::from(path)))
    } else if output.status.code() == Some(1) {
        Ok(None)
    } else {
        Err(DialogError::Cancelled)
    }
}
