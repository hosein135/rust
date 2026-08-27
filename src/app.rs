//! Main IDE application state and UI.

use crate::editor::{self, cursor_line_col};
use crate::project::{
    is_testbench_path, load_file, save_file, IdeProject, OpenFile, TreeNode,
};
use crate::templates::{self, counter_example};
use crate::theme;
use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers, RichText};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Console,
    Problems,
}

#[derive(Default)]
struct Dialogs {
    new_module: bool,
    new_tb: bool,
    about: bool,
    module_name: String,
    tb_dut: String,
}

pub struct VerilogIde {
    project: Option<IdeProject>,
    tree: Option<TreeNode>,
    open: Vec<OpenFile>,
    active: Option<usize>,
    console: String,
    problems: Vec<String>,
    bottom: BottomTab,
    status: String,
    font_size: f32,
    dialogs: Dialogs,
    left_width: f32,
    bottom_height: f32,
    search: String,
    search_open: bool,
}

impl VerilogIde {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let mut ide = Self {
            project: None,
            tree: None,
            open: Vec::new(),
            active: None,
            console: String::from(
                "Verilog IDE ready.\nOpen a folder (File -> Open Project) or create a sample project.\n",
            ),
            problems: Vec::new(),
            bottom: BottomTab::Console,
            status: "Ready".into(),
            font_size: 14.0,
            dialogs: Dialogs::default(),
            left_width: 240.0,
            bottom_height: 180.0,
            search: String::new(),
            search_open: false,
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

        ide
    }

    fn refresh_tree(&mut self) {
        self.tree = self.project.as_ref().map(|p| p.build_tree());
    }

    fn open_project(&mut self, root: PathBuf) {
        let project = IdeProject::new(root);
        self.log(&format!("Opened project: {}\n", project.root.display()));
        self.project = Some(project);
        self.open.clear();
        self.active = None;
        self.refresh_tree();
        self.status = "Project opened".into();

        if let Some(p) = self.project.as_ref() {
            if let Some(first) = p.list_verilog_files().into_iter().next() {
                self.open_path(&first);
            }
        }
    }

