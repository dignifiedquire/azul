//! Azul is a free, functional, immediate-mode GUI framework for rapid development
//! of desktop applications written in Rust, supported by the Mozilla WebRender
//! rendering engine, using a flexbox-based CSS / DOM model for layout and styling.
//!
//! # Concept
//!
//! Azul is largely based on the principle of immediate-mode GUI frameworks, which
//! is that the entire UI (in Azuls case the DOM) is reconstructed and re-rendered
//! on every frame (instead of having functions that mutate the UI state like
//! `button.setText()`). This method of constructing UIs has a performance overhead
//! over methods that retain the UI, therefore Azul only calls the [`Layout::layout()`]
//! function when its absolutely necessary - inside of a callback, you can return
//! whether it is necessary to redraw the screen or not (by returning
//! [`Redraw`] or [`DontRedraw`], respectively).
//!
//! In difference to other immediate-mode frameworks, Azul does not immediately
//! draw to the screen, but rather "draws" to a `Dom`. This has several advantages,
//! such as making it possible to layout code at runtime, [loading a `Dom` from
//! an XML file], recognizing state changes by diffing two frames, as well as being
//! able to reparent DOMs into almost any configuration to make components reusable
//! independent of the context they are in.
//!
//! # Development lifecycle
//!
//! A huge problem when working with GUI applications in Rust is managing the
//! compile time. Having to recompile your entire code when you just want to
//! shift an element a pixel to the right is not a good developer experience.
//! Azul has three main methods of combating compile time:
//!
//! - The [XML] system, which allows you to load DOMs at runtime [from a file]
//! - The [CSS] system, which allows you to [load and parse stylesheets]
//!
//! Due to Azuls stateless rendering architecutre, hot-reloading also preserves
//! the current application state. Once you are done layouting your applications
//! UI, you can [transpile the XML code to valid Rust source code] using [azulc],
//! the Azul-XML-to-Rust compiler.
//!
//! Please note that the compiler isn't perfect - the XML system is very limited,
//! and parsing XML has a certain performance overhead, since it's done on every frame.
//! That is fine for debug builds, but the XML system should not be used in release mode.
//!
//! When you are done with designing the callbacks of your widget, you may want to
//! package the widget up to autmatically react to certain events without having the
//! user of your widget write any code to hook up the callbacks - for this purpose,
//! Azul features a [two way data binding] system.
//!
//! # Custom drawing and embedding external applications
//!
//! Azul is mostly concerned with rendering text, images and rectangular boxes (divs).
//! Any other content can be drawn by drawing to an OpenGL texture (using a
//! [`GlCallback`]) and handing the texture as an "image" to Azul. This is also how
//! components like a video player or other OpenGL-based visualizations can exist
//! outside of the core library and be "injected" into the UI.
//!
//! You can draw to an OpenGL texture and hand it to Azul in order to display it
//! in the UI - the texture doesn't have to come from Azul itself, you can inject
//! it from an external application.
//!
//! # Limitations
//!
//! There are a few limitations that should be noted:
//!
//! - There are no scrollbars yet. Creating scrollable frames can be done by
//!   [creating an `IFrameCallback`].
//! - Similarly, there is no clipping of overflowing content yet - clipping only
//!   works for `IFrameCallback`s.
//! - There is no support for CSS animations of any kind yet
//! - Changing dynamic variables will trigger an entire UI relayout and restyling
//!
//! # Hello world
//!
//! ```no_run
//! use azul::prelude::*;
//!
//! struct MyDataModel { }
//!
//! impl Layout for MyDataModel {
//!     fn layout(&self, _: LayoutInfo<Self>) -> Dom<Self> {
//!         Dom::label("Hello World")
//!     }
//! }
//!
//! fn main() {
//!     let mut app = App::new(MyDataModel { }, AppConfig::default()).unwrap();
//!     let window = app.create_window(WindowCreateOptions::default(), css::native()).unwrap();
//!     app.run(window).unwrap();
//! }
//! ```
//!
//! Running this code should return a window similar to this:
//!
//! ![Opening a blank window](https://raw.githubusercontent.com/maps4print/azul/master/doc/azul_tutorial_empty_window.png)
//!
//! # Tutorials
//!
//! Explaining all concepts and examples is too much to be included in
//! this API reference. Please refer to the [wiki](https://github.com/maps4print/azul/wiki)
//! or use the links below to learn about how to use Azul.
//!
//! - [Getting Started](https://github.com/maps4print/azul/wiki/Getting-Started)
//! - [A simple counter](https://github.com/maps4print/azul/wiki/A-simple-counter)
//! - [Styling your app with CSS](https://github.com/maps4print/azul/wiki/Styling-your-application-with-CSS)
//! - [SVG drawing](https://github.com/maps4print/azul/wiki/SVG-drawing)
//! - [OpenGL drawing](https://github.com/maps4print/azul/wiki/OpenGL-drawing)
//! - [Timers, timers, tasks and async IO](https://github.com/maps4print/azul/wiki/Timers,-timers,-tasks-and-async-IO)
//! - [Two-way data binding](https://github.com/maps4print/azul/wiki/Two-way-data-binding)
//! - [Unit testing](https://github.com/maps4print/azul/wiki/Unit-testing)
//!
//! [`Layout::layout()`]: ../azul/traits/trait.Layout.html
//! [widgets]: ../azul/widgets/index.html
//! [loading a `Dom` from an XML file]: ../azul/dom/struct.Dom.html#method.from_file
//! [XML]: ../azul/xml/index.html
//! [`Redraw`]: ../azul/callbacks/constant.Redraw.html
//! [`DontRedraw`]: ../azul/callbacks/constant.DontRedraw.html
//! [`GlCallback`]: ../azul/callbacks/struct.GlCallback.html
//! [creating an `IFrameCallback`]: ../azul/dom/struct.Dom.html#method.iframe
//! [from a file]: ../azul/dom/struct.Dom.html#method.from_file
//! [CSS]: ../azul/css/index.html
//! [load and parse stylesheets]: ../azul/css/fn.from_str.html
//! [transpile the XML code to valid Rust source code]: https://github.com/maps4print/azul/wiki/XML-to-Rust-compilation
//! [azulc]: https://crates.io/crates/azulc
//! [two way data binding]: https://github.com/maps4print/azul/wiki/Two-way-data-binding

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/maps4print/azul/master/assets/images/azul_logo_full_min.svg.png",
    html_favicon_url = "https://raw.githubusercontent.com/maps4print/azul/master/assets/images/favicon.ico"
)]
#![allow(dead_code)]
#![deny(unused_must_use)]
#![deny(unreachable_patterns)]
#![deny(missing_copy_implementations)]
#![deny(clippy::all)]

