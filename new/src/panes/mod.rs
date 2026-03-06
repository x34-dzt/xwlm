pub mod modes;
pub mod monitor_map;
pub mod scale;
pub mod transform;
pub mod workspace;

#[derive(Debug, Default, PartialEq)]
pub enum Pane {
    #[default]
    Map,
    Mode,
    Scale,
    Transform,
    Worksapce,
}

impl Pane {
    pub fn next(&self) -> Pane {
        match self {
            Pane::Map => Pane::Mode,
            Pane::Mode => Pane::Worksapce,
            Pane::Worksapce => Pane::Scale,
            Pane::Scale => Pane::Transform,
            Pane::Transform => Pane::Map,
        }
    }
}
