use anyhow::Result;
use opencv::{
    core::Vector,
    imgcodecs::{imdecode, IMREAD_COLOR},
};

pub(crate) fn process_frame(frame_data: &[u8]) -> Result<()> {
    let frame = imdecode(&Vector::from_slice(frame_data), IMREAD_COLOR)?;

    // Send the frame to the visualizer, if the visualizer is enabled
    if let (Ok(visualizer), Ok(mut global_frame)) = (
        crate::image::VISUALIZER_ENABLED.try_lock(),
        crate::image::CURRENT_FRAME.try_lock(),
    ) {
        if *visualizer {
            *global_frame = frame;
        }
    }
    Ok(())
}