#[macro_use]
extern crate lazy_static;
#[cfg(feature = "logging")]
#[macro_use]
extern crate log;

#[cfg(feature = "widgets")]
pub mod widgets {
    pub use azul_widgets::{button, errors, label, table_view, text_input};

    #[cfg(any(feature = "svg", feature = "svg_parsing"))]
    pub mod svg {

        use azul_core::ui_solver::ResolvedTextLayoutOptions;
        use azul_css::{LayoutPoint, StyleTextAlignmentHorz};
        pub use azul_widgets::svg::*;

        pub fn svg_text_layout_from_str(
            text: &str,
            font_bytes: &[u8],
            font_index: u32,
            mut text_layout_options: ResolvedTextLayoutOptions,
            horizontal_alignment: StyleTextAlignmentHorz,
        ) -> SvgTextLayout {
            use crate::text_layout;

            text_layout_options.font_size_px = SVG_FAKE_FONT_SIZE;
            let words = text_layout::split_text_into_words(text);
            let scaled_words = text_layout::words_to_scaled_words(
                &words,
                font_bytes,
                font_index,
                SVG_FAKE_FONT_SIZE,
            );
            let word_positions =
                text_layout::position_words(&words, &scaled_words, &text_layout_options);
            let mut inline_text_layout =
                text_layout::word_positions_to_inline_text_layout(&word_positions, &scaled_words);
            inline_text_layout.align_children_horizontal(horizontal_alignment);
            let layouted_glyphs = text_layout::get_layouted_glyphs(
                &word_positions,
                &scaled_words,
                &inline_text_layout,
                LayoutPoint::zero(),
            );

            SvgTextLayout {
                words,
                scaled_words,
                word_positions,
                layouted_glyphs,
                inline_text_layout,
            }
        }
    }
}

// Crate-internal macros
#[macro_use]
mod macros;

/// Manages application state (`App` / `AppState` / `AppResources`), wrapping resources and app state
pub mod app;
/// Type definitions for various types of callbacks, as well as focus and scroll handling
pub use azul_core::callbacks;
/// Async IO helpers / (`Task` / `Timer` / `Thread`)
pub use azul_core::r#async;
/// CSS type definitions / CSS parsing functions
#[cfg(any(feature = "css_parser", feature = "native_style"))]
pub mod css;
/// Bindings to the native file-chooser, color picker, etc. dialogs
pub mod dialogs;
/// DOM diffing
pub use azul_core::diff;
/// DOM / HTML node handling
pub use azul_core::dom;
/// OpenGL helper functions, necessary to create OpenGL textures, manage contexts, etc.
pub use azul_core::gl;
/// Handles text layout (modularized, can be used as a standalone module)
pub mod text_layout;
/// Main `Layout` trait definition + convenience traits for `Arc<Mutex<T>>`
pub mod traits;
/// Window state handling and window-related information
pub mod window;
/// XML-based DOM serialization and XML-to-Rust compiler implementation
pub mod xml;

