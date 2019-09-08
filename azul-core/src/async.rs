use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::callbacks::{
    DontRedraw, TimerCallback, TimerCallbackInfo, TimerCallbackReturn, TimerCallbackType,
};

/// Should a timer terminate or not - used to remove active timers
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TerminateTimer {
    /// Remove the timer from the list of active timers
    Terminate,
    /// Do nothing and let the timers continue to run
    Continue,
}

static MAX_DAEMON_ID: AtomicUsize = AtomicUsize::new(0);

/// ID for uniquely identifying a timer
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerId {
    id: usize,
}

impl TimerId {
    /// Generates a new, unique `TimerId`.
    pub fn new() -> Self {
        TimerId {
            id: MAX_DAEMON_ID.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// A `Timer` is a function that is run on every frame.
///
/// There are often a lot of visual tasks such as animations or fetching the
/// next frame for a GIF or video, etc. - that need to run every frame or every X milliseconds,
/// but they aren't heavy enough to warrant creating a thread - otherwise the framework
/// would create too many threads, which leads to a lot of context switching and bad performance.
///
/// The callback of a `Timer` should be fast enough to run under 16ms,
/// otherwise running timers will block the main UI thread.
pub struct Timer<T> {
    /// Stores when the timer was created (usually acquired by `Instant::now()`)
    pub created: Instant,
    /// When the timer was last called (`None` only when the timer hasn't been called yet).
    pub last_run: Option<Instant>,
    /// If the timer shouldn't start instantly, but rather be delayed by a certain timeframe
    pub delay: Option<Duration>,
    /// How frequently the timer should run, i.e. set this to `Some(Duration::from_millis(16))`
    /// to run the timer every 16ms. If this value is set to `None`, (the default), the timer
    /// will execute the timer as-fast-as-possible (i.e. at a faster framerate
    /// than the framework itself) - which might be  performance intensive.
    pub interval: Option<Duration>,
    /// When to stop the timer (for example, you can stop the
    /// execution after 5s using `Some(Duration::from_secs(5))`).
    pub timeout: Option<Duration>,
    /// Callback to be called for this timer
    pub callback: TimerCallback<T>,
}

impl<T> Timer<T> {
    /// Create a new timer
    pub fn new(callback: TimerCallbackType<T>) -> Self {
        Timer {
            created: Instant::now(),
            last_run: None,
            delay: None,
            interval: None,
            timeout: None,
            callback: TimerCallback(callback),
        }
    }

    /// Delays the timer to not start immediately but rather
    /// start after a certain time frame has elapsed.
    #[inline]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Converts the timer into a timer, running the function only
    /// if the given `Duration` has elapsed since the last run
    #[inline]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Converts the timer into a countdown, by giving it a maximum duration
    /// (counted from the creation of the Timer, not the first use).
    #[inline]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Crate-internal: Invokes the timer if the timer and
    /// the `self.timeout` allow it to
    pub fn invoke<'a>(&mut self, info: TimerCallbackInfo<'a, T>) -> TimerCallbackReturn {
        let instant_now = Instant::now();
        let delay = self.delay.unwrap_or_else(|| Duration::from_millis(0));

        // Check if the timers timeout is reached
        if let Some(timeout) = self.timeout {
            if instant_now - self.created > timeout {
                return (DontRedraw, TerminateTimer::Terminate);
            }
        }

        if let Some(interval) = self.interval {
            let last_run = match self.last_run {
                Some(s) => s,
                None => self.created + delay,
            };
            if instant_now - last_run < interval {
                return (DontRedraw, TerminateTimer::Continue);
            }
        }

        let res = (self.callback.0)(info);

        self.last_run = Some(instant_now);

        res
    }
}

// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)] for Timer<T>

impl<T> fmt::Debug for Timer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Timer {{ \
             created: {:?}, \
             last_run: {:?}, \
             delay: {:?}, \
             interval: {:?}, \
             timeout: {:?}, \
             callback: {:?}, \
             }}",
            self.created, self.last_run, self.delay, self.interval, self.timeout, self.callback,
        )
    }
}

impl<T> Clone for Timer<T> {
    fn clone(&self) -> Self {
        Timer { ..*self }
    }
}

impl<T> Hash for Timer<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.created.hash(state);
        self.last_run.hash(state);
        self.delay.hash(state);
        self.interval.hash(state);
        self.timeout.hash(state);
        self.callback.hash(state);
    }
}

impl<T> PartialEq for Timer<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.created == rhs.created
            && self.last_run == rhs.last_run
            && self.delay == rhs.delay
            && self.interval == rhs.interval
            && self.timeout == rhs.timeout
            && self.callback == rhs.callback
    }
}

impl<T> Eq for Timer<T> {}

impl<T> Copy for Timer<T> {}

/// Simple struct that is used by Azul internally to determine when the thread has finished executing.
/// When this struct goes out of scope, Azul will call `.join()` on the thread (so in order to not
/// block the main thread, simply let it go out of scope naturally.
pub struct DropCheck(Arc<()>);

