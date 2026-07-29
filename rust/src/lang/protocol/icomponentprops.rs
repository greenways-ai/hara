pub trait IComponentProps {
    type Props;

    fn props(&self) -> Self::Props;
}
