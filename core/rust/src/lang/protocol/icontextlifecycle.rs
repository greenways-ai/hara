pub trait IContextLifeCycle<M, P> {
    fn has_module(&self, module: &M) -> bool;
    fn setup_module(&mut self, module: M);
    fn teardown_module(&mut self, module: &M);
    fn has_pointer(&self, pointer: &P) -> bool;
    fn setup_pointer(&mut self, pointer: P);
    fn teardown_pointer(&mut self, pointer: &P);
}
