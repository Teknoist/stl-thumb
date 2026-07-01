extern crate cgmath;
#[macro_use]
extern crate glium;
extern crate image;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mint;

pub mod config;
mod fxaa;
mod mesh;

use cgmath::EuclideanSpace;
use config::{AAMethod, Config};
use glium::backend::Facade;
use glium::glutin::dpi::PhysicalSize;
use glium::glutin::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use glium::{glutin, CapabilitiesSource, Surface};
use image::{ImageEncoder, ImageFormat};
use libc::c_char;
use mesh::Mesh;
use std::error::Error;
use std::ffi::CStr;
use std::{io, panic, slice, thread, time};

#[cfg(target_os = "windows")]
use std::sync::{mpsc, OnceLock};

#[cfg(target_os = "linux")]
use std::env;

// TODO: Move this stuff to config module
const CAM_FOV_DEG: f32 = 30.0;
const CAM_POSITION: cgmath::Point3<f32> = cgmath::Point3 {
    x: 2.0,
    y: -4.0,
    z: 2.0,
};

fn print_matrix(m: [[f32; 4]; 4]) {
    for row in &m {
        debug!("{:.3}\t{:.3}\t{:.3}\t{:.3}", row[0], row[1], row[2], row[3]);
    }
    debug!("");
}

fn print_context_info(display: &glium::backend::Context) {
    // Print context information
    info!("GL Version:   {:?}", display.get_opengl_version());
    info!("GL Version:   {}", display.get_opengl_version_string());
    info!("GLSL Version: {:?}", display.get_supported_glsl_version());
    info!("Vendor:       {}", display.get_opengl_vendor_string());
    info!("Renderer      {}", display.get_opengl_renderer_string());
    info!("Free GPU Mem: {:?}", display.get_free_video_memory());
    info!(
        "Depth Bits:   {:?}\n",
        display.get_capabilities().depth_bits
    );
}

fn create_normal_display(
    config: &Config,
) -> Result<(glium::Display, EventLoop<()>), Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let display = create_normal_display_with_event_loop(config, &event_loop)?;
    Ok((display, event_loop))
}

fn create_normal_display_with_event_loop(
    config: &Config,
    event_loop: &EventLoop<()>,
) -> Result<glium::Display, Box<dyn Error>> {
    let window_dim = PhysicalSize::new(config.width, config.height);
    let window = glutin::window::WindowBuilder::new()
        .with_title("stl-thumb")
        .with_inner_size(window_dim)
        .with_min_inner_size(window_dim)
        .with_max_inner_size(window_dim)
        .with_visible(config.visible);
    let cb = glutin::ContextBuilder::new().with_depth_buffer(24);
    //.with_multisampling(8);
    //.with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGlEs, (2, 0)));
    let display = glium::Display::new(window, cb, event_loop)?;
    print_context_info(&display);
    Ok(display)
}

#[cfg(target_os = "windows")]
fn create_headless_display(
    config: &Config,
    event_loop: &EventLoop<()>,
) -> Result<glium::HeadlessRenderer, Box<dyn Error>> {
    let size = PhysicalSize::new(config.width, config.height);
    let cb = glutin::ContextBuilder::new();
    let context = cb.build_headless(event_loop, size)?;

    let context = unsafe { context.treat_as_current() };
    let display = glium::backend::glutin::headless::Headless::new(context)?;
    print_context_info(&display);
    Ok(display)
}

