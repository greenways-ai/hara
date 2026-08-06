pub trait IComponent {
    type Metadata;

    fn props(&self) -> Self::Metadata;
    fn status(&self) -> Self::Metadata;
    fn started(&self) -> bool;
    fn stopped(&self) -> bool;
    fn start(&mut self);
    fn stop(&mut self);
    fn kill(&mut self) {
        self.stop();
    }
    fn remote(&self) -> bool {
        false
    }
}
