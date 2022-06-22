//! A utility crate for retrying failable operations with various configuration options.
//!
//! # Example
//! ```rust
//! use attempt::Attempt;
//! # type Data = ();
//! # type Error = String;
//!
//! fn fetch_data_from_unreliable_api() -> Result<Data, Error> {
//!     // Fetch data from an API which randomly fails for no reason.
//!     // We've all dealt with one of these.
//!     # Ok(())
//! }
//!
//! fn main() {
//!     let res: Result<Data, Error> =
//!         Attempt::to(fetch_data_from_unreliable_api)
//!             .delay(std::time::Duration::from_secs(1))
//!             .max_tries(1000)
//!             .run();
//!
//!     // Be careful with this one.
//!     let res: Data =
//!         Attempt::infinitely(fetch_data_from_unreliable_api);
//!
//!     // "Sensible" default of 10 max tries with an increasing delay between
//!     //  each attempt starting at 500ms.
//!     let res: Result<Data, Error> = Attempt::to(fetch_data_from_unreliable_api).run();
//! }
//!
//! async fn fetch_data_from_unreliable_api_async() -> Result<Data, Error> {
//!     // Fetch data from an API which randomly fails for no reason, but do it async!
//!     # Ok(())
//! }
//!
//! async fn async_attempt_example() -> Result<Data, Error> {
//!     Attempt::to(fetch_data_from_unreliable_api_async)
//!         .delay(std::time::Duration::from_secs(1))
//!         .max_tries(1000)
//!         .run_async()
//!         .await
//! }
//! ```

use std::time::Duration;

/// This type provides an API for retrying failable functions.
///
/// See the documentation for this type's methods for detailed examples and the module
/// documentation for an overview example.
pub struct Attempt<F> {
    /// The function that will be ran and retried if necessary.
    func: F,

    /// The interval of time between each attempt.
    ///
    /// This duration will be multiplied by `delay_growth_magnitude` on each epoch.
    /// When `delay` is [`None`], the function will be called infinitly until an [`Ok`] is
    /// returned.
    delay: Option<Duration>,

    /// The magnitude of growth by which the `delay` will be multiplied by after each try.
    delay_growth_magnitude: f32,

    /// The maximum number of tries before the function returns an error.
    ///
    /// When `max_tries` is [`None`], the function will be called infinitly until an [`Ok`] is
    /// returned.
    max_tries: Option<usize>,
}

/// The default magnitude by which the delay between tries increases.
pub const DEFAULT_DELAY_GROWTH: f32 = 1.25;

/// The default cap on number of tries.
pub const DEFAULT_MAX_TRIES: usize = 10;

impl<F> Attempt<F> {
    /// Constructs a new [`Attempt`] which, when executed with either [`Attempt::run`] or
    /// [`Attempt::run_async`], will run the provided function `func` until it returns [`Ok`] or
    /// one of the limits are exceeded.
    ///
    /// The [`Attempt`] is constructed with the default configuration:
    /// * No time delay between attempts (thread will not sleep)
    /// * A default delay growth magnitude of 1.25 (25% increase each attempt)
    /// * A cap on maximum tries of 10
    /// These defaults are in place to hopefully prevent any accidental infinite loops.
    pub fn to(func: F) -> Attempt<F> {
        Attempt {
            func,
            delay: None,
            delay_growth_magnitude: DEFAULT_DELAY_GROWTH,
            max_tries: Some(DEFAULT_MAX_TRIES),
        }
    }

    /// Shortcut function to construct and run an [`Attempt`] that will retry a function infinitely
    /// until an [`Ok`] value is produced.
    ///
    /// Other than removing the limit on maximum attempts, this function uses the default
    /// configuration outlined in the documentation for [`Attempt::to`]. Using this function is
    /// honestly a terrible idea, especially for production code, but it may be useful for
    /// prototyping, idk.
    pub fn infinitely<T, E>(func: F) -> T
    where
        F: Fn() -> Result<T, E>,
    {
        Attempt::to(func)
            .no_max_tries()
            .run()
            .unwrap_or_else(|_| unreachable!())
    }

    /// Removes the limit on the maximum number of calls to the function that will be made before
    /// propagating an [`Err`].
    ///
    /// Please keep in mind that this setting can result in infinite loops and/or getting banned
    /// from a third-party API.
    pub fn no_max_tries(mut self) -> Self {
        self.max_tries = None;

        self
    }

    /// Sets the maximum bound for the maximum number of calls to the function that will be made
    /// before propagating an [`Err`].
    ///
    /// Must be greater than 0 (checked by assertion).
    pub fn max_tries(mut self, max_tries: usize) -> Self {
        assert!(max_tries > 0);
        self.max_tries = Some(max_tries);

        self
    }

    /// Removes the delay between each call to the function.
    pub fn no_delay(mut self) -> Self {
        self.delay = None;

        self
    }

    /// Sets the duration of the delay between each call to the function.
    ///
    /// For synchronous functions, the delay is implemented using [`std::thread::sleep`]. For
    /// async functions, the delay uses [`tokio::time::sleep`].
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);

        self
    }

    /// Sets the magnitude which the delay will be multiplied by after each epoch following the
    /// second try. For example, say our magnitude is 2.0, and our delay is 1 second. If the first
    /// call to the function fails, [`Attempt`] will wait 1 second before executing the function
    /// again. If that call also fails, [`Attempt`] will wait 2 seconds before executing the
    /// function a third time, and so on.
    pub fn delay_growth_magnitude(mut self, magnitude: f32) -> Self {
        self.delay_growth_magnitude = magnitude;

        self
    }

    pub fn run<T, E>(self) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
    {
        let execute_fn = self.func;
        let mut delay = self.delay;

        for iteration in 0.. {
            match execute_fn() {
                Ok(res) => return Ok(res),
                Err(err) => {
                    if let Some(max_tries) = self.max_tries {
                        if iteration + 1 >= max_tries {
                            return Err(err);
                        }
                    }

                    if let Some(epoch_delay) = delay {
                        std::thread::sleep(epoch_delay);

                        delay = Some(epoch_delay.mul_f32(self.delay_growth_magnitude));
                    }
                }
            }
        }

        unreachable!()
    }

    /// Runs the asynchronous function repeatedly until it returns [`Ok`] or the maximum attempt
    /// limit is reached, sleeping (using [`tokio::time::sleep`]) for the configured delay time if
    /// one is set.
    ///
    /// # Example
    /// ```rust
    /// # use attempt::Attempt;
    /// # #[tokio::main]
    /// # async fn main() {
    /// Attempt::to(|| async { if (true) { Ok(())  } else { Err(()) } })
    ///     .run_async()
    ///     .await
    ///     .expect("should retry until an Ok is produced");
    /// # }
    /// ```
    #[cfg(feature = "async")]
    pub async fn run_async<Fut, T, E>(self) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let execute_fn = self.func;
        let mut delay = self.delay;

        for iteration in 0.. {
            match execute_fn().await {
                Ok(res) => return Ok(res),
                Err(err) => {
                    if let Some(max_tries) = self.max_tries {
                        if iteration + 1 >= max_tries {
                            return Err(err);
                        }
                    }

                    if let Some(epoch_delay) = delay {
                        tokio::time::sleep(epoch_delay).await;

                        delay = Some(epoch_delay.mul_f32(self.delay_growth_magnitude));
                    }
                }
            }
        }

        unreachable!()
    }
}
