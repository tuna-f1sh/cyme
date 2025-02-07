//! Leverages the usb HotplugEvent to create a stream of system USB devices
//!
//! See the watch cli for a usage example.
use super::nusb::NusbProfiler;
use super::{Device, SystemProfile, WatchEvent};
use crate::error::Error;
use ::nusb::hotplug::HotplugEvent;
use ::nusb::watch_devices;
use chrono::Local;
use futures_lite::stream::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// A stream that yields an updated [`SystemProfile`] when a USB device is connected or disconnected
pub struct SystemProfileStream {
    spusb: Arc<Mutex<SystemProfile>>,
    watch_stream: Pin<Box<dyn Stream<Item = HotplugEvent> + Send>>,
}

impl SystemProfileStream {
    /// Create a new [`SystemProfileStream`]
    ///
    /// Will create a new [`SystemProfile`] and watch for USB devices
    pub fn new() -> Result<Self, Error> {
        let spusb = Arc::new(Mutex::new(super::get_spusb_with_extra()?));
        let watch_stream = Box::pin(watch_devices()?);
        Ok(Self {
            spusb,
            watch_stream,
        })
    }

    /// Create a new [`SystemProfileStream`] with a initial [`SystemProfile`]
    pub fn new_with_spusb(spusb: Arc<Mutex<SystemProfile>>) -> Result<Self, Error> {
        let watch_stream = Box::pin(watch_devices()?);
        Ok(Self {
            spusb,
            watch_stream,
        })
    }

    /// Get the [`SystemProfile`] from the stream
    pub fn get_profile(&self) -> Arc<Mutex<SystemProfile>> {
        Arc::clone(&self.spusb)
    }
}

impl Stream for SystemProfileStream {
    type Item = Arc<Mutex<SystemProfile>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let mut profiler = NusbProfiler::new();

        match Pin::new(&mut this.watch_stream).poll_next(cx) {
            Poll::Ready(Some(event)) => {
                let mut spusb = this.spusb.lock().unwrap();

                match event {
                    HotplugEvent::Connected(device) => {
                        let mut cyme_device: Device =
                            profiler.build_spdevice(&device, true).unwrap();
                        cyme_device.last_event = Some(WatchEvent::Connected(Local::now()));

                        // is it existing? TODO this is a mess, need to take existing, put devices into new and replace since might have new descriptors
                        if let Some(existing) = spusb.get_node_mut(&cyme_device.port_path()) {
                            let devices = std::mem::take(&mut existing.devices);
                            cyme_device.devices = devices;
                            *existing = cyme_device;
                        // else we have to stick into tree at correct place
                        } else if cyme_device.is_trunk_device() {
                            let bus = spusb.get_bus_mut(cyme_device.location_id.bus).unwrap();
                            if let Some(bd) = bus.devices.as_mut() {
                                bd.push(cyme_device);
                            } else {
                                bus.devices = Some(vec![cyme_device]);
                            }
                        } else if let Ok(parent_path) = cyme_device.parent_path() {
                            if let Some(parent) = spusb.get_node_mut(&parent_path) {
                                if let Some(bd) = parent.devices.as_mut() {
                                    bd.push(cyme_device);
                                } else {
                                    parent.devices = Some(vec![cyme_device]);
                                }
                            }
                        }
                    }
                    HotplugEvent::Disconnected(id) => {
                        if let Some(device) = spusb.get_id_mut(&id) {
                            device.last_event = Some(WatchEvent::Disconnected(Local::now()));
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
