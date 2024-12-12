use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui};

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
                ui.button("Save");
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
    });
}

#[derive(Resource, Default)]
struct EditorState {
    text: String
}