/// A `Task` is a seperate thread that is owned by the framework.
///
/// In difference to a `Thread`, you don't have to `await()` the result of a `Task`,
/// you can just hand the task to the framework (via `AppResources::add_task`) and
/// the framework will automatically update the UI when the task is finished.
/// This is useful to offload actions such as loading long files, etc. to a background thread.
///
/// Azul will join the thread automatically after it is finished (joining won't block the UI).
pub struct Task<T> {
    // Thread handle of the currently in-progress task
    join_handle: Option<JoinHandle<()>>,
    dropcheck: Weak<()>,
    /// Timer that will run directly after this task is completed.
    pub after_completion_timer: Option<Timer<T>>,
}

pub type TaskCallback<U> = fn(Arc<Mutex<U>>, DropCheck);

impl<T> Task<T> {
    /// Creates a new task from a callback and a set of input data - which has to be wrapped in an `Arc<Mutex<T>>>`.
    pub fn new<U: Send + 'static>(data: Arc<Mutex<U>>, callback: TaskCallback<U>) -> Self {
        let thread_check = Arc::new(());
        let thread_weak = Arc::downgrade(&thread_check);
        let thread_handle = thread::spawn(move || callback(data, DropCheck(thread_check)));

        Self {
            join_handle: Some(thread_handle),
            dropcheck: thread_weak,
            after_completion_timer: None,
        }
    }

    /// Stores a `Timer` that will run after the task has finished.
    ///
    /// Often necessary to "clean up" or copy data from the background task into the UI.
    #[inline]
    pub fn then(mut self, timer: Timer<T>) -> Self {
        self.after_completion_timer = Some(timer);
        self
    }

    /// Returns true if the task has been finished, false otherwise
    pub fn is_finished(&self) -> bool {
        self.dropcheck.upgrade().is_none()
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(thread_handle) = self.join_handle.take() {
            let _ = thread_handle.join().unwrap();
        }
    }
}

/// A `Thread` is a simple abstraction over `std::thread` that allows to offload a pure
/// function to a different thread (essentially emulating async / await for older compilers)
pub struct Thread<T> {
    data: Option<Arc<Mutex<T>>>,
    join_handle: Option<JoinHandle<()>>,
}

/// Error that can happen while calling `.await()`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AwaitError {
    /// Arc::into_inner() failed
    ArcUnlockError,
    /// The background thread panicked
    ThreadJoinError,
    /// Mutex::into_inner() failed
    MutexIntoInnerError,
}

impl<T> Thread<T> {
    /// Creates a new thread that spawns a certain (pure) function on a separate thread.
    /// This is a workaround until `await` is implemented. Note that invoking this function
    /// will create an OS-level thread.
    ///
    /// **Warning**: You *must* call `.await()`, otherwise the `Thread` will panic when it is dropped!
    ///
    /// # Example
    ///
    /// ```rust
    /// # use azul_core::r#async::Thread;
    /// #
    /// fn pure_function(input: usize) -> usize { input + 1 }
    ///
    /// let thread_1 = Thread::new(5, pure_function);
    /// let thread_2 = Thread::new(10, pure_function);
    /// let thread_3 = Thread::new(20, pure_function);
    ///
    /// // thread_1, thread_2 and thread_3 run in parallel here...
    ///
    /// let result_1 = thread_1.r#await();
    /// let result_2 = thread_2.r#await();
    /// let result_3 = thread_3.r#await();
    ///
    /// assert_eq!(result_1, Ok(6));
    /// assert_eq!(result_2, Ok(11));
    /// assert_eq!(result_3, Ok(21));
    /// ```
    pub fn new<U>(initial_data: U, callback: fn(U) -> T) -> Self
    where
        T: Send + 'static,
        U: Send + 'static,
    {
        use std::mem;

        // Reserve memory for T and zero it
        let data = Arc::new(Mutex::new(unsafe { mem::zeroed() }));
        let data_arc = data.clone();

        // For some reason, Rust doesn't realize that we're *moving* the data into the
        // child thread, which is why the 'static is unnecessary - that would only be necessary
        // if we'd reference the data from the main thread
        let thread_handle = thread::spawn(move || {
            *data_arc.lock().unwrap() = callback(initial_data);
        });

        Self {
            data: Some(data),
            join_handle: Some(thread_handle),
        }
    }

    /// Block until the internal thread has finished and return T
    pub fn r#await(mut self) -> Result<T, AwaitError> {
        // .await() can only be called once, so these .unwrap()s are safe
        let handle = self.join_handle.take().unwrap();
        let data = self.data.take().unwrap();

        handle.join().map_err(|_| AwaitError::ThreadJoinError)?;

        let data_arc = Arc::try_unwrap(data).map_err(|_| AwaitError::ArcUnlockError)?;
        let data = data_arc
            .into_inner()
            .map_err(|_| AwaitError::MutexIntoInnerError)?;

        Ok(data)
    }
}

impl<T> Drop for Thread<T> {
    fn drop(&mut self) {
        if self.join_handle.take().is_some() {
            panic!("Thread has not been await()-ed correctly!");
        }
    }
}
