use std::sync::{Arc, Mutex};
use log::{info, error};
use eframe;
use eframe::egui;

use crate::connections::serial::SerialConnection;
use crate::core::ConnectionManager;
use crate::core::session::Session;

pub fn launch_gui() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "putty_rs GUI",
        native_options,
        Box::new(|_cc| {
            // eframe 0.30+ requires returning Result<Box<dyn App>, Box<dyn std::error::Error>>
            Ok(Box::new(MyGuiApp::default()))
        }),
    )
}

/// The main GUI application struct.
pub struct MyGuiApp {
    // Inputs for port and baud
    port: String,
    baud_str: String,

    // Are we connected?
    connected: bool,

    // The session, if connected
    session: Option<Session>,

    // The text buffer holding incoming data (like terminal output)
    incoming_text: Arc<Mutex<String>>,

    // The user “terminal” input buffer
    terminal_input: String,
    old_terminal_input: String,
}

// Provide a custom `Default` so we can specify initial values:
impl Default for MyGuiApp {
    fn default() -> Self {
        MyGuiApp {
            port: "/dev/pts/3".to_owned(),    // Default port
            baud_str: "115200".to_owned(),   // Default baud
            connected: false,
            session: None,
            incoming_text: Arc::new(Mutex::new(String::new())),
            terminal_input: String::new(),
            old_terminal_input: String::new(),
        }
    }
}

impl eframe::App for MyGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("putty_rs GUI - Immediate Terminal");

            // Port / Baud / Connect / Disconnect
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

            // “Output” area (incoming data)
            ui.label("Terminal Output:");
            {
                let text_guard = self.incoming_text.lock().unwrap();
                // Make a local mutable copy for display only (read-only)
                let mut read_only_copy = text_guard.clone();
                drop(text_guard);

                egui::ScrollArea::vertical()
                    .id_salt("scroll_incoming_output")
                    .show(ui, |ui| {
                        ui.code_editor(&mut read_only_copy);
                    });
            }

            ui.separator();

            // “Input” area
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
                    // Send only new substring
                    let new_chars = &self.terminal_input[old_len..new_len];
                    self.send_chars(new_chars);
                }
            }

            // Remember new input for next frame
            self.old_terminal_input = self.terminal_input.clone();

            ctx.request_repaint();
        });
    }
}

impl MyGuiApp {
    fn connect(&mut self) {
        let baud = match self.baud_str.parse::<u32>() {
            Ok(b) => b,
            Err(_) => {
                error!("Invalid baud rate");
                return;
            }
        };
        let manager = ConnectionManager::new();
        let connection = SerialConnection::new(self.port.clone(), baud);

        match manager.create_connection(connection) {
            Ok(conn) => {
                let text_ref = self.incoming_text.clone();
                // Callback for incoming data: append to “incoming_text”
                let on_byte = move |byte: u8| {
                    let mut guard = text_ref.lock().unwrap();
                    if byte == b'\r' {
                        guard.push('\n');
                    } else {
                        guard.push(byte as char);
                    }
                };

                let mut session = Session::new(manager, Box::new(conn), on_byte);
                session.start();

                self.session = Some(session);
                self.connected = true;
                info!("Connected to {} at {}", self.port, baud);
            }
            Err(e) => {
                error!("Failed to connect: {:?}", e);
            }
        }
    }

    fn disconnect(&mut self) {
        if let Some(ref mut s) = self.session {
            if let Err(e) = s.stop() {
                error!("Disconnect error: {:?}", e);
            }
        }
        self.session = None;
        self.connected = false;
        info!("Disconnected.");
    }

    fn send_chars(&self, chars: &str) {
        if let Some(s) = &self.session {
            if !chars.is_empty() {
                if let Err(e) = s.write_bytes(chars.as_bytes()) {
                    error!("Error sending data: {:?}", e);
                }
            }
        }
    }
}