#[cfg(target_os = "linux")]
fn create_headless_display(config: &Config) -> Result<glium::HeadlessRenderer, Box<dyn Error>> {
    use glium::glutin::platform::unix::{EventLoopBuilderExtUnix, HeadlessContextExt};

    let size = PhysicalSize::new(config.width, config.height);
    let cb = glutin::ContextBuilder::new();
    let context: glium::glutin::Context<glium::glutin::NotCurrent>;

    // Linux requires an elaborate chain of attempts and fallbacks to find the ideal type of opengl context.

    // If there is no X server or Wayland, creating the event loop will fail first.
    // If this happens we catch the panic and fall back to osmesa software rendering, which doesn't require an event loop.
    // TODO: Submit PR upstream to stop panicing
    let event_loop_result: Result<EventLoop<()>, _> =
        panic::catch_unwind(|| EventLoopBuilder::new().with_any_thread(true).build());

    match event_loop_result {
        Ok(event_loop) => {
            context = {
                // Try surfaceless, headless, and osmesa in that order
                // This is the procedure recommended in
                // https://github.com/rust-windowing/glutin/blob/bab33a84dfb094ff65c059400bed7993434638e2/glutin_examples/examples/headless.rs
                match cb.clone().build_surfaceless(&event_loop) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Unable to create surfaceless GL context. Trying headless instead. Reason: {:?}", e);
                        match cb.clone().build_headless(&event_loop, size) {
                            Ok(c) => c,
                            Err(e) => {
                                warn!("Unable to create headless GL context. Trying osmesa software renderer instead. Reason: {:?}", e);
                                cb.build_osmesa(size)?
                            }
                        }
                    }
                }
            };
        }
        Err(e) => {
            warn!(
                "No Wayland or X server. Falling back to osmesa software rendering. Reason {:?}",
                e
            );
            context = cb.build_osmesa(size)?;
        }
    };

    let context = unsafe { context.treat_as_current() };
    let display = glium::backend::glutin::headless::Headless::new(context)?;
    print_context_info(&display);
    Ok(display)
}

fn render_pipeline<F>(
    display: &F,
    config: &Config,
    mesh: &Mesh,
    framebuffer: &mut glium::framebuffer::SimpleFrameBuffer,
    texture: &glium::Texture2d,
) -> image::DynamicImage
where
    F: Facade,
{
    // Graphics Stuff
    // ==============

    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
    };

    // Load and compile shaders
    // ------------------------

    let vertex_shader_src = include_str!("shaders/model.vert");
    let pixel_shader_src = include_str!("shaders/model.frag");

    // TODO: Cache program binary
    let program = glium::Program::from_source(display, vertex_shader_src, pixel_shader_src, None);
    let program = match program {
        Ok(p) => p,
        Err(glium::CompilationError(err, _)) => {
            error!("{}", err);
            panic!("Compiling shaders");
        }
        Err(err) => panic!("{}", err),
    };

    // Send mesh data to GPU
    // ---------------------

    let vertex_buf = glium::VertexBuffer::new(display, &mesh.vertices).unwrap();
    let normal_buf = glium::VertexBuffer::new(display, &mesh.normals).unwrap();
    // Can use NoIndices here because STLs are dumb
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    // Setup uniforms
    // --------------

    // Transformation matrix (positions, scales and rotates model)
    let transform_matrix = mesh.scale_and_center();

    // View matrix (convert to positions relative to camera)
    let view_matrix = cgmath::Matrix4::look_at_rh(
        CAM_POSITION,
        cgmath::Point3::origin(),
        cgmath::Vector3::unit_z(),
    );
    debug!("View:");
    print_matrix(view_matrix.into());

    // Perspective matrix (give illusion of depth)
    let perspective_matrix = cgmath::perspective(
        cgmath::Deg(CAM_FOV_DEG),
        config.width as f32 / config.height as f32,
        0.1,
        1024.0,
    );
    debug!("Perspective:");
    print_matrix(perspective_matrix.into());

    // Direction of light source
    //let light_dir = [-1.4, 0.4, -0.7f32];
    let light_dir = [-1.1, 0.4, 1.0f32];

    let uniforms = uniform! {
        //model: Into::<[[f32; 4]; 4]>::into(transform_matrix),
        //view: Into::<[[f32; 4]; 4]>::into(view_matrix),
        modelview: Into::<[[f32; 4]; 4]>::into(view_matrix * transform_matrix),
        perspective: Into::<[[f32; 4]; 4]>::into(perspective_matrix),
        u_light: light_dir,
        ambient_color: config.material.ambient,
        diffuse_color: config.material.diffuse,
        specular_color: config.material.specular,
    };

    // Draw
    // ----

    // Create FXAA system
    let fxaa = fxaa::FxaaSystem::new(display);
    let fxaa_enable = matches!(config.aamethod, AAMethod::FXAA);

    fxaa::draw(&fxaa, framebuffer, fxaa_enable, |target| {
        // Fills background color and clears depth buffer
        target.clear_color_and_depth(config.background, 1.0);
        target
            .draw(
                (&vertex_buf, &normal_buf),
                indices,
                &program,
                &uniforms,
                &params,
            )
            .unwrap();
        // TODO: Shadows
    });

    // Convert Image
    // =============

    let pixels: glium::texture::RawImage2d<u8> = texture.read();
    let img = image::ImageBuffer::from_raw(config.width, config.height, pixels.data.into_owned())
        .unwrap();

    image::DynamicImage::ImageRgba8(img).flipv()
}

