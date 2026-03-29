pub trait OutputWriter: Send + Sync {
    fn info(&self, msg: &str);
    fn success(&self, msg: &str);
    fn warning(&self, msg: &str);
    fn step(&self, msg: &str);
    fn intro(&self, title: &str);
    fn outro(&self, msg: &str);
    fn raw(&self, msg: &str);
    fn spinner(&self) -> Box<dyn Spinner>;
}

pub trait Spinner: Send {
    fn start(&self, msg: &str);
    fn stop(&self, msg: &str);
}