/// Slab-allocated DOM nodes
use azul_core::id_tree;
/// UI Description & display list handling (webrender)
use azul_core::ui_description;
/// Manages the hover / focus tags for the DOM items
use azul_core::ui_state;
/// ImageId / FontId handling and caching
mod app_resources;
/// The compositor takes all textures (user-defined + the UI texture(s)) and draws them on
/// top of each other
mod compositor;
/// Converts the UI description (the styled HTML nodes)
/// to an actual display list (+ layout)
mod display_list;
/// Default logger, can be turned off with `feature = "logging"`
#[cfg(feature = "logging")]
mod logging;
/// HarfBuzz text shaping utilities
mod text_shaping;
/// Flexbox-based UI solver
mod ui_solver;
/// Window state handling and diffing
mod window_state;
/// Translation between data types (so that Azuls API can be independent of the actual "backend" type)
mod wr_translate;

pub use azul_core::{FastHashMap, FastHashSet};

/// Font & image resource handling, lookup and caching
pub mod resources {
    // re-export everything *except* the AppResources (which are exported under the "app" module)
    pub use crate::app_resources::{
        font_source_get_bytes, image_source_get_bytes, CssFontId, CssImageId, FontId,
        FontReloadError, FontSource, ImageId, ImageReloadError, ImageSource, LoadedFont, RawImage,
        RawImageFormat, TextCache, TextId,
    };
}

/// Quick exports of common types
pub mod prelude {
    pub use crate::app::{App, AppConfig, AppResources, AppState};
    pub use crate::callbacks::*;
    pub use crate::dom::{
        Dom, DomHash, DomString, EventFilter, FocusEventFilter, HoverEventFilter, NodeData,
        NodeType, NotEventFilter, On, TabIndex, WindowEventFilter,
    };
    pub use crate::gl::{
        FragmentShaderCompileError, GLuint, GlApiVersion, GlShader, GlShaderCreateError,
        GlShaderLinkError, IndexBufferFormat, Texture, Uniform, UniformType, VertexAttribute,
        VertexAttributeType, VertexBuffer, VertexLayout, VertexLayoutDescription,
        VertexShaderCompileError,
    };
    pub use crate::r#async::{DropCheck, Task, TerminateTimer, Timer, TimerId};
    pub use crate::resources::{
        FontId, FontSource, ImageId, ImageSource, RawImageFormat, TextCache, TextId,
    };
    pub use crate::text_layout::GlyphInstance;
    pub use crate::traits::{Layout, Modify};
    pub use crate::window::{
        keymap, AcceleratorKey, AvailableMonitorsIter, DebugState, KeyboardState, LogicalPosition,
        LogicalSize, MouseState, PhysicalPosition, PhysicalSize, RendererType, ScanCode,
        VirtualKeyCode, Window, WindowCreateOptions, WindowMonitorTarget, WindowState,
    };
    pub use crate::xml::{DomXml, XmlComponent, XmlComponentMap};
    pub use azul_core::{
        callbacks::StackCheckedPointer,
        ui_solver::{ResolvedTextLayoutOptions, TextLayoutOptions},
    };
    pub use azul_css::*;

    pub use crate::css;
    #[cfg(feature = "logging")]
    pub use log::LevelFilter;
}

/// Re-exports of errors
pub mod errors {
    pub use crate::{
        app::RuntimeError,
        app_resources::{FontReloadError, ImageReloadError},
        window::WindowCreateError,
    };
    // TODO: re-export the sub-types of ClipboardError!
    pub use clipboard2::ClipboardError;

    #[derive(Debug)]
    pub enum Error {
        Resource(ResourceReloadError),
        Clipboard(ClipboardError),
        WindowCreate(WindowCreateError),
    }

    impl_from!(ResourceReloadError, Error::Resource);
    impl_from!(ClipboardError, Error::Clipboard);
    impl_from!(WindowCreateError, Error::WindowCreate);

    #[derive(Debug)]
    pub enum ResourceReloadError {
        Image(ImageReloadError),
        Font(FontReloadError),
    }

    impl_from!(ImageReloadError, ResourceReloadError::Image);
    impl_from!(FontReloadError, ResourceReloadError::Font);

    impl_display!(ResourceReloadError, {
        Image(e) => format!("Failed to load image: {}", e),
        Font(e) => format!("Failed to load font: {}", e),
    });

    impl_display!(Error, {
        Resource(e) => format!("{}", e),
        Clipboard(e) => format!("Clipboard error: {}", e),
        WindowCreate(e) => format!("Window creation error: {}", e),
    });
}