fn render_mesh_to_image<F>(
    display: &F,
    config: &Config,
    mesh: &Mesh,
) -> Result<image::DynamicImage, Box<dyn Error>>
where
    F: Facade,
{
    let texture = glium::Texture2d::empty(display, config.width, config.height)?;
    let depthtexture = glium::texture::DepthTexture2d::empty(display, config.width, config.height)?;
    let mut framebuffer =
        glium::framebuffer::SimpleFrameBuffer::with_depth_buffer(display, &texture, &depthtexture)?;
    Ok(render_pipeline(
        display,
        config,
        mesh,
        &mut framebuffer,
        &texture,
    ))
}

pub fn render_to_window(config: Config) -> Result<(), Box<dyn Error>> {
    // Get geometry from model file
    // ==========================
    let mesh = Mesh::load(&config.model_filename, config.recalc_normals)?;

    // Create GL context
    // =================
    let (display, event_loop) = create_normal_display(&config)?;

    let sleep_time = time::Duration::from_millis(10);

    let texture = glium::Texture2d::empty(&display, config.width, config.height).unwrap();
    let depthtexture =
        glium::texture::DepthTexture2d::empty(&display, config.width, config.height).unwrap();

    event_loop.run(move |ev, _, control_flow| {
        *control_flow =
            glutin::event_loop::ControlFlow::WaitUntil(std::time::Instant::now() + sleep_time);
        let mut framebuffer = glium::framebuffer::SimpleFrameBuffer::with_depth_buffer(
            &display,
            &texture,
            &depthtexture,
        )
        .unwrap();

        match ev {
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                return;
            }
            glutin::event::Event::NewEvents(glutin::event::StartCause::Init) => {
                render_pipeline(&display, &config, &mesh, &mut framebuffer, &texture);
            }
            _ => (),
        }

        let target = display.draw();
        target.blit_from_simple_framebuffer(
            &framebuffer,
            &glium::Rect {
                left: 0,
                bottom: 0,
                width: config.width,
                height: config.height,
            },
            &glium::BlitTarget {
                left: 0,
                bottom: 0,
                width: config.width as i32,
                height: config.height as i32,
            },
            glium::uniforms::MagnifySamplerFilter::Nearest,
        );
        target.finish().unwrap();
    });
}

#[cfg(target_os = "windows")]
struct WindowsRenderRequest {
    config: Config,
    response: mpsc::Sender<Result<image::DynamicImage, String>>,
}

#[cfg(target_os = "windows")]
static WINDOWS_RENDERER: OnceLock<mpsc::Sender<WindowsRenderRequest>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn render_to_image_with_event_loop(
    config: &Config,
    event_loop: &EventLoop<()>,
) -> Result<image::DynamicImage, Box<dyn Error>> {
    // Get geometry from model file
    // =========================
    let mesh = Mesh::load(&config.model_filename, config.recalc_normals)?;

    // Create GL context
    // =================
    // 1. If not visible create a headless context.
    // 2. If headless context creation fails, create a normal context with a hidden window.
    match create_headless_display(config, event_loop) {
        Ok(display) => render_mesh_to_image(&display, config, &mesh),
        Err(e) => {
            warn!(
                "Unable to create headless GL context. Trying hidden window instead. Reason: {:?}",
                e
            );
            let display = create_normal_display_with_event_loop(config, event_loop)?;
            render_mesh_to_image(&display, config, &mesh)
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_renderer() -> &'static mpsc::Sender<WindowsRenderRequest> {
    WINDOWS_RENDERER.get_or_init(|| {
        let (request_tx, request_rx) = mpsc::channel::<WindowsRenderRequest>();

        thread::Builder::new()
            .name("stl-thumb-renderer".to_string())
            .spawn(move || {
                use glium::glutin::platform::windows::EventLoopBuilderExtWindows;

                // winit permits only one EventLoop per process on Windows. Keep
                // it on a dedicated worker and serialize shell-extension calls.
                let event_loop: EventLoop<()> =
                    EventLoopBuilder::new().with_any_thread(true).build();

                for request in request_rx {
                    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                        render_to_image_with_event_loop(&request.config, &event_loop)
                    }))
                    .map_err(|_| "Renderer panicked while creating a thumbnail".to_string())
                    .and_then(|result| result.map_err(|error| error.to_string()));

                    let _ = request.response.send(result);
                }
            })
            .expect("Unable to start Windows render worker");

        request_tx
    })
}

