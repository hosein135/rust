//! Main IDE application state and UI (GPUI Component).

use crate::editor::{cursor_line_col, position_from_cursor};
use crate::project::{build_tree_items, load_file, save_file, IdeProject, OpenFile};
use crate::templates::{self, counter_example};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Root, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::{
        Editor, EditorState, Input, InputEvent, InputState, TabSize, Textarea, TextareaState,
    },
    list::ListItem,
    resizable::{h_resizable, resizable_panel, v_resizable},
    status_bar::StatusBar,
    tree::{TreeState, tree},
    *,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Console,
    Problems,
}

pub struct VerilogIde {
    project: Option<IdeProject>,
    tree_state: Entity<TreeState>,
    open: Vec<OpenFile>,
    active: Option<usize>,
    editor: Entity<EditorState>,
    console: Entity<TextareaState>,
    problems: Vec<String>,
    bottom: BottomTab,
    status: SharedString,
    dialog_module_input: Entity<InputState>,
    dialog_tb_input: Entity<InputState>,
    dialog_search_input: Entity<InputState>,
    search_query: String,
    bottom_height: Pixels,
    _subscriptions: Vec<Subscription>,
}

impl VerilogIde {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("c")
                .line_number(true)
                .indent_guides(true)
                .soft_wrap(false)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .placeholder("Open a Verilog file to start editing...")
        });

        let console = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(6)
                .default_value(
                    "Verilog IDE ready.\nOpen a folder or create a sample project.\n",
                )
        });

        let tree_state = cx.new(|cx| TreeState::new(cx));
        let dialog_module_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("module_name"));
        let dialog_tb_input = cx.new(|cx| InputState::new(window, cx).placeholder("dut_name"));
        let dialog_search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find..."));

        let mut ide = Self {
            project: None,
            tree_state,
            open: Vec::new(),
            active: None,
            editor,
            console,
            problems: Vec::new(),
            bottom: BottomTab::Console,
            status: "Ready".into(),
            dialog_module_input,
            dialog_tb_input,
            dialog_search_input,
            search_query: String::new(),
            bottom_height: px(180.),
            _subscriptions: Vec::new(),
        };

        ide._subscriptions.push(cx.subscribe(&ide.editor, {
            move |this, editor, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    if let Some(i) = this.active {
                        if let Some(file) = this.open.get_mut(i) {
                            file.content = editor.read(cx).value().to_string();
                            file.dirty = true;
                            file.cursor = editor.read(cx).cursor();
                        }
                    }
                    cx.notify();
                }
            }
        }));

        for candidate in [
            PathBuf::from("samples"),
            PathBuf::from("examples"),
            std::env::current_dir()
                .ok()
                .map(|p| p.join("samples"))
                .unwrap_or_default(),
        ] {
            if candidate.is_dir() {
                ide.open_project(candidate, window, cx);
                break;
            }
        }

        ide
    }

    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        if let Some(project) = self.project.as_ref() {
            let items = build_tree_items(&project.root, &project.root);
            self.tree_state.update(cx, |state, cx| {
                state.set_items(items, cx);
            });
        }
    }

    fn sync_editor_to_active(&mut self, cx: &mut Context<Self>) {
        if let Some(i) = self.active {
            if let Some(file) = self.open.get_mut(i) {
                file.content = self.editor.read(cx).value().to_string();
                file.cursor = self.editor.read(cx).cursor();
            }
        }
    }

    fn load_active_into_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.active else { return };
        let Some(file) = self.open.get(i).cloned() else { return };
        self.editor.update(cx, |state, cx| {
            state.set_value(file.content.clone(), window, cx);
            state.set_cursor_position(
                position_from_cursor(&file.content, file.cursor),
                window,
                cx,
            );
        });
    }

    fn log(&mut self, msg: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.console.update(cx, |state, cx| {
            let mut value = state.value().to_string();
            value.push_str(msg);
            state.set_value(value, window, cx);
        });
    }

    fn log_err(&mut self, msg: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.console.update(cx, |state, cx| {
            let mut value = state.value().to_string();
            value.push_str(&format!("ERROR: {msg}\n"));
            state.set_value(value, window, cx);
        });
        self.status = msg.into();
    }

    fn open_project(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let project = IdeProject::new(root);
        self.log(
            &format!("Opened project: {}\n", project.root.display()),
            window,
            cx,
        );
        self.project = Some(project);
        self.open.clear();
        self.active = None;
        self.refresh_tree(cx);
        self.status = "Project opened".into();

        if let Some(p) = self.project.as_ref() {
            if let Some(first) = p.list_verilog_files().into_iter().next() {
                self.open_path(&first, window, cx);
            }
        }
        cx.notify();
    }

    fn open_path(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_editor_to_active(cx);

        if let Some(idx) = self.open.iter().position(|f| f.path == path) {
            self.active = Some(idx);
            self.load_active_into_editor(window, cx);
            cx.notify();
            return;
        }

        match load_file(path) {
            Ok(file) => {
                self.open.push(file);
                self.active = Some(self.open.len() - 1);
                self.status = format!("Opened {}", path.display()).into();
                self.load_active_into_editor(window, cx);
            }
            Err(e) => {
                self.log_err(&e, window, cx);
                self.problems.push(e);
            }
        }
        cx.notify();
    }

    fn save_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_editor_to_active(cx);
        let Some(i) = self.active else { return };
        match save_file(&mut self.open[i]) {
            Ok(()) => {
                self.status = format!("Saved {}", self.open[i].path.display()).into();
                self.log(
                    &format!("Saved {}\n", self.open[i].path.display()),
                    window,
                    cx,
                );
            }
            Err(e) => {
                self.log_err(&e, window, cx);
                self.problems.push(e);
            }
        }
        cx.notify();
    }

    fn save_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_editor_to_active(cx);
        for f in &mut self.open {
            if f.dirty {
                if let Err(e) = save_file(f) {
                    self.problems.push(e.clone());
                    self.log(&format!("ERROR: {e}\n"), window, cx);
                }
            }
        }
        self.status = "Saved all".into();
        cx.notify();
    }

    fn close_tab(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.get(idx).map(|f| f.dirty).unwrap_or(false) {
            let _ = save_file(&mut self.open[idx]);
        }
        self.open.remove(idx);
        self.active = if self.open.is_empty() {
            None
        } else {
            Some(idx.min(self.open.len() - 1))
        };
        self.load_active_into_editor(window, cx);
        cx.notify();
    }

    fn create_sample_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose parent folder for sample project".into()),
        });
        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let Some(parent) = path.await.ok().flatten().and_then(|p| p.into_iter().next()) else {
                return;
            };
            let root = parent.join("verilog-sample");
            let _ = window.update(|window, cx| {
                if let Err(e) = std::fs::create_dir_all(&root) {
                    view.update(cx, |this, cx| {
                        this.log_err(&e.to_string(), window, cx);
                        cx.notify();
                    });
                    return;
                }
                let (n1, c1, n2, c2) = counter_example();
                let _ = std::fs::write(root.join(n1), c1);
                let _ = std::fs::write(root.join(n2), c2);
                view.update(cx, |this, cx| {
                    this.open_project(root, window, cx);
                    this.log("Created sample counter + testbench project.\n", window, cx);
                });
            });
        })
        .detach();
    }

    fn prompt_open_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open project folder".into()),
        });
        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let Some(folder) = path.await.ok().flatten().and_then(|p| p.into_iter().next()) else {
                return;
            };
            let _ = window.update(|window, cx| {
                view.update(cx, |this, cx| {
                    this.open_project(folder, window, cx);
                });
            });
        })
        .detach();
    }

    fn show_new_module_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.dialog_module_input.clone();
        let view = cx.entity();
        window.open_alert_dialog(cx, move |dialog, window, cx| {
            dialog
                .title("New Verilog Module")
                .child(div().child("Module name:"))
                .child(Input::new(&input))
                .on_ok(move |_, window, cx| {
                    let name = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| {
                        if name.trim().is_empty() {
                            return;
                        }
                        let Some(project) = this.project.as_ref() else {
                            this.log_err("Open a project first.", window, cx);
                            cx.notify();
                            return;
                        };
                        let path = project.root.join(format!("{}.v", name.trim()));
                        let body = templates::module_template(name.trim());
                        if let Err(e) = std::fs::write(&path, body) {
                            this.log_err(&e.to_string(), window, cx);
                            cx.notify();
                            return;
                        }
                        this.refresh_tree(cx);
                        this.open_path(&path, window, cx);
                        this.log(
                            &format!("Created module {}\n", path.display()),
                            window,
                            cx,
                        );
                        cx.notify();
                    });
                    true
                });
        });
    }

    fn show_new_tb_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.dialog_tb_input.clone();
        let view = cx.entity();
        window.open_alert_dialog(cx, move |dialog, window, cx| {
            dialog
                .title("New Testbench")
                .child(div().child("DUT module name:"))
                .child(Input::new(&input))
                .on_ok(move |_, window, cx| {
                    let dut = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| {
                        if dut.trim().is_empty() {
                            return;
                        }
                        let Some(project) = this.project.as_ref() else {
                            this.log_err("Open a project first.", window, cx);
                            cx.notify();
                            return;
                        };
                        let path = project.root.join(format!("{}_tb.v", dut.trim()));
                        let body = templates::testbench_template(dut.trim());
                        if let Err(e) = std::fs::write(&path, body) {
                            this.log_err(&e.to_string(), window, cx);
                            cx.notify();
                            return;
                        }
                        this.refresh_tree(cx);
                        this.open_path(&path, window, cx);
                        this.log(
                            &format!("Created testbench {}\n", path.display()),
                            window,
                            cx,
                        );
                        cx.notify();
                    });
                    true
                });
        });
    }

    fn show_find_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.dialog_search_input.clone();
        let view = cx.entity();
        window.open_alert_dialog(cx, move |dialog, window, cx| {
            input.update(cx, |state, cx| {
                state.focus(window, cx);
            });
            dialog.title("Find").child(Input::new(&input)).on_ok(
                move |_, window, cx| {
                    let search = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| {
                        this.search_query = search;
                        this.find_next(window, cx);
                    });
                    true
                },
            );
        });
    }

    fn show_about_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.open_alert_dialog(cx, |dialog, _, _| {
            dialog
                .title("About Verilog IDE")
                .child(div().child("Verilog IDE"))
                .child(div().child("Desktop IDE for Verilog HDL and testbenches."))
                .child(div().child("Built with Rust + GPUI Component."))
                .on_ok(|_, _, _| true);
        });
    }

    fn find_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let needle = self.search_query.clone();
        if needle.is_empty() {
            return;
        }
        self.sync_editor_to_active(cx);
        let Some(i) = self.active else { return };
        let content = self.open[i].content.clone();
        let start = self.open[i].cursor.saturating_add(1).min(content.len());
        let found = content[start..]
            .find(&needle)
            .map(|o| start + o)
            .or_else(|| content.find(&needle));
        if let Some(pos) = found {
            self.open[i].cursor = pos;
            self.editor.update(cx, |state, cx| {
                state.set_cursor_position(position_from_cursor(&content, pos), window, cx);
            });
            self.status = format!("Found at {pos}").into();
        } else {
            self.status = "Not found".into();
        }
        cx.notify();
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("open-project")
                    .label("Open")
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.prompt_open_project(window, cx);
                    })),
            )
            .child(
                Button::new("save")
                    .label("Save")
                    .small()
                    .disabled(self.active.is_none())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.save_active(window, cx);
                    })),
            )
            .child(
                Button::new("save-all")
                    .label("Save All")
                    .small()
                    .disabled(self.open.is_empty())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.save_all(window, cx);
                    })),
            )
            .child(
                Button::new("new-module")
                    .label("+ Module")
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_new_module_dialog(window, cx);
                    })),
            )
            .child(
                Button::new("new-tb")
                    .label("+ Testbench")
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_new_tb_dialog(window, cx);
                    })),
            )
            .child(
                Button::new("sample")
                    .label("Sample")
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.create_sample_project(window, cx);
                    })),
            )
            .child(
                Button::new("find")
                    .label("Find")
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_find_dialog(window, cx);
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .items_center()
                    .when_some(self.project.as_ref(), |this, p| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().primary)
                                .child(p.name.clone()),
                        )
                    }),
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .h_full()
            .w_full()
            .bg(cx.theme().sidebar)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().sidebar_foreground)
                    .child("Explorer"),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.project.is_none(), |this| {
                        this.child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No project open."),
                                )
                                .child(
                                    Button::new("sidebar-open")
                                        .label("Open Project...")
                                        .small()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.prompt_open_project(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("sidebar-sample")
                                        .label("Create Sample...")
                                        .small()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.create_sample_project(window, cx);
                                        })),
                                ),
                        )
                    })
                    .when(self.project.is_some(), |this| {
                        this.child(self.render_file_tree(cx))
                    }),
            )
    }

    fn render_file_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        tree(
            &self.tree_state,
            move |ix, entry, _selected, window, cx| {
                let item = entry.item().clone();
                let icon = if !entry.is_folder() {
                    IconName::File
                } else if entry.is_expanded() {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                };

                ListItem::new(ix)
                    .w_full()
                    .rounded(cx.theme().radius)
                    .py_0p5()
                    .px_2()
                    .pl(px(16.) * entry.depth() + px(8.))
                    .child(h_flex().gap_2().child(icon).child(item.label.clone()))
                    .on_click(cx.listener({
                        move |_, _, window, cx| {
                            if item.is_folder() {
                                return;
                            }
                            let path = PathBuf::from(item.id.as_str());
                            view.update(cx, |this, cx| {
                                this.open_path(&path, window, cx);
                            });
                        }
                    }))
            },
        )
        .text_sm()
        .p_1()
        .h_full()
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .children(self.open.iter().enumerate().map(|(i, f)| {
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
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(("tab", i))
                            .label(title)
                            .small()
                            .selected(selected)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.sync_editor_to_active(cx);
                                this.active = Some(i);
                                this.load_active_into_editor(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(("close-tab", i))
                            .label("×")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.close_tab(i, window, cx);
                            })),
                    )
            }))
    }

    fn render_editor_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .child(
                    div()
                        .text_xl()
                        .font_bold()
                        .text_color(cx.theme().primary)
                        .child("Verilog IDE"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Edit Verilog modules and testbenches."),
                )
                .child(
                    Button::new("welcome-open")
                        .label("Open Project Folder")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.prompt_open_project(window, cx);
                        })),
                )
                .child(
                    Button::new("welcome-sample")
                        .label("Create Sample Counter Project")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.create_sample_project(window, cx);
                        })),
                )
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_tabs(cx))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(
                            Editor::new(&self.editor)
                                .h(relative(1.))
                                .bordered(false)
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size),
                        ),
                )
        }
    }

    fn render_bottom(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .h_full()
            .child(
                h_flex()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("console-tab")
                            .label("Console")
                            .xsmall()
                            .ghost()
                            .selected(self.bottom == BottomTab::Console)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.bottom = BottomTab::Console;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("problems-tab")
                            .label(format!("Problems ({})", self.problems.len()))
                            .xsmall()
                            .ghost()
                            .selected(self.bottom == BottomTab::Problems)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.bottom = BottomTab::Problems;
                                cx.notify();
                            })),
                    )
                    .child(
                        div().flex_1().child(
                            Button::new("clear-bottom")
                                .label("Clear")
                                .xsmall()
                                .ghost()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    match this.bottom {
                                        BottomTab::Console => {
                                            this.console.update(cx, |state, cx| {
                                                state.set_value("", window, cx);
                                            });
                                        }
                                        BottomTab::Problems => this.problems.clear(),
                                    }
                                    cx.notify();
                                })),
                        ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.bottom == BottomTab::Console, |this| {
                        this.child(
                            Textarea::new(&self.console)
                                .h_full()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size),
                        )
                    })
                    .when(self.bottom == BottomTab::Problems, |this| {
                        this.child(
                            v_flex()
                                .p_2()
                                .gap_1()
                                .overflow_y_scroll()
                                .when(self.problems.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().chart_2)
                                            .child("No problems."),
                                    )
                                })
                                .children(self.problems.iter().enumerate().map(|(i, p)| {
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().destructive)
                                        .child(format!("{}. {p}", i + 1))
                                })),
                        )
                    }),
            )
    }
}

impl Render for VerilogIde {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_line = if let Some(i) = self.active {
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
                self.status.to_string()
            }
        } else {
            self.status.to_string()
        };

        let dialog_layer = Root::render_dialog_layer(window, cx);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_3()
                        .occlude()
                        .child(div().font_semibold().child("Verilog IDE"))
                        .child(
                            Button::new("about")
                                .label("About")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.show_about_dialog(window, cx);
                                })),
                        ),
                ),
            )
            .child(self.render_toolbar(cx))
            .child(
                h_resizable("main-split")
                    .flex_1()
                    .min_h_0()
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .child(self.render_sidebar(cx)),
                    )
                    .child(
                        v_resizable("editor-bottom")
                            .flex_1()
                            .min_w_0()
                            .child(
                                resizable_panel()
                                    .flex_1()
                                    .child(self.render_editor_area(cx)),
                            )
                            .child(
                                resizable_panel()
                                    .size(self.bottom_height)
                                    .child(self.render_bottom(cx)),
                            ),
                    ),
            )
            .child(
                StatusBar::new()
                    .left(div().text_xs().child(self.status.clone()))
                    .right(div().text_xs().child(status_line)),
            )
            .children(dialog_layer)
    }
}
