use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ptr;
use std::time::Duration;

use ab_glyph::FontArc;
use glyph_brush::{
    BrushAction, BrushError, GlyphBrush, GlyphBrushBuilder, GlyphVertex, Rectangle, Section, Text,
};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{PixelFormat, PixelFormatEnum};
use sdl2::render::{BlendMode, Texture, TextureCreator, TextureValueError, WindowCanvas};
use sdl2::sys;

#[derive(Clone, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

struct LazyTexture {
    raw_data: Vec<u8>,
    tex_dims: (u32, u32),
    internal_update: fn(&sdl2::rect::Rect, &mut Texture, &Vec<u8>, &Color),
}

impl Debug for LazyTexture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let raw_data = format!("{:?}", self.raw_data.len());
        let tex_dims = format!("{:?}", self.tex_dims);
        write!(
            f,
            "LazyTexture {{ raw_data: {}, tex_dims: {{ {} }} }}",
            raw_data, tex_dims
        )
    }
}

#[derive(Debug)]
struct SizeMismatch {}

impl Display for SizeMismatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Out of Bounds")
    }
}

impl Error for SizeMismatch {}

impl LazyTexture {
    fn new_empty() -> Self {
        LazyTexture {
            raw_data: vec![],
            tex_dims: (0, 0),
            internal_update: backup_update_texture,
        }
    }

    fn resize(&mut self, dims: (u32, u32)) {
        self.tex_dims = dims;
        self.raw_data.resize((dims.0 * dims.1) as usize, 0);
        println!("Resize: {:?}", dims);
    }

    fn create_texture<'a, T>(
        &self,
        tex_creator: &'a TextureCreator<T>,
        color: &Color,
    ) -> Result<Texture<'a>, TextureValueError> {
        println!("Materialize {:?}", &self.tex_dims);
        let mut texture = tex_creator.create_texture_static(
            PixelFormatEnum::RGBA32,
            self.tex_dims.0,
            self.tex_dims.1,
        )?;
        texture.set_blend_mode(BlendMode::Blend);
        (self.internal_update)(
            &sdl2::rect::Rect::new(0, 0, self.tex_dims.0, self.tex_dims.1),
            &mut texture,
            &self.raw_data,
            color,
        );
        Ok(texture)
    }

    fn lazy_update(&mut self, rect: Rectangle<u32>, tex_data: &[u8]) -> Result<(), SizeMismatch> {
        println!("update_texture() {:?} {:?}", &rect, tex_data.len());
        let mut data_iter = tex_data.iter();
        for (i, a) in self.raw_data.iter_mut().enumerate() {
            let y = i as u32 / self.tex_dims.1;
            if y >= rect.min[1] && y < rect.max[1] {
                let x = i as u32 % self.tex_dims.0;
                if x >= rect.min[0] && x < rect.max[0] {
                    *a = *data_iter.next().ok_or(SizeMismatch {})?;
                }
            }
        }
        if data_iter.next() != None {
            return Err(SizeMismatch {});
        }
        Ok(())
    }

    fn update(&self, texture: &mut Texture, color: &Color) {
        let rect = sdl2::rect::Rect::new(0, 0, self.tex_dims.0, self.tex_dims.1);
        (self.internal_update)(&rect, texture, &self.raw_data, color);
    }
}

#[derive(Clone)]
struct SDLPolygon {
    vers: Vec<sys::SDL_Vertex>,
    inds: Vec<i32>,
}

impl Debug for SDLPolygon {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let vers = self
            .vers
            .iter()
            .map(|v| format!("SDL_Vertex {{ x: {}, y: {} }}", v.position.x, v.position.y))
            .collect::<Vec<String>>()
            .join(", ");
        let inds = self
            .inds
            .iter()
            .map(|i| format!("{}", i))
            .collect::<Vec<String>>()
            .join(" ,");
        write!(f, "SDLPolygon {{ vers: [{}], inds: [{}] }}", vers, inds)
    }
}

