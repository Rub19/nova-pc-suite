use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{EventLoopProxy, Platform, WindowAdapter};
use std::{
    rc::Rc,
    sync::{mpsc, Mutex},
    time::Duration,
};

type Job = Box<dyn FnOnce() + Send>;

enum Message {
    Job(Job),
    Quit,
}

struct Proxy(Mutex<mpsc::Sender<Message>>);

impl EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        let _ = self.0.lock().unwrap().send(Message::Quit);
        Ok(())
    }
    fn invoke_from_event_loop(&self, event: Job) -> Result<(), slint::EventLoopError> {
        let _ = self.0.lock().unwrap().send(Message::Job(event));
        Ok(())
    }
}

struct Headless {
    window: Rc<MinimalSoftwareWindow>,
    sender: mpsc::Sender<Message>,
    receiver: Mutex<mpsc::Receiver<Message>>,
}

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(Proxy(Mutex::new(self.sender.clone()))))
    }
    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        loop {
            slint::platform::update_timers_and_animations();
            while let Ok(message) = self.receiver.lock().unwrap().try_recv() {
                match message {
                    Message::Job(job) => job(),
                    Message::Quit => return Ok(()),
                }
            }
            std::thread::sleep(Duration::from_millis(8));
        }
    }
}

pub fn install() {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let (sender, receiver) = mpsc::channel();
    slint::platform::set_platform(Box::new(Headless {
        window,
        sender,
        receiver: Mutex::new(receiver),
    }))
    .expect("plateforme sans fenêtre impossible");
}
