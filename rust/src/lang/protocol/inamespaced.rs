pub trait INamespaced {
    fn get_name(&self) -> &str;
    fn get_namespace(&self) -> Option<&str>;

    fn path_string(&self) -> String {
        match self.get_namespace() {
            Some(namespace) => format!("{namespace}/{}", self.get_name()),
            None => self.get_name().to_owned(),
        }
    }
}
