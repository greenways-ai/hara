pub trait IComponentTrack {
    type Path;

    fn track_path(&self) -> Self::Path;
}
