use crate::{VisualizationData, Visualizer};
use kiss3d::{
    camera::ArcBall,
    event::{Key, WindowEvent},
    light::Light,
    nalgebra::{Point3, Translation3},
    window::Window,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct ThreeDVisualizer {
    sender: Option<Sender<VisualizationData>>,
    handle: Option<thread::JoinHandle<()>>,
    _running: bool,
}

impl ThreeDVisualizer {
    pub fn new() -> Self {
        Self {
            sender: None,
            handle: None,
            _running: false,
        }
    }
    
    pub fn start(&mut self) {
        let (tx, rx): (Sender<VisualizationData>, Receiver<VisualizationData>) = mpsc::channel();
        self.sender = Some(tx);
        self._running = true;
        
        let handle = thread::spawn(move || {
            run_3d_window(rx);
        });
        
        self.handle = Some(handle);
    }
    
    pub fn stop(&mut self) {
        self._running = false;
    }
}

impl Default for ThreeDVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for ThreeDVisualizer {
    fn name(&self) -> &str {
        "3DVisualizer"
    }
    
    fn update(&mut self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sender) = &self.sender {
            sender.send(data.clone())?;
        }
        Ok(())
    }
    
    fn save_snapshot(&self, _filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("3D snapshot not implemented yet");
        Ok(())
    }
}

fn run_3d_window(rx: Receiver<VisualizationData>) {
    let mut window = Window::new("Cell DT - 3D Visualization");
    window.set_light(Light::StickToCamera);
    
    let mut camera = ArcBall::new(
        Point3::new(10.0, 10.0, 10.0),
        Point3::new(0.0, 0.0, 0.0),
    );
    
    let mut spheres = Vec::new();
    
    while window.render_with_camera(&mut camera) {
        if let Ok(data) = rx.try_recv() {
            update_visualization(&mut window, &mut spheres, &data);
        }
        
        for event in window.events().iter() {
            match event.value {
                WindowEvent::Key(Key::Escape, ..) => return,
                WindowEvent::Key(Key::R, ..) => reset_camera(&mut camera),
                _ => {}
            }
        }
    }
}

fn update_visualization(window: &mut Window, spheres: &mut Vec<kiss3d::scene::SceneNode>, data: &VisualizationData) {
    for sphere in spheres.iter_mut() {
        window.remove_node(sphere);
    }
    spheres.clear();
    
    let cell_count = data.cell_count.min(1000);
    
    for i in 0..cell_count {
        let phi = (i as f32) * 2.0 * std::f32::consts::PI / (cell_count as f32).sqrt();
        let theta = (i as f32) * std::f32::consts::PI / (cell_count as f32).sqrt();
        
        let x = (theta.sin() * phi.cos()) * 5.0;
        let y = (theta.sin() * phi.sin()) * 5.0;
        let z = theta.cos() * 5.0;
        
        let mut sphere = window.add_sphere(0.2);
        sphere.set_local_translation(Translation3::new(x, y, z));
        
        let phase_index = i % 4;
        let color = match phase_index {
            0 => Point3::new(0.0, 1.0, 0.0),
            1 => Point3::new(1.0, 1.0, 0.0),
            2 => Point3::new(1.0, 0.5, 0.0),
            _ => Point3::new(1.0, 0.0, 0.0),
        };
        
        sphere.set_color(color.x, color.y, color.z);
        spheres.push(sphere);
    }
}

fn reset_camera(_camera: &mut ArcBall) {
    // Сброс камеры в исходное положение
}
