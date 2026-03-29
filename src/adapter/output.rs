use crate::port::output::{OutputWriter, Spinner};

#[derive(Default)]
pub struct CliOutput;

impl OutputWriter for CliOutput {
    fn info(&self, msg: &str) {
        cliclack::log::info(msg).ok();
    }

    fn success(&self, msg: &str) {
        cliclack::log::success(msg).ok();
    }

    fn warning(&self, msg: &str) {
        cliclack::log::warning(msg).ok();
    }

    fn step(&self, msg: &str) {
        cliclack::log::step(msg).ok();
    }

    fn intro(&self, title: &str) {
        cliclack::intro(title).ok();
    }

    fn outro(&self, msg: &str) {
        cliclack::outro(msg).ok();
    }

    fn raw(&self, msg: &str) {
        println!("{}", msg);
    }

    fn spinner(&self) -> Box<dyn Spinner> {
        Box::new(CliSpinner(cliclack::spinner()))
    }
}

struct CliSpinner(cliclack::ProgressBar);

impl Spinner for CliSpinner {
    fn start(&self, msg: &str) {
        self.0.start(msg);
    }

    fn stop(&self, msg: &str) {
        self.0.stop(msg);
    }
}
