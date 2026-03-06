use wlx_monitors::WlTransform;

pub const TRANSFORMS: [WlTransform; 8] = [
    WlTransform::Normal,
    WlTransform::Rotate90,
    WlTransform::Rotate180,
    WlTransform::Rotate270,
    WlTransform::Flipped,
    WlTransform::Flipped90,
    WlTransform::Flipped180,
    WlTransform::Flipped270,
];

pub const REPEAT_WINDOW_MS: u128 = 200;
