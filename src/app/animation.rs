use std::time::{Duration, Instant};

use adw::prelude::*;

use super::{FADE_DURATION_MS, FADE_FRAME_MS, LauncherHandles, reset_launcher};

pub(super) fn animate_show(handles: &LauncherHandles) {
    cancel_animation(handles);
    handles.sheet.set_opacity(0.0);
    handles.window.present();
    handles.search_entry.grab_focus();

    let sheet = handles.sheet.clone();
    let animation_source = handles.animation_source.clone();
    let started_at = Instant::now();

    let source = glib::timeout_add_local(Duration::from_millis(FADE_FRAME_MS), move || {
        let progress = (started_at.elapsed().as_millis() as f64 / FADE_DURATION_MS as f64).min(1.0);
        sheet.set_opacity(progress);

        if progress >= 1.0 {
            *animation_source.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    *handles.animation_source.borrow_mut() = Some(source);
}

pub(super) fn animate_hide(handles: &LauncherHandles) {
    cancel_animation(handles);

    let sheet = handles.sheet.clone();
    let fade_handles = handles.clone();
    let animation_source = handles.animation_source.clone();
    let start_opacity = sheet.opacity();
    let started_at = Instant::now();

    let source = glib::timeout_add_local(Duration::from_millis(FADE_FRAME_MS), move || {
        let progress = (started_at.elapsed().as_millis() as f64 / FADE_DURATION_MS as f64).min(1.0);
        let opacity = (start_opacity * (1.0 - progress)).max(0.0);
        sheet.set_opacity(opacity);

        if progress >= 1.0 {
            fade_handles.window.hide();
            reset_launcher(&fade_handles);
            fade_handles.sheet.set_opacity(1.0);
            *animation_source.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    *handles.animation_source.borrow_mut() = Some(source);
}

pub(super) fn cancel_animation(handles: &LauncherHandles) {
    if let Some(source) = handles.animation_source.borrow_mut().take() {
        source.remove();
    }
}
