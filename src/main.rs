use std::sync::Arc;

use rand::Rng;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::AutoCommandBufferBuilder,
    descriptor::{descriptor_set::PersistentDescriptorSet, PipelineLayoutAbstract},
    device::{Device, DeviceExtensions, Features},
    image::ImageUsage,
    instance::{Instance, PhysicalDevice},
    pipeline::ComputePipeline,
    swapchain::{self, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain},
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// mod mandelbrot {
//     vulkano_shaders::shader! {
//         ty: "compute",
//         path: "shaders/mandelbrot.glsl",
//     }
// }

pub mod motion {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/slime/motion.glsl",
    }
}

pub mod trail_decay {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/slime/trail_decay.glsl",
    }
}

const AGENTS: usize = 100_000;
const AGENT_MOVE_SPEED: f32 = 1.25;
const AGENT_SENSOR_SIZE: i32 = 2;
const AGENT_SENSOR_DISTANCE: f32 = 5.0;
const AGENT_SENSOR_ANGLE: f32 = std::f32::consts::PI / 4.0;
const AGENT_TURN_RATE: f32 = std::f32::consts::PI / 4.0;

const TRAIL_DECAY_RATE: f32 = 0.02;
const TRAIL_BLUR_RATE: f32 = 0.4;

fn main() {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no devices!");
    println!(
        "Using device: {} (type: {:?})",
        physical.name(),
        physical.ty()
    );

    // Window stuff

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("Cool Rust Program")
        .with_inner_size(LogicalSize::new(1920, 1080))
        // .with_maximized(true)
        .with_fullscreen(None)
        .with_decorations(false)
        .build_vk_surface(&event_loop, instance.clone())
        .expect("failed to create window surface");

    let queue_family = physical
        .queue_families()
        .find(|q| q.supports_graphics() && surface.is_supported(*q).unwrap_or(false))
        .expect("no graphical queue family");

    let (device, mut queues) = Device::new(
        physical,
        &Features::none(),
        &DeviceExtensions {
            khr_swapchain: true,
            khr_storage_buffer_storage_class: true,
            ..DeviceExtensions::none()
        },
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("failed to create device");

    let queue = queues.next().unwrap();

    let (mut swapchain, mut images) = {
        let caps = surface.capabilities(physical).unwrap();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();

        if !caps.supported_usage_flags.transfer_destination {
            panic!("window surface doesn't support being transfer destination");
        }

        println!(
            "Creating swapchain: alpha: {:?}, format: {:?}, dimensions: {:?}",
            alpha, format, dimensions
        );
        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage {
                transfer_destination: true,
                storage: true,
                ..ImageUsage::color_attachment()
            },
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .expect("failed to create swapchain")
    };

    let motion_shader = motion::Shader::load(device.clone()).unwrap();
    let motion_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &motion_shader.main_entry_point(), &(), None).unwrap(),
    );

    let trail_decay_shader = trail_decay::Shader::load(device.clone()).unwrap();
    let trail_decay_pipeline = Arc::new(
        ComputePipeline::new(
            device.clone(),
            &trail_decay_shader.main_entry_point(),
            &(),
            None,
        )
        .unwrap(),
    );

    let mut rng = rand::thread_rng();
    let dims = surface.window().inner_size();
    let max_radius = dims.width.min(dims.height) as f32 * 0.7 / 2.0;
    let agents = (0..AGENTS).map(|_| {
        let r = max_radius * rng.gen::<f32>().sqrt();
        let theta = rng.gen::<f32>() * std::f32::consts::TAU;
        motion::ty::Agent {
            position: [
                dims.width as f32 / 2.0 + (r * theta.cos()),
                dims.height as f32 / 2.0 + (r * theta.sin()),
            ],
            angle: std::f32::consts::PI - theta, // point back towards the center
            age: 0.0,
        }
    });
    let agent_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, agents).unwrap();

    let motion_layout1 = motion_pipeline.layout().descriptor_set_layout(1).unwrap();
    let motion_set1 = Arc::new(
        PersistentDescriptorSet::start(motion_layout1.clone())
            .add_buffer(agent_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());
    // let mut frame_timer = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }

        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => match input.virtual_keycode {
            Some(winit::event::VirtualKeyCode::Escape) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        },

        Event::MainEventsCleared => {}

        Event::RedrawEventsCleared => {
            // println!("frame: {:?}", std::time::Instant::now() - frame_timer);
            // frame_timer = std::time::Instant::now();

            previous_frame_end.as_mut().unwrap().cleanup_finished();

            let dimensions: [u32; 2] = surface.window().inner_size().into();
            if recreate_swapchain {
                let (new_swapchain, new_images) =
                    match swapchain.recreate_with_dimensions(dimensions) {
                        Ok(r) => r,
                        Err(swapchain::SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(e) => panic!("failed to recreate swapchain: {:?}", e),
                    };

                swapchain = new_swapchain;
                images = new_images;
                recreate_swapchain = false;
            }

            let (image_num, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(swapchain::AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            let mut builder =
                AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                    .unwrap();

            let motion_layout0 = motion_pipeline.layout().descriptor_set_layout(0).unwrap();
            let motion_set0 = Arc::new(
                PersistentDescriptorSet::start(motion_layout0.clone())
                    .add_image(images[image_num].clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            );

            let trail_decay_layout0 = trail_decay_pipeline
                .layout()
                .descriptor_set_layout(0)
                .unwrap();
            let trail_decay_set0 = Arc::new(
                PersistentDescriptorSet::start(trail_decay_layout0.clone())
                    .add_image(images[image_num].clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            );

            builder
                .dispatch(
                    [agent_buffer.len() as u32 / 1, 1, 1],
                    motion_pipeline.clone(),
                    (motion_set0.clone(), motion_set1.clone()),
                    motion::ty::Constants {
                        move_speed: AGENT_MOVE_SPEED,
                        sensor_distance: AGENT_SENSOR_DISTANCE,
                        sensor_size: AGENT_SENSOR_SIZE,
                        sensor_angle: AGENT_SENSOR_ANGLE,
                        turn_speed: AGENT_TURN_RATE,
                    },
                    vec![],
                )
                .unwrap()
                .dispatch(
                    [dimensions[0], dimensions[1], 1],
                    trail_decay_pipeline.clone(),
                    trail_decay_set0.clone(),
                    trail_decay::ty::Constants {
                        decay_speed: TRAIL_DECAY_RATE,
                        diffuse_speed: TRAIL_BLUR_RATE,
                    },
                    vec![],
                )
                .unwrap();

            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
            }
        }

        _ => {}
    });
}