    fn open_path(&mut self, path: &Path) {
        if let Some(idx) = self.open.iter().position(|f| f.path == path) {
            self.active = Some(idx);
            return;
        }
        match load_file(path) {
            Ok(file) => {
                self.open.push(file);
                self.active = Some(self.open.len() - 1);
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => {
                self.log_err(&e);
                self.problems.push(e);
            }
        }
    }

    fn save_active(&mut self) {
        let Some(i) = self.active else { return };
        match save_file(&mut self.open[i]) {
            Ok(()) => {
                self.status = format!("Saved {}", self.open[i].path.display());
                self.log(&format!("Saved {}\n", self.open[i].path.display()));
            }
            Err(e) => {
                self.log_err(&e);
                self.problems.push(e);
            }
        }
    }

    fn save_all(&mut self) {
        for f in &mut self.open {
            if f.dirty {
                if let Err(e) = save_file(f) {
                    self.problems.push(e.clone());
                    self.console.push_str(&format!("ERROR: {e}\n"));
                }
            }
        }
        self.status = "Saved all".into();
    }

    fn close_tab(&mut self, idx: usize) {
        if self.open.get(idx).map(|f| f.dirty).unwrap_or(false) {
            let _ = save_file(&mut self.open[idx]);
        }
        self.open.remove(idx);
        self.active = if self.open.is_empty() {
            None
        } else {
            Some(idx.min(self.open.len() - 1))
        };
    }

    fn log(&mut self, msg: &str) {
        self.console.push_str(msg);
    }

    fn log_err(&mut self, msg: &str) {
        self.console.push_str(&format!("ERROR: {msg}\n"));
        self.status = msg.to_string();
    }

    fn create_sample_project(&mut self) {
        let dir = rfd::FileDialog::new()
            .set_title("Choose parent folder for sample project")
            .pick_folder();
        let Some(parent) = dir else { return };
        let root = parent.join("verilog-sample");
        if let Err(e) = std::fs::create_dir_all(&root) {
            self.log_err(&e.to_string());
            return;
        }
        let (n1, c1, n2, c2) = counter_example();
        let _ = std::fs::write(root.join(n1), c1);
        let _ = std::fs::write(root.join(n2), c2);
        self.open_project(root);
        self.log("Created sample counter + testbench project.\n");
    }

    fn new_module_file(&mut self) {
        let name = self.dialogs.module_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let Some(project) = self.project.as_ref() else {
            self.log_err("Open a project first.");
            return;
        };
        let path = project.root.join(format!("{name}.v"));
        let body = templates::module_template(&name);
        if let Err(e) = std::fs::write(&path, body) {
            self.log_err(&e.to_string());
            return;
        }
        self.dialogs.new_module = false;
        self.dialogs.module_name.clear();
        self.refresh_tree();
        self.open_path(&path);
        self.log(&format!("Created module {}\n", path.display()));
    }

    fn new_tb_file(&mut self) {
        let dut = self.dialogs.tb_dut.trim().to_string();
        if dut.is_empty() {
            return;
        }
        let Some(project) = self.project.as_ref() else {
            self.log_err("Open a project first.");
            return;
        };
        let path = project.root.join(format!("{dut}_tb.v"));
        let body = templates::testbench_template(&dut);
        if let Err(e) = std::fs::write(&path, body) {
            self.log_err(&e.to_string());
            return;
        }
        self.dialogs.new_tb = false;
        self.dialogs.tb_dut.clear();
        self.refresh_tree();
        self.open_path(&path);
        self.log(&format!("Created testbench {}\n", path.display()));
    }

    fn find_next(&mut self) {
        let Some(i) = self.active else { return };
        let needle = self.search.clone();
        if needle.is_empty() {
            return;
        }
        let start = self.open[i]
            .cursor
            .saturating_add(1)
            .min(self.open[i].content.len());
        let found = self.open[i].content[start..]
            .find(&needle)
            .map(|o| start + o)
            .or_else(|| self.open[i].content.find(&needle));
        if let Some(pos) = found {
            self.open[i].cursor = pos;
            self.status = format!("Found at {pos}");
        } else {
            self.status = "Not found".into();
        }
    }

    fn ui_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Project...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.open_project(folder);
                        }
                        ui.close_menu();
                    }
                    if ui.button("New Sample Project...").clicked() {
                        self.create_sample_project();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("New Module...").clicked() {
                        self.dialogs.new_module = true;
                        ui.close_menu();
                    }
                    if ui.button("New Testbench...").clicked() {
                        self.dialogs.new_tb = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(self.active.is_some(), egui::Button::new("Save"))
                        .clicked()
                    {
                        self.save_active();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(!self.open.is_empty(), egui::Button::new("Save All"))
                        .clicked()
                    {
                        self.save_all();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Find...").clicked() {
                        self.search_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Increase Font").clicked() {
                        self.font_size = (self.font_size + 1.0).min(28.0);
                    }
                    if ui.button("Decrease Font").clicked() {
                        self.font_size = (self.font_size - 1.0).max(10.0);
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.dialogs.about = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(p) = &self.project {
                        ui.label(RichText::new(&p.name).color(theme::ACCENT).strong());
                    }
                });
            });
        });
    }

    fn ui_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new(" Open ").color(theme::TEXT)))
                    .on_hover_text("Open project folder")
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.open_project(folder);
                    }
                }
                if ui.button(" Save ").clicked() {
                    self.save_active();
                }
                ui.separator();
                if ui.button(" + Module ").clicked() {
                    self.dialogs.new_module = true;
                }
                if ui.button(" + Testbench ").clicked() {
                    self.dialogs.new_tb = true;
                }
                if ui.button(" Sample ").clicked() {
                    self.create_sample_project();
                }
            });
        });
    }

    fn ui_status(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).small().color(theme::TEXT_DIM));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(i) = self.active {
                            if let Some(f) = self.open.get(i) {
                                let (line, col) = cursor_line_col(&f.content, f.cursor);
                                let dirty = if f.dirty { " *" } else { "" };
                                ui.label(
                                    RichText::new(format!(
                                        "Ln {line}, Col {col}{dirty}  |  {}",
                                        f.path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("?")
                                    ))
                                    .small()
                                    .color(theme::TEXT_DIM),
                                );
                            }
                        }
                    });
                });
            });
    }

    fn ui_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Explorer").size(14.0).color(theme::ACCENT));
        ui.separator();
        if self.project.is_none() {
            ui.label(RichText::new("No project open.").color(theme::TEXT_DIM));
            if ui.button("Open Project...").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.open_project(folder);
                }
            }
            if ui.button("Create Sample...").clicked() {
                self.create_sample_project();
            }
            return;
        }

        if ui.button("Refresh").clicked() {
            self.refresh_tree();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(tree) = self.tree.clone() {
                self.draw_tree(ui, &tree, 0);
            }
        });
    }

    fn draw_tree(&mut self, ui: &mut egui::Ui, node: &TreeNode, depth: usize) {
        let indent = 8.0 * depth as f32;
        if node.is_dir {
            egui::CollapsingHeader::new(
                RichText::new(format!("[DIR] {}", node.name)).color(theme::TEXT),
            )
            .default_open(depth < 2)
            .show(ui, |ui| {
                for child in &node.children {
                    self.draw_tree(ui, child, depth + 1);
                }
            });
        } else {
            let is_tb = is_testbench_path(&node.path);
            let tag = if is_tb { "[TB]" } else { "[V] " };
            let label = format!("{tag} {}", node.name);
            let selected = self
                .active
                .and_then(|i| self.open.get(i))
                .map(|f| f.path == node.path)
                .unwrap_or(false);
            ui.horizontal(|ui| {
                ui.add_space(indent);
                if ui
                    .selectable_label(
                        selected,
                        RichText::new(label).color(if is_tb {
                            theme::WARN
                        } else {
                            theme::TEXT
                        }),
                    )
                    .clicked()
                {
                    self.open_path(&node.path);
                }
            });
        }
    }

    fn ui_editor_area(&mut self, ui: &mut egui::Ui) {
        if self.open.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading(RichText::new("Verilog IDE").color(theme::ACCENT).size(28.0));
                ui.label(
                    RichText::new("Edit Verilog modules and testbenches.")
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(16.0);
                if ui.button("Open Project Folder").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.open_project(folder);
                    }
                }
                if ui.button("Create Sample Counter Project").clicked() {
                    self.create_sample_project();
                }
            });
            return;
        }

        ui.horizontal(|ui| {
            let mut close_idx = None;
            for (i, f) in self.open.iter().enumerate() {
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
                let resp = ui.selectable_label(selected, title);
                if resp.clicked() {
                    self.active = Some(i);
                }
                if resp.middle_clicked() {
                    close_idx = Some(i);
                }
                if ui.small_button("x").on_hover_text("Close").clicked() {
                    close_idx = Some(i);
                }
            }
            if let Some(i) = close_idx {
                self.close_tab(i);
            }
        });
        ui.separator();

        if let Some(i) = self.active {
            let font = self.font_size;
            let file = &mut self.open[i];
            let out = editor::show_editor(ui, &mut file.content, &mut file.cursor, font);
            if out.changed {
                file.dirty = true;
            }
        }
    }

    fn ui_bottom(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.bottom == BottomTab::Console, "Console")
                .clicked()
            {
                self.bottom = BottomTab::Console;
            }
            if ui
                .selectable_label(
                    self.bottom == BottomTab::Problems,
                    format!("Problems ({})", self.problems.len()),
                )
                .clicked()
            {
                self.bottom = BottomTab::Problems;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    match self.bottom {
                        BottomTab::Console => self.console.clear(),
                        BottomTab::Problems => self.problems.clear(),
                    }
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                match self.bottom {
                    BottomTab::Console => {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.console)
                                .font(egui::FontId::monospace(12.0))
                                .desired_width(f32::INFINITY)
                                .interactive(true),
                        );
                    }
                    BottomTab::Problems => {
                        if self.problems.is_empty() {
                            ui.label(RichText::new("No problems.").color(theme::OK));
                        } else {
                            for (i, p) in self.problems.iter().enumerate() {
                                ui.colored_label(theme::DANGER, format!("{}. {p}", i + 1));
                            }
                        }
                    }
                }
            });
    }

    fn ui_dialogs(&mut self, ctx: &egui::Context) {
        if self.dialogs.new_module {
            egui::Window::new("New Verilog Module")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Module name:");
                    ui.text_edit_singleline(&mut self.dialogs.module_name);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            self.new_module_file();
                        }
                        if ui.button("Cancel").clicked() {
                            self.dialogs.new_module = false;
                        }
                    });
                });
        }

        if self.dialogs.new_tb {
            egui::Window::new("New Testbench")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("DUT module name:");
                    ui.text_edit_singleline(&mut self.dialogs.tb_dut);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            self.new_tb_file();
                        }
                        if ui.button("Cancel").clicked() {
                            self.dialogs.new_tb = false;
                        }
                    });
                });
        }

        if self.dialogs.about {
            egui::Window::new("About Verilog IDE")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Verilog IDE");
                    ui.label("Desktop IDE for Verilog HDL and testbenches.");
                    ui.label("Built with Rust + egui.");
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        self.dialogs.about = false;
                    }
                });
        }

        if self.search_open {
            egui::Window::new("Find")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_TOP, [-20.0, 60.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let r = ui.text_edit_singleline(&mut self.search);
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            self.find_next();
                        }
                        if ui.button("Find").clicked() {
                            self.find_next();
                        }
                        if ui.button("Close").clicked() {
                            self.search_open = false;
                        }
                    });
                });
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
        let find = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
        let open = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);

        ctx.input_mut(|i| {
            if i.consume_shortcut(&save) {
                self.save_active();
            }
            if i.consume_shortcut(&find) {
                self.search_open = true;
            }
            if i.consume_shortcut(&open) {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.open_project(folder);
                }
            }
        });
    }
}

impl eframe::App for VerilogIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.ui_menu(ctx);
        self.ui_toolbar(ctx);
        self.ui_status(ctx);
        self.ui_dialogs(ctx);

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(self.bottom_height)
            .min_height(80.0)
            .show(ctx, |ui| {
                self.bottom_height = ui.available_height();
                self.ui_bottom(ui);
            });

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.left_width)
            .min_width(160.0)
            .show(ctx, |ui| {
                self.left_width = ui.available_width();
                self.ui_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_editor_area(ui);
        });
    }
}
