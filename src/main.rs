use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui};
use egui_file::FileDialog;
use serde::{Serialize, Deserialize};
use std::{fs::File, io::Read};
use std::io::Write;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(EditorState::default())
        .add_systems(Startup, setup)
        .add_systems(Update, ui)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite::from_image(
        asset_server.load("wrench.png")
    ));
}

fn ui(mut contexts: EguiContexts, mut editor_state: ResMut<EditorState>) {
    egui::TopBottomPanel::top("menu").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New...").clicked() {
                    editor_state.new();
                }
                if ui.button("Save As...").clicked() {
                    let mut dialog = FileDialog::save_file(None);
                    dialog.open();
                    editor_state.file_dialog = Some(dialog)
                }
            });
        });
    });
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        egui::SidePanel::new(egui::panel::Side::Left, "main_left_panel")
            .default_width(512.).show_inside(ui, |ui| 
        {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut editor_state.text).code_editor())
            });
        });

        let mut file_to_save = None;

        if let Some(file_dialog) = &mut editor_state.file_dialog {
            if file_dialog.show(ui.ctx()).selected() {
                if let Some(file) = file_dialog.path() {
                    file_to_save = Some(file.as_os_str().to_string_lossy().into_owned());
                }
            }
        }

        if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
            new_project_dialog.show(ui.ctx());
        }

        if let Some(file_to_save) = file_to_save {
            editor_state.set_project_file_path(file_to_save);
            editor_state.save();
        }
    });
}

#[derive(Resource, Default)]
struct EditorState {
    project_file_path: String,
    text: String,
    file_dialog: Option<FileDialog>,
    project_data: Option<ProjectData>,
    new_file_dialog: Option<NewProjectDialog>
}

impl EditorState {
    fn set_project_file_path(&mut self, path: String) {
        self.project_file_path = path
    }

    fn new(&mut self) {
        let mut new_file_dialog = NewProjectDialog::default();
        new_file_dialog.open();
        self.new_file_dialog = Some(new_file_dialog);
        self.project_data = Some(ProjectData::default());
    }

    fn save(&mut self) {
        let mut vec = Vec::new();
        if let Some(project_data) = &self.project_data {
            vec = serde_json::to_vec_pretty(&project_data).unwrap();
        }

        File::create(self.project_file_path.clone()).unwrap().write_all(&vec[..]);
    }
}

#[derive(Serialize, Deserialize, Default)]
struct ProjectData {
    lyrics: String,
}

struct NewProjectDialog {
    is_open: bool,
    artist: String
}

impl Default for NewProjectDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            artist: "glass beach".into()
        }
    }
}

impl NewProjectDialog {
    fn open(&mut self) {
        self.is_open = true;
    }

    fn show(&mut self, ctx: &egui::Context) {
        if self.is_open {
            egui::Window::new("New Project").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Artist");
                    ui.text_edit_singleline(&mut self.artist)
                });
                ui.label("Title");
                ui.label("Album");
                ui.label("Song File");
                ui.button("Create");
            });
        }
    }
}