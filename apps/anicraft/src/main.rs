//! AniCraft desktop app (milestone 1.2: the application shell).
//!
//! A dark, Craft-styled eframe window wired to the shared infrastructure: `craft-ui` themes the
//! egui widget states and `craft-quit` routes window closes through the platform's own quit
//! (on macOS, leaving by closing the window is not a safe exit). The editor itself — stage,
//! timeline, tools, the engine session — lands in later milestones; this shell is deliberately
//! empty of features so nothing here is a placeholder.
//!
//! Usage: `anicraft [--version]`
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod theme;

/// The window's default and minimum sizes (the sibling apps' defaults; the stage viewport and
/// its fit-to-monitor behaviour come with the Phase 2 document foundation).
const DEFAULT_SIZE: [f32; 2] = [1440.0, 900.0];
const MIN_SIZE: [f32; 2] = [800.0, 500.0];

/// The desktop shell: a dark window identified as AniCraft. It keeps no document state yet, so
/// there is nothing to save on exit.
struct AniCraftApp;

impl eframe::App for AniCraftApp {
    /// The app's own close decision would go here (an unsaved-changes check once the engine
    /// session exists). `craft-quit`'s hook goes last, so it sees any `CancelClose` the app sent
    /// and otherwise takes the close over the platform's own way.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        craft_quit::on_close_request(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t = theme::Tokens::DARK;
        // The future stage viewport's surround: the dark pasteboard, with the app identified
        // against it. No panels, tools or documents exist yet, so nothing else is drawn.
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.pasteboard)).show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add_space(48.0);
                ui.label(egui::RichText::new("AniCraft").strong().size(t.font_size + 2.0).color(t.text_strong));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(11.0).color(t.text_dim));
            });
        });
    }
}

fn main() -> std::process::ExitCode {
    let mut show_version = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" => show_version = true,
            other => {
                eprintln!("anicraft: unknown argument `{other}` (the shell takes no files)");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    if show_version {
        println!("anicraft {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    let outcome = eframe::run_native(
        "AniCraft",
        native_options(),
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx, theme::Tokens::DARK);
            // macOS: replace winit's default app menu with one whose Quit (⌘Q) asks the app
            // first, like the close button (craft-quit). On error winit's menu stays and the
            // app still runs; fail the start-up rather than start without a graceful quit.
            #[cfg(target_os = "macos")]
            craft_quit::install_app_menu(&cc.egui_ctx, "AniCraft")?;
            Ok(Box::new(AniCraftApp))
        }),
    );
    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("anicraft: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        // The app id matches the future desktop-entry file name so Wayland docks find the
        // launcher icon (sibling convention: `ai.storyteller.<app>`).
        viewport: egui::ViewportBuilder::default()
            .with_title("AniCraft")
            .with_inner_size(DEFAULT_SIZE)
            .with_min_inner_size(MIN_SIZE)
            .with_app_id("ai.storyteller.anicraft"),
        ..Default::default()
    }
}
