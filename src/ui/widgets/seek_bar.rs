use std::{cell::Cell, rc::Rc};

use gtk::prelude::*;

#[derive(Clone)]
pub struct SeekBar {
    scale: gtk::Scale,
    /// True when we're programmatically updating the value (not user action).
    updating: Rc<Cell<bool>>,
    /// True when the user is actively dragging the slider.
    dragging: Rc<Cell<bool>>,
}

impl SeekBar {
    pub fn new() -> Self {
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 0.1);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        scale.set_sensitive(false);
        scale.add_css_class("seek-bar");

        let dragging = Rc::new(Cell::new(false));

        // Detect drag start/end via GestureClick on the scale
        let drag_flag = dragging.clone();
        let press = gtk::GestureClick::new();
        press.connect_pressed(move |_, _, _, _| {
            drag_flag.set(true);
        });
        scale.add_controller(press);

        let drag_flag = dragging.clone();
        let release = gtk::GestureClick::new();
        release.connect_released(move |_, _, _, _| {
            drag_flag.set(false);
        });
        // Use released on a separate gesture with propagation phase Target
        release.set_propagation_phase(gtk::PropagationPhase::Bubble);
        scale.add_controller(release);

        Self {
            scale,
            updating: Rc::new(Cell::new(false)),
            dragging,
        }
    }

    pub fn widget(&self) -> &gtk::Scale {
        &self.scale
    }

    /// Returns true if the user is currently dragging the seek bar.
    pub fn is_dragging(&self) -> bool {
        self.dragging.get()
    }

    /// Called by the app to sync the UI with the backend position.
    /// Skips update when the user is dragging to avoid fighting.
    pub fn set_position(&self, position_seconds: f64, duration_seconds: f64) {
        let upper = duration_seconds.max(1.0);

        self.updating.set(true);
        self.scale.set_range(0.0, upper);
        // Don't update value while user is dragging — prevents stutter
        if !self.dragging.get() {
            self.scale.set_value(position_seconds.clamp(0.0, upper));
        }
        self.scale.set_sensitive(duration_seconds > 0.0);
        self.updating.set(false);
    }

    /// Bind a callback that fires when the USER changes the seek position.
    /// Programmatic updates (from set_position) are ignored.
    pub fn bind_seek<F>(&self, on_seek: F)
    where
        F: Fn(f64) + 'static,
    {
        let updating = self.updating.clone();
        self.scale.connect_value_changed(move |scale| {
            // Only seek when it's a user action, not our programmatic update
            if !updating.get() {
                on_seek(scale.value());
            }
        });
    }
}
