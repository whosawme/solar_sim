use ggez::{Context, GameResult};
use ggez::graphics::{self, Color, DrawParam, Mesh, Text};
use ggez::event::{self, EventHandler};
use ggez::input::{keyboard::{KeyCode, KeyInput}, mouse::MouseButton};
use ggez::mint::Point2;
use rand::Rng;
use std::f32::consts::PI;

const WINDOW_WIDTH: f32 = 1600.0;
const WINDOW_HEIGHT: f32 = 1200.0;
const G: f32 = 1.0;
const DT: f32 = 0.016;

struct Button {
    rect: graphics::Rect,
    text: String,
    clicked: bool,
}

impl Button {
    fn new(x: f32, y: f32, w: f32, h: f32, text: &str) -> Self {
        Button {
            rect: graphics::Rect::new(x, y, w, h),
            text: text.to_string(),
            clicked: false,
        }
    }

    fn contains(&self, point: Point2<f32>) -> bool {
        self.rect.contains(point)
    }

    fn draw(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult {
        let rect = Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.rect,
            if self.clicked { Color::BLUE } else { Color::from_rgb(100, 100, 100) },
        )?;
        canvas.draw(&rect, DrawParam::default());
        
        let text = Text::new(&self.text);
        let text_pos = Point2 {
            x: self.rect.x + 10.0,
            y: self.rect.y + 5.0,
        };
        canvas.draw(&text, DrawParam::default().dest(text_pos).color(Color::WHITE));
        Ok(())
    }
}

struct Slider {
    value: f32,
    min: f32,
    max: f32,
    label: String,
    y_pos: f32,
    text_input: Option<String>,
}

impl Slider {
    fn new(value: f32, min: f32, max: f32, label: &str, y_pos: f32, text_input: bool) -> Self {
        Slider {
            value,
            min,
            max,
            label: label.to_string(),
            y_pos,
            text_input: if text_input { Some(String::new()) } else { None },
        }
    }

    fn handle_click(&mut self, x: f32, y: f32) -> bool {
        if y >= self.y_pos && y <= self.y_pos + 20.0 && x >= 150.0 && x <= 350.0 {
            self.value = self.min + (self.max - self.min) * ((x - 150.0) / 200.0);
            true
        } else {
            false
        }
    }

    fn draw(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult {
        let text = Text::new(&self.label);
        canvas.draw(&text, DrawParam::default().dest([10.0, self.y_pos]).color(Color::WHITE));

        let slider_bg = Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            graphics::Rect::new(150.0, self.y_pos, 200.0, 20.0),
            Color::from_rgb(50, 50, 50),
        )?;
        canvas.draw(&slider_bg, DrawParam::default());

        let position = 150.0 + 200.0 * (self.value - self.min) / (self.max - self.min);
        let slider_handle = Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            Point2 { x: position, y: self.y_pos + 10.0 },
            10.0,
            0.1,
            Color::WHITE,
        )?;
        canvas.draw(&slider_handle, DrawParam::default());

        // Display value
        let value_text = if self.value >= 1000.0 {
            format!("{:.1e}", self.value)
        } else {
            format!("{:.2}", self.value)
        };
        let value_display = Text::new(&value_text);
        canvas.draw(&value_display, DrawParam::default().dest([360.0, self.y_pos]).color(Color::WHITE));

        // Text input for particle count
        if let Some(text_input) = &self.text_input {
            let input_bg = Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                graphics::Rect::new(420.0, self.y_pos, 60.0, 20.0),
                Color::from_rgb(30, 30, 30),
            )?;
            canvas.draw(&input_bg, DrawParam::default());
            let input_text = Text::new(text_input);
            canvas.draw(&input_text, DrawParam::default().dest([425.0, self.y_pos]).color(Color::WHITE));
        }

        Ok(())
    }
}

#[derive(Clone)]
struct Particle {
    position: Point2<f32>,
    velocity: Point2<f32>,
    acceleration: Point2<f32>,
    mass: f32,
    radius: f32,
}

impl Particle {
    fn new(x: f32, y: f32, mass: f32) -> Self {
        Particle {
            position: Point2 { x, y },
            velocity: Point2 { x: 0.0, y: 0.0 },
            acceleration: Point2 { x: 0.0, y: 0.0 },
            mass,
            radius: mass.powf(0.3).max(2.0),
        }
    }

    fn calculate_acceleration(&mut self, particles: &[Particle]) {
        self.acceleration = Point2 { x: 0.0, y: 0.0 };
        
        for other in particles {
            if std::ptr::eq(self, other) {
                continue;
            }

            let dx = other.position.x - self.position.x;
            let dy = other.position.y - self.position.y;
            let softening = particles[0].mass.log10();
            let dist_squared = dx * dx + dy * dy + softening;
            let dist = dist_squared.sqrt();

            if dist < self.radius + other.radius {
                continue;
            }

            let force = G * other.mass / dist_squared;
            
            self.acceleration.x += force * dx / dist;
            self.acceleration.y += force * dy / dist;
        }
    }