fn into_vertex(vd: GlyphVertex) -> SDLPolygon {
    let alpha_for_all_vertices = 255;
    let v1 = sys::SDL_Vertex {
        position: sys::SDL_FPoint {
            x: vd.pixel_coords.min.x,
            y: vd.pixel_coords.min.y,
        },
        color: sys::SDL_Color {
            r: 255,
            g: 255,
            b: 255,
            a: alpha_for_all_vertices,
        },
        tex_coord: sys::SDL_FPoint {
            x: vd.tex_coords.min.x,
            y: vd.tex_coords.min.y,
        },
    };
    let v2 = sys::SDL_Vertex {
        position: sys::SDL_FPoint {
            x: vd.pixel_coords.min.x,
            y: vd.pixel_coords.max.y,
        },
        color: sys::SDL_Color {
            r: 255,
            g: 255,
            b: 255,
            a: alpha_for_all_vertices,
        },
        tex_coord: sys::SDL_FPoint {
            x: vd.tex_coords.min.x,
            y: vd.tex_coords.max.y,
        },
    };
    let v3 = sys::SDL_Vertex {
        position: sys::SDL_FPoint {
            x: vd.pixel_coords.max.x,
            y: vd.pixel_coords.max.y,
        },
        color: sys::SDL_Color {
            r: 255,
            g: 255,
            b: 255,
            a: alpha_for_all_vertices,
        },
        tex_coord: sys::SDL_FPoint {
            x: vd.tex_coords.max.x,
            y: vd.tex_coords.max.y,
        },
    };
    let v4 = sys::SDL_Vertex {
        position: sys::SDL_FPoint {
            x: vd.pixel_coords.max.x,
            y: vd.pixel_coords.min.y,
        },
        color: sys::SDL_Color {
            r: 255,
            g: 255,
            b: 255,
            a: alpha_for_all_vertices,
        },
        tex_coord: sys::SDL_FPoint {
            x: vd.tex_coords.max.x,
            y: vd.tex_coords.min.y,
        },
    };
    SDLPolygon {
        vers: vec![v1, v2, v3, v4],
        inds: vec![0, 1, 2, 2, 3, 0],
    }
}

struct TextUtils<'a, 'b, T> {
    glyph_brush: &'a mut GlyphBrush<SDLPolygon>,
    tex_creator: &'b TextureCreator<T>,
}

struct SDLPolyWithLazyTex {
    poly: Vec<SDLPolygon>,
    lazy_tex: LazyTexture,
}

struct LazySDLText {
    text: String,
    size: f32,
    color: Color,
    built: Vec<SDLPolygon>,
    lazy_tex: LazyTexture,
}

impl LazySDLText {
    fn new(text: String, size: f32, color: Color) -> Self {
        LazySDLText {
            text,
            size,
            color,
            built: vec![],
            lazy_tex: LazyTexture::new_empty(),
        }
    }