#[cfg(target_os = "windows")]
fn render_to_image_on_windows(config: &Config) -> Result<image::DynamicImage, Box<dyn Error>> {
    let (response_tx, response_rx) = mpsc::channel();
    windows_renderer()
        .send(WindowsRenderRequest {
            config: config.clone(),
            response: response_tx,
        })
        .map_err(|_| io::Error::other("Windows render worker is unavailable"))?;

    response_rx
        .recv()
        .map_err(|_| io::Error::other("Windows render worker stopped unexpectedly"))?
        .map_err(|message| io::Error::other(message).into())
}

pub fn render_to_image(config: &Config) -> Result<image::DynamicImage, Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    {
        render_to_image_on_windows(config)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Get geometry from model file
        let mesh = Mesh::load(&config.model_filename, config.recalc_normals)?;

        let img = match create_headless_display(config) {
            Ok(display) => render_mesh_to_image(&display, config, &mesh)?,
            Err(e) => {
                warn!(
                    "Unable to create headless GL context. Trying hidden window instead. Reason: {:?}",
                    e
                );
                let (display, _) = create_normal_display(config)?;
                render_mesh_to_image(&display, config, &mesh)?
            }
        };

        Ok(img)
    }
}

pub fn render_to_file(config: &Config) -> Result<(), Box<dyn Error>> {
    let img = render_to_image(config)?;

    // write_to() requires a seekable writer for performance reasons.
    // So we create an in-memory buffer and then dump that to the output.
    // I wonder if it would be better to use std::io::BufWriter for writing files instead.
    let mut buff: Vec<u8> = Vec::new();
    let mut cursor = io::Cursor::new(&mut buff);

    // Encode image with specified format
    // If encoding a PNG image, use fastest compression method
    // Not sure if this is really necessary. Fast is the default anyways.
    match config.format {
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                &mut cursor,
                image::codecs::png::CompressionType::Fast,
                //image::codecs::png::CompressionType::Default,
                image::codecs::png::FilterType::Adaptive,
            );
            encoder.write_image(
                img.as_bytes(),
                config.width,
                config.height,
                img.color().into(),
            )?;
        }
        _ => img.write_to(&mut cursor, config.format.to_owned())?,
    }
    //img.write_to(&mut cursor, config.format.to_owned())?;

    // Do not create the destination until encoding succeeds. This avoids
    // leaving an empty file behind when an encoder rejects the image.
    let mut output: Box<dyn io::Write> = match config.img_filename.as_str() {
        "-" => Box::new(io::stdout()),
        _ => Box::new(std::fs::File::create(&config.img_filename)?),
    };

    output.write_all(&buff)?;
    output.flush()?;

    Ok(())
}

