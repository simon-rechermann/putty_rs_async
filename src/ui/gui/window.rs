use std::sync::{Arc, Mutex};
use log::{info, error};
use eframe;
use eframe::egui;

use crate::ui::cli::cli::Args;
use crate::connections::serial::SerialConnection;
use crate::core::connection_manager::{ConnectionManager, ConnectionHandle};

pub fn launch_gui(args: Args) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "putty_rs GUI",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(MyGuiApp::new(args.port, args.baud)))
        }),
    )
}

/// The main GUI application struct.
pub struct MyGuiApp {
    port: String,
    baud_str: String,

    // A Session that can hold multiple connections
    session: ConnectionManager,

    // We track whether we're connected
    connected: bool,

    // A handle to the current connection (if connected)
    handle: Option<ConnectionHandle>,

    // Buffers for displaying incoming data and typed input
    incoming_text: Arc<Mutex<String>>,
    terminal_input: String,
    old_terminal_input: String,
}

impl Default for MyGuiApp {
    fn default() -> Self {
        MyGuiApp {
            port: "/dev/pts/3".to_owned(),
            baud_str: "115200".to_owned(),
            session: ConnectionManager::new(),
            connected: false,
            handle: None,
            incoming_text: Arc::new(Mutex::new(String::new())),
            terminal_input: String::new(),
            old_terminal_input: String::new(),
        }
    }
}

impl eframe::App for MyGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("putty_rs GUI - Multiple Connections in Session");

            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port);

                ui.label("Baud:");
                ui.text_edit_singleline(&mut self.baud_str);

                if !self.connected {
                    if ui.button("Connect").clicked() {
                        self.connect();
                    }
                } else {
                    if ui.button("Disconnect").clicked() {
                        self.disconnect();
                    }
                }
            });

            ui.separator();

            ui.label("Terminal Output:");
            {
                let text_guard = self.incoming_text.lock().unwrap();
                let mut read_only_copy = text_guard.clone();
                drop(text_guard);

                egui::ScrollArea::vertical()
                    .id_salt("scroll_incoming_output")
                    .show(ui, |ui| {
                        ui.code_editor(&mut read_only_copy);
                    });
            }

            ui.separator();

            ui.label("Type here (new chars sent immediately):");
            egui::ScrollArea::vertical()
                .id_salt("scroll_terminal_input")
                .show(ui, |ui| {
                    ui.code_editor(&mut self.terminal_input);
                });

            // If connected, detect newly typed characters
            if self.connected {
                let old_len = self.old_terminal_input.len();
                let new_len = self.terminal_input.len();
                if new_len > old_len {
                    let new_chars = &self.terminal_input[old_len..new_len];
                    self.send_chars(new_chars);
                }
            }

            self.old_terminal_input = self.terminal_input.clone();

            ctx.request_repaint();
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Ensure we disconnect & free the port if still connected.
        if self.connected {
            self.disconnect();
        }
    }
}

impl MyGuiApp {
    pub fn new(port: Option<String>, baud: u32) -> Self {
        let mut s = Self::default();
        if let Some(p) = port {
            s.port = p;
        }
        s.baud_str = baud.to_string();
        s
    }

    fn connect(&mut self) {
        let baud = match self.baud_str.parse::<u32>() {
            Ok(b) => b,
            Err(_) => {
                error!("Invalid baud rate");
                return;
            }
        };

        let connection = SerialConnection::new(self.port.clone(), baud);

        let text_ref = self.incoming_text.clone();
        // This callback is called for every received byte, including the port id
        let on_byte = move |_conn_id: String, byte: u8| {
            let mut guard = text_ref.lock().unwrap();
            guard.push(byte as char);
        };

        match self.session.add_connection(self.port.clone(), Box::new(connection), on_byte) {
            Ok(h) => {
                self.handle = Some(h);
                self.connected = true;
                info!("Connected to {} at {}", self.port, baud);
            }
            Err(e) => {
                error!("Failed to connect: {:?}", e);
            }
        }
    }

    fn disconnect(&mut self) {
        if let Some(h) = self.handle.take() {
            if let Err(e) = h.stop() {
                error!("Error stopping connection: {:?}", e);
            }
        }
        self.connected = false;
        info!("Disconnected.");
    }

    fn send_chars(&self, chars: &str) {
        if let Some(ref h) = self.handle {
            if !chars.is_empty() {
                if let Err(e) = h.write_bytes(chars.as_bytes()) {
                    error!("Write error: {:?}", e);
                }
            }
        }
    }
}
