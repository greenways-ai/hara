pub trait ISpace<C, K, O> {
    type Runtime;

    fn context_set(&mut self, context: C, key: K, options: O);
    fn context_unset(&mut self, context: &C);
    fn context_list(&self) -> Vec<C>;
    fn context_get(&self, context: &C) -> Option<O>;
    fn runtime_active(&self) -> Vec<Self::Runtime>;
    fn runtime_get(&self, context: &C) -> Option<Self::Runtime>;
    fn runtime_start(&mut self, context: C) -> Self::Runtime;
    fn runtime_started(&self, context: &C) -> bool;
    fn runtime_stopped(&self, context: &C) -> bool;
    fn runtime_stop(&mut self, context: &C);
}