/// Allows utilizing `stl-thumb` from C-like languages
///
/// This function renders an image of the file `model_filename_c` and stores it into the buffer `buf_ptr`.
///
/// You must provide a memory buffer large enough to store the image. Images are written in 8-bit RGBA format,
/// so the buffer must be at least `width`*`height`*4 bytes in size. `model_filename_c` is a pointer to a C string with
/// the file path.
///
/// Returns `true` if succesful and `false` if unsuccesful.
///
/// # Example in C
/// ```c
/// const char* model_filename_c = "3DBenchy.stl";
/// int width = 256;
/// int height = 256;
///
/// int img_size = width * height * 4;
/// buf_ptr = (uchar *) malloc(img_size);
///
/// render_to_buffer(buf_ptr, width, height, model_filename_c);
/// ```
///
/// # Safety
///
/// * `buf_ptr` _must_ point to a valid initialized buffer, at least `width * height * 4` bytes long.
/// * `model_filename_c` must point to a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn render_to_buffer(
    buf_ptr: *mut u8,
    width: u32,
    height: u32,
    model_filename_c: *const c_char,
) -> bool {
    // Workaround for issues with OpenGL 3.1 on Mesa 18.3
    #[cfg(target_os = "linux")]
    env::set_var("MESA_GL_VERSION_OVERRIDE", "2.1");

    // Check that the buffer pointer is valid
    if buf_ptr.is_null() {
        error!("Image buffer pointer is null");
        return false;
    };
    if width == 0 || height == 0 {
        error!("Image dimensions must be greater than zero");
        return false;
    }
    let buf_size = match (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
    {
        Some(size) if size <= isize::MAX as usize => size,
        _ => {
            error!("Image dimensions are too large");
            return false;
        }
    };
    let buf = unsafe { slice::from_raw_parts_mut(buf_ptr, buf_size) };

    // Check validity of provided file path string
    let model_filename_cstr = unsafe {
        if model_filename_c.is_null() {
            error!("model file path pointer is null");
            return false;
        }
        CStr::from_ptr(model_filename_c)
    };
    let model_filename_str = match model_filename_cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            error!("Invalid model file path {:?}", model_filename_cstr);
            return false;
        }
    };

    // Setup configuration for the renderer
    let config = Config {
        model_filename: model_filename_str.to_string(),
        width,
        height,
        ..Default::default()
    };

    // Render

    // Run renderer in seperate thread so OpenGL problems do not crash caller
    let render_thread =
        thread::spawn(move || render_to_image(&config).map_err(|error| error.to_string()));

    let img = match render_thread.join() {
        Ok(Ok(image)) => image,
        Ok(Err(e)) => {
            error!("Application error: {}", e);
            return false;
        }
        Err(e) => {
            error!("Application error: {:?}", e);
            return false;
        }
    };

    // Copy image to output buffer
    match img.as_rgba8() {
        Some(s) => buf.copy_from_slice(s),
        None => {
            error!("Unable to get image");
            return false;
        }
    }

    true
}

// TODO: Move tests to their own file
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::ErrorKind;

    #[test]
    fn cube_stl() {
        let img_filename = "cube-stl.png".to_string();
        let config = Config {
            model_filename: "test_data/cube.stl".to_string(),
            img_filename: img_filename.clone(),
            format: image::ImageFormat::Png,
            ..Default::default()
        };

        match fs::remove_file(&img_filename) {
            Ok(_) => (),
            Err(ref error) if error.kind() == ErrorKind::NotFound => (),
            Err(_) => {
                panic!("Couldn't clean files before testing");
            }
        }

        render_to_file(&config).expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }

    #[test]
    fn cube_obj() {
        let img_filename = "cube-obj.png".to_string();
        let config = Config {
            model_filename: "test_data/cube.obj".to_string(),
            img_filename: img_filename.clone(),
            format: image::ImageFormat::Png,
            ..Default::default()
        };

        match fs::remove_file(&img_filename) {
            Ok(_) => (),
            Err(ref error) if error.kind() == ErrorKind::NotFound => (),
            Err(_) => {
                panic!("Couldn't clean files before testing");
            }
        }

        render_to_file(&config).expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }

    #[test]
    fn cube_3mf() {
        let img_filename = "cube-3mf.png".to_string();
        let config = Config {
            model_filename: "test_data/cube.3mf".to_string(),
            img_filename: img_filename.clone(),
            format: image::ImageFormat::Png,
            ..Default::default()
        };

        match fs::remove_file(&img_filename) {
            Ok(_) => (),
            Err(ref error) if error.kind() == ErrorKind::NotFound => (),
            Err(_) => {
                panic!("Couldn't clean files before testing");
            }
        }

        render_to_file(&config).expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }
}
