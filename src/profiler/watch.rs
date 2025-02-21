//! Leverages the usb HotplugEvent to create a stream of system USB devices
//!
//! See the watch cli for a usage example.
use super::nusb::NusbProfiler;
use super::{Device, DeviceEvent, SystemProfile};
use crate::error::Error;
use ::nusb::hotplug::HotplugEvent;
use ::nusb::watch_devices;
use chrono::Local;
use futures_lite::stream::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Builder for [`SystemProfileStream`]
#[derive(Default)]
pub struct SystemProfileStreamBuilder {
    spusb: Option<SystemProfile>,
    verbose: bool,
}

impl SystemProfileStreamBuilder {
    /// Create a new [`SystemProfileStreamBuilder`]
    pub fn new() -> Self {
        Self {
            spusb: None,
            verbose: true,
        }
    }

    /// Set the verbosity of the stream
    ///
    /// When set to true, the stream will include full device descriptors for verbose printing
    pub fn is_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the initial [`SystemProfile`] for the stream
    pub fn with_spusb(mut self, spusb: SystemProfile) -> Self {
        self.spusb = Some(spusb);
        self
    }

    /// Build the [`SystemProfileStream`]
    pub fn build(self) -> Result<SystemProfileStream, Error> {
        let spusb = if let Some(spusb) = self.spusb {
            Arc::new(Mutex::new(spusb))
        } else if self.verbose {
            Arc::new(Mutex::new(super::get_spusb_with_extra()?))
        } else {
            Arc::new(Mutex::new(super::get_spusb()?))
        };
        let mut new = SystemProfileStream::new(spusb)?;
        new.verbose = self.verbose;
        Ok(new)
    }
}

/// A stream that yields an updated [`SystemProfile`] when a USB device is connected or disconnected
pub struct SystemProfileStream {
    spusb: Arc<Mutex<SystemProfile>>,
    watch_stream: Pin<Box<dyn Stream<Item = HotplugEvent> + Send>>,
    verbose: bool,
}

impl SystemProfileStream {
    /// Create a new [`SystemProfileStream`] with a initial [`SystemProfile`]
    pub fn new(spusb: Arc<Mutex<SystemProfile>>) -> Result<Self, Error> {
        let watch_stream = Box::pin(watch_devices()?);
        Ok(Self {
            spusb,
            watch_stream,
            verbose: true,
        })
    }

    /// Get the [`SystemProfile`] from the stream
    pub fn get_profile(&self) -> Arc<Mutex<SystemProfile>> {
        Arc::clone(&self.spusb)
    }

    /// Re-profile the system USB devices
    ///
    /// Last events will be lost
    pub fn reprofile(&self) -> Arc<Mutex<SystemProfile>> {
        if self.verbose {
            Arc::new(Mutex::new(super::get_spusb_with_extra().unwrap()))
        } else {
            Arc::new(Mutex::new(super::get_spusb().unwrap()))
        }
    }
}

impl Stream for SystemProfileStream {
    type Item = Arc<Mutex<SystemProfile>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let extra = self.verbose;
        let this = self.get_mut();
        let mut profiler = NusbProfiler::new();

        match Pin::new(&mut this.watch_stream).poll_next(cx) {
            Poll::Ready(Some(event)) => {
                let mut spusb = this.spusb.lock().unwrap();

                match event {
                    HotplugEvent::Connected(device) => {
                        let mut cyme_device: Device =
                            profiler.build_spdevice(&device, extra).unwrap();
                        cyme_device.last_event = DeviceEvent::Connected(Local::now());
                        spusb.insert(cyme_device);
                    }
                    HotplugEvent::Disconnected(id) => {
                        if let Some(device) = spusb.get_id_mut(&id) {
                            device.last_event = DeviceEvent::Disconnected(Local::now());
                        }
                    }
                }
                Poll::Ready(Some(Arc::clone(&this.spusb)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
