use anyhow::Result;
use opencv::{core::Mat, highgui};
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    pub(crate) static ref VISUALIZER_ENABLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub(crate) static ref CURRENT_FRAME: Arc<Mutex<Mat>> = Arc::new(Mutex::new(Mat::default()));
}

#[allow(dead_code)]
mod visualizations {
    use anyhow::Result;
    use opencv::{
        core::{Mat, Point, Scalar},
        imgproc::{put_text, HersheyFonts, LineTypes},
    };
    use std::time::SystemTime;

    pub struct FPSCounter {
        frame_times: Vec<f64>,
        last_frame_time: SystemTime,
    }

    impl FPSCounter {
        pub fn new() -> Self {
            Self {
                frame_times: vec![],
                last_frame_time: SystemTime::now(),
            }
        }

        pub fn tick(&mut self) -> f64 {
            let now = SystemTime::now();
            self.frame_times
                .push(1.0 / self.last_frame_time.elapsed().unwrap().as_secs_f64());

            let fps = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;

            self.last_frame_time = now;
            if self.frame_times.len() == 100 {
                self.frame_times.remove(0);
            }

            fps
        }
    }

    pub fn hud(fps: f64, frame: &mut Mat) -> Result<()> {
        put_text(
            frame,
            &format!("FPS: {:.1}", fps),
            Point::new(0, 50),
            HersheyFonts::FONT_HERSHEY_SIMPLEX as i32,
            2.0,
            Scalar::new(0.0, 0.0, 255.0, 0.0),
            5,
            LineTypes::LINE_4 as i32,
            false,
        )?;
        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) async fn run_visualizer() -> Result<()> {
    opencv::highgui::named_window(
        "Gathering the Magic - Camera",
        opencv::highgui::WINDOW_AUTOSIZE,
    )
    .unwrap();

    let mut fps_counter = visualizations::FPSCounter::new();

    {
        *VISUALIZER_ENABLED.lock().await = true;
    }

    let mut frame = Mat::default();
    loop {
        if let Ok(global_frame) = CURRENT_FRAME.try_lock() {
            frame = global_frame.clone()
        }

        visualizations::hud(fps_counter.tick(), &mut frame)?;
        let _ = highgui::imshow("Gathering the Magic - Camera", &frame);

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            break;
        }
    }

    Ok(())
}
