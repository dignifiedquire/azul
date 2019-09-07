use std::{collections::BTreeMap, fmt};

use crate::{
    app_resources::AppResources,
    r#async::{Task, Timer, TimerId},
    window::{FakeWindow, WindowId},
    FastHashMap,
};

/// Wrapper for your application data, stores the data, windows and resources, as
/// well as running timers and asynchronous tasks.
///
/// In order to be layout-able, your data model needs to satisfy the `Layout` trait,
/// which maps the state of your application to a DOM (how the application data should be laid out)
pub struct AppState<T> {
    /// Your data (the global struct which all callbacks will have access to)
    pub data: T,
    /// This field represents the state of the windows, public to the user. You can
    /// mess around with the state as you like, however, the actual window won't update
    /// until the next frame. This is done to "decouple" the frameworks internal
    /// state updating logic from the user code (and to make the API future-proof
    /// in case extra functions are introduced).
    ///
    /// Another reason this is needed is to (later) introduce testing for the window
    /// state - if the API would directly modify the window itself, these changes
    /// wouldn't be recorded anywhere, so there wouldn't be a way to unit-test certain APIs.
    ///
    /// The state of these `FakeWindow`s gets deleted and recreated on each frame, especially
    /// the app's style. This should force a user to design his code in a functional way,
    /// without relying on state-based conditions. Example:
    ///
    /// ```no_run,ignore
    /// let window_state = &mut app_state.windows[event.window];
    /// // Update the title
    /// window_state.state.title = "Hello";
    /// ```
    pub windows: BTreeMap<WindowId, FakeWindow<T>>,
    /// Fonts, images and cached text that is currently loaded inside the app (window-independent).
    ///
    /// Accessing this field is often required to load new fonts or images, so instead of
    /// requiring the `FontHashMap`, a lot of functions just require the whole `AppResources` field.
    pub resources: AppResources,
    /// Currently running timers (polling functions, run on the main thread)
    pub timers: FastHashMap<TimerId, Timer<T>>,
    /// Currently running tasks (asynchronous functions running each on a different thread)
    pub tasks: Vec<Task<T>>,
}

/// Same as the [AppState](./struct.AppState.html) but without the
/// `self.data` field - used for default callbacks, so that callbacks can
/// load and unload fonts or images + access the system clipboard
///
/// Default callbacks don't have access to the `AppState.data` field,
/// since they use a `StackCheckedPointer` instead.
pub struct AppStateNoData<'a, T> {
    /// See [`AppState.windows`](./struct.AppState.html#structfield.windows)
    pub windows: &'a BTreeMap<WindowId, FakeWindow<T>>,
    /// See [`AppState.resources`](./struct.AppState.html#structfield.resources)
    pub resources: &'a mut AppResources,
    /// Currently running timers (polling functions, run on the main thread)
    pub timers: &'a mut FastHashMap<TimerId, Timer<T>>,
    /// Currently running tasks (asynchronous functions running each on a different thread)
    pub tasks: &'a mut Vec<Task<T>>,
}

macro_rules! impl_task_api {() => {
    /// Insert a timer into the list of active timers.
    /// Replaces the existing timer if called with the same TimerId.
    pub fn add_timer(&mut self, id: TimerId, timer: Timer<T>) {
        self.timers.insert(id, timer);
    }

    pub fn has_timer(&self, timer_id: &TimerId) -> bool {
        self.get_timer(timer_id).is_some()
    }

    pub fn get_timer(&self, timer_id: &TimerId) -> Option<Timer<T>> {
        self.timers.get(&timer_id).cloned()
    }

    pub fn delete_timer(&mut self, timer_id: &TimerId) -> Option<Timer<T>> {
        self.timers.remove(timer_id)
    }

    /// Custom tasks can be used when the `AppState` isn't `Send`. For example
    /// `SvgCache` isn't thread-safe, since it has to interact with OpenGL, so
    /// it can't be sent to other threads safely.
    ///
    /// What you can do instead, is take a part of your application data, wrap
    /// that in an `Arc<Mutex<>>` and push a task that takes it onto the queue.
    /// This way you can modify a part of the application state on a different
    /// thread, while not requiring that everything is thread-safe.
    ///
    /// While you can't modify the `SvgCache` from a different thread, you can
    /// modify other things in the `AppState` and leave the SVG cache alone.
    pub fn add_task(&mut self, task: Task<T>) {
        self.tasks.push(task);
    }
}}

impl<T> AppState<T> {
    pub fn new(initial_data: T) -> Self {
        Self {
            data: initial_data,
            windows: BTreeMap::new(),
            resources: AppResources::default(),
            timers: FastHashMap::default(),
            tasks: Vec::new(),
        }
    }

    impl_task_api!();
}

impl<'a, T: 'a> AppStateNoData<'a, T> {
    impl_task_api!();
}

/// Error returned by the `.run()` function
///
/// If the `.run()` function would panic, that would need `T` to
/// implement `Debug`, which is not necessary if we just return an error.
#[derive(Debug)]
pub enum RuntimeError {
    /// Error indexing into internal BTreeMap - wrong window ID
    WindowIndexError,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RuntimeError::*;
        match self {
            WindowIndexError => write!(f, "Invalid window index"),
        }
    }
}
