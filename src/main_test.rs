#[macro_use]
extern crate stdweb;
use std::rc::Rc;
use std::sync::Mutex;
use stdweb::web::*;

struct App {
    app: Option<Rc<Mutex<App>>>,
    val: i32,
}

impl App {
    fn test(&mut self) {
        self.val += 1;
        let app = self.app.clone().unwrap();
        set_timeout(
            move || {
                let mut app = app.lock().unwrap();
                app.val += 1;
                js!(console.log("a=", @{app.val}));
            },
            1000,
        );
    }
}

fn main() {
    stdweb::initialize();

    let app = Rc::new(Mutex::new(App { val: 0, app: None }));
    let mut app1 = app.lock().unwrap();
    app1.app = Some(app.clone());

    app1.test();
    app1.test();

    stdweb::event_loop();
}
