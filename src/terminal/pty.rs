// PTY lifecycle: spawn, resize, read/write, cleanup

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};

/// Event listener that forwards terminal events and triggers egui repaints.
#[derive(Clone)]
pub struct EventProxy {
    ctx: egui::Context,
    event_tx: std::sync::mpsc::Sender<Event>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.event_tx.send(event);
        self.ctx.request_repaint();
    }
}

/// Terminal dimensions implementing alacritty_terminal's Dimensions trait.
pub struct TermDimensions {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// Manages a PTY + alacritty_terminal::Term pair.
pub struct PtyHandle {
    pub term: Arc<Mutex<Term<EventProxy>>>,
    pub title: Arc<Mutex<String>>,
    pub alive: Arc<AtomicBool>,
    pub bell_fired: Arc<AtomicBool>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    last_input_at: Arc<Mutex<Instant>>,
    last_output_at: Arc<Mutex<Instant>>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    _event_thread: thread::JoinHandle<()>,
    _reader_thread: thread::JoinHandle<()>,
    _waiter_thread: thread::JoinHandle<()>,
}

impl PtyHandle {
    /// Spawn a new terminal with a shell process.
    pub fn spawn(
        ctx: &egui::Context,
        rows: u16,
        cols: u16,
        title: &str,
        cwd: Option<&std::path::Path>,
    ) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Build shell command
        let mut cmd = CommandBuilder::new_default_prog();
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("VOID_TERMINAL", "1");
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let mut child = pair.slave.spawn_command(cmd)?;
        let killer = child.clone_killer();
        drop(pair.slave);

        // Create terminal state machine
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let event_proxy = EventProxy {
            ctx: ctx.clone(),
            event_tx,
        };

        let config = Config::default();
        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
        };
        let term = Arc::new(Mutex::new(Term::new(config, &dims, event_proxy)));
        let title = Arc::new(Mutex::new(title.to_string()));
        let alive = Arc::new(AtomicBool::new(true));
        let bell_fired = Arc::new(AtomicBool::new(false));
        let now = Instant::now();
        let last_input_at = Arc::new(Mutex::new(now));
        let last_output_at = Arc::new(Mutex::new(now));

        // Set up I/O
        let mut reader = pair.master.try_clone_reader()?;
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer()?));

        // Spawn reader thread
        let term_clone = term.clone();
        let alive_clone = alive.clone();
        let writer_clone = writer.clone();
        let ctx_clone = ctx.clone();
        let title_clone = title.clone();
        let alive_clone_events = alive.clone();
        let ctx_clone_events = ctx.clone();
        let last_output_clone = last_output_at.clone();
        let bell_clone = bell_fired.clone();
        let alive_clone_wait = alive.clone();
        let ctx_clone_wait = ctx.clone();

        let event_thread = thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                match event {
                    Event::PtyWrite(text) => {
                        if let Ok(mut w) = writer_clone.lock() {
                            let _ = w.write_all(text.as_bytes());
                            let _ = w.flush();
                        }
                    }
                    Event::Title(t) => {
                        if let Ok(mut title) = title_clone.lock() {
                            *title = t;
                        }
                    }
                    Event::ResetTitle => {
                        if let Ok(mut title) = title_clone.lock() {
                            *title = "Terminal".to_string();
                        }
                    }
                    Event::ChildExit(_) | Event::Exit => {
                        alive_clone_events.store(false, Ordering::Relaxed);
                    }
                    Event::Bell => {
                        bell_clone.store(true, Ordering::Relaxed);
                    }
                    Event::ClipboardStore(_, data) => {
                        // OSC 52: program (vim, etc.) wants to set clipboard
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(data);
                        }
                    }
                    Event::Wakeup
                    | Event::MouseCursorDirty
                    | Event::CursorBlinkingChange
                    | Event::ClipboardLoad(_, _)
                    | Event::ColorRequest(_, _)
                    | Event::TextAreaSizeRequest(_) => {}
                }

                ctx_clone_events.request_repaint();
                if !alive_clone_events.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        let reader_thread = thread::spawn(move || {
            let mut processor: Processor = Processor::new();
            let mut buf = [0u8; 4096];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Feed bytes to terminal parser
                        {
                            let Ok(mut term) = term_clone.lock() else {
                                break;
                            };
                            processor.advance(&mut *term, &buf[..n]);
                        }
                        if let Ok(mut last_output) = last_output_clone.lock() {
                            *last_output = Instant::now();
                        }

                        ctx_clone.request_repaint();
                    }
                    Err(e) => {
                        log::debug!("PTY read error: {e}");
                        break;
                    }
                }
            }

            alive_clone.store(false, Ordering::Relaxed);
            ctx_clone.request_repaint();
        });

        let waiter_thread = thread::spawn(move || {
            let _ = child.wait();
            alive_clone_wait.store(false, Ordering::Relaxed);
            ctx_clone_wait.request_repaint();
        });

        Ok(Self {
            term,
            title,
            alive,
            bell_fired,
            writer,
            last_input_at,
            last_output_at,
            master: pair.master,
            killer,
            _event_thread: event_thread,
            _reader_thread: reader_thread,
            _waiter_thread: waiter_thread,
        })
    }

    /// Write bytes to the PTY (keyboard input).
    pub fn write(&self, data: &[u8]) {
        if let Ok(mut last_input) = self.last_input_at.lock() {
            *last_input = Instant::now();
        }
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.write_all(data);
            let _ = writer.flush();
        }
    }

    /// Resize the PTY and terminal grid.
    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });

        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
        };
        if let Ok(mut term) = self.term.lock() {
            term.resize(dims);
        }
    }

    /// Check if the child process is still alive.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn should_hide_cursor_for_streaming_output(&self) -> bool {
        const CURSOR_HIDE_AFTER_OUTPUT: Duration = Duration::from_millis(220);

        let Ok(last_output) = self.last_output_at.lock() else {
            return false;
        };
        let Ok(last_input) = self.last_input_at.lock() else {
            return false;
        };

        *last_output > *last_input && last_output.elapsed() < CURSOR_HIDE_AFTER_OUTPUT
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.killer.kill();
    }
}