    fn update(&mut self, dt: f32, particles: &[Particle]) {
        // First half-kick
        self.velocity.x += self.acceleration.x * dt * 0.5;
        self.velocity.y += self.acceleration.y * dt * 0.5;
        
        // Drift
        self.position.x += self.velocity.x * dt;
        self.position.y += self.velocity.y * dt;
        
        // Update accelerations
        self.calculate_acceleration(particles);
        
        // Second half-kick
        self.velocity.x += self.acceleration.x * dt * 0.5;
        self.velocity.y += self.acceleration.y * dt * 0.5;
    }
}

struct SimulationState {
    particles: Vec<Particle>,
    particle_count: usize,
    initial_mass_range: (f32, f32),
    initial_velocity_multiplier: f32,
    paused: bool,
    zoom: f32,
    pan: Point2<f32>,
    buttons: Vec<Button>,
    sliders: Vec<Slider>,
    is_panning: bool,
    last_mouse_pos: Point2<f32>,
    adding_mass: bool,
    mass_preview: Option<Point2<f32>>,
}

impl SimulationState {
    fn new() -> Self {
        let mut state = SimulationState {
            particles: Vec::new(),
            particle_count: 100,
            initial_mass_range: (1.0, 5.0),
            initial_velocity_multiplier: 1.0,
            paused: true,
            zoom: 1.0,
            pan: Point2 { x: 0.0, y: 0.0 },
            buttons: vec![
                Button::new(10.0, 10.0, 100.0, 30.0, "Run/Pause"),
                Button::new(120.0, 10.0, 100.0, 30.0, "Reset"),
                Button::new(230.0, 10.0, 100.0, 30.0, "Add Mass"),
            ],
            sliders: vec![
                Slider::new(1.0, 0.1, 10.0, "Time Speed", 50.0, false),
                Slider::new(100.0, 10.0, 1000.0, "Particles", 90.0, true),
                Slider::new(1.0, 0.1, 5.0, "Velocity", 130.0, false),
                Slider::new(3.0, 0.1, 100.0, "Mass", 170.0, false),
                Slider::new(1.0, 0.1, 10.0, "Softening", 210.0, false),
                Slider::new(0.016, 0.001, 0.1, "Time Step", 250.0, false),
                Slider::new(1000.0, 100.0, 5000.0, "Central Mass", 290.0, false),
            ],
            is_panning: false,
            last_mouse_pos: Point2 { x: 0.0, y: 0.0 },
            adding_mass: false,
            mass_preview: None,
        };
        state.reset();
        state
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.particles.clear();

        self.particles.push(Particle::new(
            WINDOW_WIDTH / 2.0,
            WINDOW_HEIGHT / 2.0,
            self.sliders[6].value,
        ));

        for _ in 0..self.particle_count {
            let distance = rng.gen_range(100.0..300.0);
            let angle = rng.gen_range(0.0..2.0 * PI);
            let x = WINDOW_WIDTH / 2.0 + distance * angle.cos();
            let y = WINDOW_HEIGHT / 2.0 + distance * angle.sin();
            
            let mut particle = Particle::new(
                x,
                y,
                rng.gen_range(self.initial_mass_range.0..self.initial_mass_range.1),
            );

            let orbital_speed = (G * self.particles[0].mass / distance).sqrt() * self.initial_velocity_multiplier;
            particle.velocity = Point2 {
                x: -orbital_speed * angle.sin(),
                y: orbital_speed * angle.cos(),
            };

            self.particles.push(particle);
        }
    }

    fn add_large_mass(&mut self, x: f32, y: f32) {
        let mass = self.sliders[3].value * 100.0;
        self.particles.push(Particle::new(x, y, mass));
    }

