use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use log::{info, error, debug};
use termios::*;

use crate::core::connection::Connection;
use crate::core::{ConnectionManager, ConnectionError};

/// This struct manages the session’s lifetime and logic.
pub struct SerialSession {
    stop_flag: Arc<AtomicBool>,
    // The connection itself
    active_conn: Arc<Mutex<Box<dyn Connection + Send>>>,
    // Possibly store the manager if we want to call destroy_connection
    manager: ConnectionManager,
}

impl SerialSession {
    /// Create a new session, connect the port, etc.
    pub fn new(manager: ConnectionManager, conn: Box<dyn Connection + Send>) -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            active_conn: Arc::new(Mutex::new(conn)),
            manager,
        }
    }

    /// Spawns the reader thread. Returns a JoinHandle.
    fn spawn_reader_thread(&self) -> std::thread::JoinHandle<()> {
        let reader_stop_flag = Arc::clone(&self.stop_flag);
        let conn_reader = Arc::clone(&self.active_conn);

        std::thread::spawn(move || {
            let mut buffer = [0u8; 1]; 
            while !reader_stop_flag.load(Ordering::SeqCst) {
                {
                    let mut conn_guard = conn_reader.lock().unwrap();
                    match conn_guard.read(&mut buffer) {
                        Ok(1) => {
                            let ch = buffer[0];
                            if ch == b'\r' {
                                print!("\r\n");
                            } else {
                                print!("{}", ch as char);
                            }
                            io::stdout().flush().unwrap();
                            debug!("Ok(1)");
                        }
                        Ok(0) => {
                            // no data
                            debug!("Ok(0)");
                        }
                        Ok(_) => {
                            // in theory won't happen for a 1-byte buffer, but we must match
                            debug!("Ok(_)");
                        }
                        Err(ConnectionError::IoError(ref io_err))
                            if io_err.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            // just a timeout, check stop_flag again in next loop run
                            debug!("TimedOut read");
                        }
                        Err(e) => {
                            debug!("Error in read: {:?}", e);
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    }

    /// Runs the main loop for writing to the serial port from stdin.
    /// We do the "Ctrl+A then x" logic here.
    pub fn run(&mut self) -> Result<(), ConnectionError> {
        // Spawn the reader thread
        let reader_thread = self.spawn_reader_thread();

        // Terminal raw mode
        let original_termios = set_raw_mode()?;
        eprintln!("Raw mode enabled. Type into the terminal to send data.");
        eprintln!("Press Ctrl+A then 'x' to exit.");

        let mut last_was_ctrl_a = false;

        // Main loop reading from stdin
        let mut buffer = [0u8; 1];
        while !self.stop_flag.load(Ordering::SeqCst) {
            if io::stdin().read(&mut buffer).is_ok() {
                let ch = buffer[0];

                // Check for Ctrl+A
                if ch == 0x01 {
                    last_was_ctrl_a = true;
                    continue;
                }

                if last_was_ctrl_a && ch == b'x' {
                    info!("Ctrl+A + x pressed. Exiting...");
                    break;
                } else {
                    last_was_ctrl_a = false;
                    if ch == b'\n' {
                        let mut conn_guard = self.active_conn.lock().unwrap();
                        conn_guard.write(b"\r")?;
                    } else {
                        let mut conn_guard = self.active_conn.lock().unwrap();
                        conn_guard.write(&[ch])?;
                    }
                }
            } else {
                error!("Error reading from stdin. Breaking out.");
                break;
            }
        }

        // Indicate to the reader thread we should exit
        self.stop_flag.store(true, Ordering::SeqCst);

        // Wait for the reader thread
        reader_thread.join().ok();

        // Restore the terminal mode
        restore_mode(original_termios);

        // Finally, call disconnect on the connection by locking the Mutex
        {
            // We only need a mutable reference to call `destroy_connection()`.
            let mut conn_guard = self.active_conn.lock().unwrap();
            // `conn_guard` is a `MutexGuard<Box<dyn Connection>>`, so `&mut **conn_guard`
            // is a `&mut dyn Connection`.
            self.manager.destroy_connection(&mut **conn_guard)?;
        }

        Ok(())
    }
}

/// Put the terminal into raw mode. Return the original termios to restore later.
fn set_raw_mode() -> Result<Termios, ConnectionError> {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd)?;
    let original_termios = termios.clone();

    termios.c_lflag &= !(ICANON | ECHO);
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    tcsetattr(stdin_fd, TCSANOW, &termios)?;
    Ok(original_termios)
}

/// Restore original termios mode
fn restore_mode(original: Termios) {
    let stdin_fd = io::stdin().as_raw_fd();
    let _ = tcsetattr(stdin_fd, TCSANOW, &original);
    // We ignore errors here for simplicity
}