    fn build_text<'tex, T>(
        &mut self,
        utils: &mut TextUtils<'_, 'tex, T>,
        tex: &mut Texture<'tex>,
    ) -> Result<Vec<SDLPolygon>, Box<dyn Error>> {
        let glyph_brush = &mut *utils.glyph_brush;
        let tex_creator = &*utils.tex_creator;
        let section = Section::default().add_text(Text::new(&self.text).with_scale(self.size));
        glyph_brush.queue(section);
        let mut brush_action;

        loop {
            let dims = glyph_brush.texture_dimensions();

            brush_action = glyph_brush.process_queued(
                |rect, tex_data| {
                    self.lazy_tex.lazy_update(rect, tex_data);
                    let sdl_rect = sdl2::rect::Rect::new(
                        rect.min[0].try_into().unwrap(),
                        rect.min[1].try_into().unwrap(),
                        rect.width(),
                        rect.height(),
                    );
                    backup_update_texture(&sdl_rect, tex, &tex_data.to_vec(), &self.color)
                },
                |vertex_data| into_vertex(vertex_data),
            );
            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    // Enlarge texture + glyph_brush texture cache and retry.
                    self.lazy_tex.resize(suggested);
                    std::mem::replace(
                        tex,
                        tex_creator
                            .create_texture_static(
                                PixelFormatEnum::RGBA32,
                                suggested.0,
                                suggested.1,
                            )
                            .expect("Hey"),
                    );
                    tex.set_blend_mode(BlendMode::Blend);
                    glyph_brush.resize_texture(suggested.0, suggested.1);
                    println!(
                        "Resizing texture -> {}x{} to fit glyphs",
                        suggested.0, suggested.1
                    );
                }
            }
        }
        match brush_action.unwrap() {
            BrushAction::Draw(vertices) => {
                // Draw new vertices.
                self.built = vertices;
                println!("Text built");
            }
            BrushAction::ReDraw => {
                // Re-draw last frame's vertices unmodified.
                // println!("Text redrawn");
            }
        }
        Ok(self.built.clone())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video.window("playground", 1200, 1200).build()?;
    let mut canvas = window.into_canvas().build()?;

    let dejavu = FontArc::try_from_slice(include_bytes!("../Nouveau_IBM.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build();
    let dim = glyph_brush.texture_dimensions();
    println!("dim = {:?}", &dim);
    let tex_creator = canvas.texture_creator();
    let mut texture_ori =
        tex_creator.create_texture_static(PixelFormatEnum::RGBA32, dim.0, dim.1)?;
    texture_ori.set_blend_mode(BlendMode::Blend);

    let text_color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 0,
    };
    let mut utils = TextUtils {
        glyph_brush: &mut glyph_brush,
        tex_creator: &tex_creator,
    };

    let mut pre_sdl_text = LazySDLText::new("Hey".to_string(), 300.0, text_color.clone());
    let mut pre_sdl_text2 = LazySDLText::new("asd".to_string(), 0.0, text_color);

    let mut built = pre_sdl_text.build_text(&mut utils, &mut texture_ori)?;
    println!("{:?}", built);
    let mut built2 = pre_sdl_text2.build_text(&mut utils, &mut texture_ori);
    println!("{:?}", built2);

    let mut texture2 = {
        let lazy_texture2 = &pre_sdl_text2.lazy_tex;
        let mut texture2 = lazy_texture2.create_texture(&tex_creator, &pre_sdl_text2.color)?;
        println!("{:?}", lazy_texture2);
        texture2
    };

    let mut texture = {
        let lazy_texture = &pre_sdl_text.lazy_tex;
        let mut texture = lazy_texture.create_texture(&tex_creator, &pre_sdl_text.color)?;
        println!("{:?}", lazy_texture);
        texture
    };

    pre_sdl_text.text = "wow".to_string();
    built = pre_sdl_text.build_text(&mut utils, &mut texture_ori)?;
    {
        let lazy_texture = &pre_sdl_text.lazy_tex;
        lazy_texture.update(&mut texture, &pre_sdl_text.color);
        println!("{:?}", lazy_texture);
    }

    let mut event_pump = sdl.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        pre_sdl_text.text = "hey".to_string();
        built = pre_sdl_text.build_text(&mut utils, &mut texture_ori)?;
        {
            let lazy_texture = &pre_sdl_text.lazy_tex;
            lazy_texture.update(&mut texture, &pre_sdl_text.color);
            // println!("{:?}", lazy_texture);
        }
        // println!("{:?}", built);

        // println!("{:?}", pre_sdl_text.lazy_tex);

        canvas.set_draw_color(sdl2::pixels::Color::RGB(128, 128, 128));
        canvas.clear();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));

        let tex = Some(texture.raw());
        // let tex: Option<*mut sys::SDL_Texture> = None;
        for em in built.iter() {
            render_geometry(&mut canvas, tex, &em.vers, &em.inds)?;
        }

        let query = texture.query();
        // println!("w = {} h = {}", query.width, query.height);
        let rect_tex = sdl2::rect::Rect::new(
            query.width as i32,
            query.height as i32,
            query.width,
            query.height,
        );
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.copy(&texture, None, rect_tex)?;

        let query = texture_ori.query();
        // println!("w = {} h = {}", query.width, query.height);
        let rect_tex = sdl2::rect::Rect::new(0, query.height as i32, query.width, query.height);
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.copy(&texture_ori, None, rect_tex)?;

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        //break;
    }
    Ok(())
}