    fn handle_mouse_click(&mut self, x: f32, y: f32) {
        let mouse_pos = Point2 { x, y };
        
        // Handle UI elements first
        let mut clicked_reset = false;
        let mut should_pause = false;
        let mut start_add_mass = false;
        
        // Only handle UI if not in mass-adding mode
        if !self.adding_mass {
        for button in &mut self.buttons {
            if button.contains(mouse_pos) {
                button.clicked = true;
                match button.text.as_str() {
                    "Run/Pause" => should_pause = true,
                    "Reset" => clicked_reset = true,
                        "Add Mass" => start_add_mass = true,
                        _ => (),
                    }
                }
            }

            for slider in &mut self.sliders {
                if slider.handle_click(x, y) {
                    match slider.label.as_str() {
                        "Particles" => self.particle_count = slider.value as usize,
                        "Velocity" => self.initial_velocity_multiplier = slider.value,
                        "Mass" => self.initial_mass_range = (slider.value * 0.5, slider.value * 1.5),
                    _ => (),
                }
                    return;
                }
            }
        }
        
        if should_pause {
            self.paused = !self.paused;
        }
        if clicked_reset {
            self.reset();
        }
        if start_add_mass {
            self.adding_mass = true;
            return;
        }

        // Handle mass placement or panning
        if self.adding_mass {
            if y > 50.0 { // Don't add mass in UI area
                self.add_large_mass(x, y);
                self.adding_mass = false;
                self.mass_preview = None;
            }
        } else {
            // Start panning if not clicking UI
            if y > 50.0 {
                self.is_panning = true;
                self.last_mouse_pos = mouse_pos;
            }
        }
    }

    fn handle_mouse_motion(&mut self, x: f32, y: f32) {
        let current_pos = Point2 { x, y };
        
        if self.is_panning {
            self.pan.x += (current_pos.x - self.last_mouse_pos.x) / self.zoom;
            self.pan.y += (current_pos.y - self.last_mouse_pos.y) / self.zoom;
            self.last_mouse_pos = current_pos;
        }

        if self.adding_mass {
            self.mass_preview = Some(current_pos);
        }
    }

    fn handle_mouse_release(&mut self) {
        for button in &mut self.buttons {
            button.clicked = false;
        }
        self.is_panning = false;
    }
}

impl EventHandler for SimulationState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        if !self.paused {
            let time_speed = self.sliders[0].value;
            let dt = DT * time_speed;
            let particles_snapshot = self.particles.clone();
            for particle in &mut self.particles {
                particle.update(dt, &particles_snapshot);
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);

        // Draw particles
        for particle in &self.particles {
            let pos = Point2 {
                x: (particle.position.x + self.pan.x) * self.zoom,
                y: (particle.position.y + self.pan.y) * self.zoom,
            };
            let circle = Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                pos,
                particle.radius * self.zoom,
                0.1,
                Color::WHITE,
            )?;
            canvas.draw(&circle, DrawParam::default());
        }

        // Draw mass preview
        if self.adding_mass {
            if let Some(pos) = self.mass_preview {
                let preview_circle = Mesh::new_circle(
                    ctx,
                    graphics::DrawMode::stroke(2.0),
                    pos,
                    (self.sliders[3].value * 0.3).max(2.0),
                    0.1,
                    Color::YELLOW,
                )?;
                canvas.draw(&preview_circle, DrawParam::default());
            }
        }

        // Draw UI elements
        for button in &self.buttons {
            button.draw(ctx, &mut canvas)?;
        }

        for slider in &self.sliders {
            slider.draw(ctx, &mut canvas)?;
        }

        // Draw mode indicator
        let mode_text = if self.adding_mass {
            "Click to place mass"
        } else {
            "Click and drag to pan"
        };
        let text = Text::new(mode_text);
        canvas.draw(&text, DrawParam::default().dest([500.0, 15.0]).color(Color::WHITE));

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        self.handle_mouse_motion(x, y);
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if button == MouseButton::Left {
            self.handle_mouse_click(x, y);
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> GameResult {
        if button == MouseButton::Left {
            self.handle_mouse_release();
        }
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) -> GameResult {
        self.zoom *= if y > 0.0 { 1.1 } else { 0.9 };
        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        match input.keycode {
            Some(KeyCode::Space) => self.paused = !self.paused,
            Some(KeyCode::R) => self.reset(),
            Some(KeyCode::W) => self.pan.y += 10.0 / self.zoom,
            Some(KeyCode::S) => self.pan.y -= 10.0 / self.zoom,
            Some(KeyCode::A) => self.pan.x += 10.0 / self.zoom,
            Some(KeyCode::D) => self.pan.x -= 10.0 / self.zoom,
            _ => (),
        }
        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> GameResult {
        if let Some(text_input) = &mut self.sliders[1].text_input {
            if character.is_numeric() || character == '\x08' {
                if character == '\x08' {
                    text_input.pop();
                } else {
                    text_input.push(character);
                }
                if let Ok(value) = text_input.parse::<f32>() {
                    if value >= self.sliders[1].min && value <= self.sliders[1].max {
                        self.sliders[1].value = value;
                        self.particle_count = value as usize;
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("solar_system", "user")
        .window_setup(ggez::conf::WindowSetup::default().title("Solar System Formation Simulator"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(WINDOW_WIDTH, WINDOW_HEIGHT));
    
    let (ctx, event_loop) = cb.build()?;
    let state = SimulationState::new();
    
    event::run(ctx, event_loop, state)
}