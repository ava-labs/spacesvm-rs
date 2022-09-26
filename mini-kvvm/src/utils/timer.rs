use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::{
    sync::{broadcast, mpsc, RwLock},
    time::{sleep, Instant},
};

#[derive(Clone)]
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/timer#Timer
pub struct Timer {
    /// Optional handler function that will fire when should_execute is true.
    handler: Option<fn()>,

    // Timeout broadcast channel is used to reset ticker threads.
    timeout_ch: broadcast::Sender<()>,

    /// New timer creation stops when true.
    finished: Arc<AtomicBool>,

    /// Notifies the timer to invoke the handler fn.
    should_execute: Arc<AtomicBool>,

    /// Duration for timer tick event.
    duration: Arc<RwLock<Option<Duration>>>,
}

impl Timer {
    pub fn new(handler: Option<fn()>) -> Self {
        let (timeout_ch, _): (broadcast::Sender<()>, broadcast::Receiver<()>) =
            broadcast::channel(1);
        Self {
            finished: Arc::new(AtomicBool::new(false)),
            should_execute: Arc::new(AtomicBool::new(false)),
            handler,
            timeout_ch,
            duration: Arc::new(RwLock::new(None)),
        }
    }

    /// Defines the duration until the handler function will be executed.
    pub async fn set_handler_duration(&self, duration: Duration) {
        let mut timer = self.duration.write().await;
        *timer = Some(duration);
        self.should_execute.store(true, Ordering::Relaxed);
        self.reset().await;
    }

    /// Cancel the currently scheduled event.
    pub async fn cancel(&self) {
        self.should_execute.store(false, Ordering::Relaxed);
        self.reset().await;
    }

    /// Stops execution of this timer.
    pub async fn stop(&self) {
        self.finished.store(true, Ordering::Relaxed);
        self.reset().await;
    }

    /// Manages a Timer lifecycle.
    pub async fn dispatch(&mut self) {
        let (ticker_tx, mut ticker_rx): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel(1);
        let cleared = Arc::new(AtomicBool::new(false));
        let reset = Arc::new(AtomicBool::new(false));

        // default duration is 0 so that we block until duration is set.
        let mut duration = Some(Duration::from_secs(0));
        while !self.finished.load(Ordering::Relaxed) {
            if cleared.load(Ordering::Relaxed) && self.should_execute.load(Ordering::Relaxed) {
                if let Some(handler) = &self.handler {
                    cleared.store(false, Ordering::Relaxed);
                    (handler)();
                }
            }

            // start a new ticker thread which sends a single tick signal.
            if reset.load(Ordering::Relaxed) {
                let ticker = ticker_tx.clone();
                tokio::spawn(async move {
                    let time = Instant::now();
                    if let Some(duration) = duration {
                        sleep(duration).await;
                    };
                    let _ = ticker.send(()).await;
                    log::debug!("Tick duration: {:?}", time.elapsed());
                });
            }

            reset.store(false, Ordering::Relaxed);
            cleared.store(false, Ordering::Relaxed);

            let mut timeout_ch = self.timeout_ch.subscribe();
            loop {
                // select will block until one of the channels is received
                tokio::select! {
                    Some(_) = ticker_rx.recv() => {
                        cleared.store(true, Ordering::Relaxed);
                        log::debug!("tick\n");
                        break;
                    },
                    resp = timeout_ch.recv() => match resp {
                        Ok(_) => {
                            // reset timer duration
                            if self.should_execute.load(Ordering::Relaxed) {
                                let guard = self.duration.read().await;
                                duration = guard.to_owned();
                                drop(guard);
                            }
                            reset.store(true, Ordering::Relaxed);
                            log::debug!("timeout\n");
                                break;
                            },
                            Err(e) => {
                                log::error!("timeout channel failed: {}", e.to_string());
                                break
                            },
                        }
                }
            }
        }
    }

    /// Calls the timeout channel which will result in a new timer event.
    pub async fn reset(&self) {
        let _ = self.timeout_ch.send(());
    }
}

#[tokio::test]
async fn timer_test() {
    fn echo() {
        println!("echo!!")
    }
    let timer = Timer::new(Some(echo));
    let mut timer_clone = timer.clone();
    tokio::spawn(async move {
        // echo will fire after 10ms
        timer.set_handler_duration(Duration::from_millis(10)).await;
        sleep(Duration::from_millis(15)).await;
        // unblock dispatch
        timer.stop().await;
    });

    timer_clone.dispatch().await;
}
