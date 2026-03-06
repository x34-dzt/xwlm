pub mod modes;
pub mod monitor_map;
pub mod scale;
pub mod transform;
pub mod workspace;

#[derive(Debug, Default)]
pub enum Pane {
    #[default]
    Map,
    Mode,
    Scale,
    Transform,
    Worksapce,
}