// *************************************************************************************************

fn render_geometry(
    canvas: &mut WindowCanvas,
    texture: Option<*mut sys::SDL_Texture>,
    vertices: &Vec<sys::SDL_Vertex>,
    indices: &Vec<i32>,
) -> Result<(), String> {
    if !vertices.is_empty() {
        let sdl_renderer = canvas.raw();
        let vers_num = vertices.len() as i32;
        let vers_ptr = (&vertices[0]) as *const sys::SDL_Vertex;
        let tex_ptr: *mut sys::SDL_Texture = match texture {
            None => ptr::null_mut(),
            Some(t) => t,
        };
        let ind_num = indices.len() as i32;
        let inds_ptr = match ind_num {
            0 => ptr::null(),
            _ => (&indices[0]),
        };
        let ret = unsafe {
            sys::SDL_RenderGeometry(sdl_renderer, tex_ptr, vers_ptr, vers_num, inds_ptr, ind_num)
        };
        if ret == -1 {
            return Err(format!(
                "Failed at SDL_RenderGeometry {}",
                sdl2::get_error()
            ));
        }
    }

    Ok(())
}

fn backup_update_texture(
    rect: &sdl2::rect::Rect,
    texture: &mut Texture,
    raw_data: &Vec<u8>,
    color: &Color,
) {
    println!("backup_update_texture()");
    let format_enum = texture.query().format;
    let bytes_per_pixel = format_enum.byte_size_per_pixel();
    let pitch = bytes_per_pixel * rect.width() as usize;
    let pixel_format = PixelFormat::try_from(format_enum)
        .expect("Failed to get a PixelFormat from PixelFormatEnum at update_texture()");
    let mut sdl_color = sdl2::pixels::Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0,
    };
    let mut new_data: Vec<u8> = vec![];
    for alpha in raw_data {
        sdl_color.a = *alpha;
        let native = sdl_color.to_u32(&pixel_format).to_ne_bytes();
        new_data.extend_from_slice(&native);
    }
    texture
        .update(*rect, &new_data, pitch)
        .expect(&format!("Failed to update_texture() {}", sdl2::get_error()));
}

/*
fn backup_text<T>(&mut self, utils: &mut TextUtils<'_, '_, T>) -> Result<Vec<SDLPolygon>, Box<dyn Error>> {
    let glyph_brush = &mut *utils.glyph_brush;
    let tex_creator = &*utils.tex_creator;
    let section = Section::default().add_text(Text::new(&self.text).with_scale(100.));
    glyph_brush.queue(section);
    let mut brush_action;

    loop {
        brush_action = glyph_brush.process_queued(
            |rect, tex_data| {update_texture(rect, tex_data); },
            |vertex_data| into_vertex(vertex_data),
        );
        match brush_action {
            Ok(_) => break,
            Err(BrushError::TextureTooSmall { suggested }) => {
                // Enlarge texture + glyph_brush texture cache and retry.
                std::mem::replace(&mut self.texture, tex_creator.create_texture_static(PixelFormatEnum::RGBA32,
                                                                                       suggested.0, suggested.1).expect("Hey"));
                self.texture.set_blend_mode(BlendMode::Blend);
                glyph_brush.resize_texture(suggested.0, suggested.1);
                println!("Resizing texture -> {}x{} to fit glyphs", suggested.0, suggested.1);
            }
        }
    }
    match brush_action.unwrap() {
        BrushAction::Draw(vertices) => {
            // Draw new vertices.
            self.built = vertices;
            println!("Text built");
        }
        BrushAction::ReDraw => {
            // Re-draw last frame's vertices unmodified.
            println!("Text redrawn");
        }
    }
    Ok(self.built.clone())
}
 */
