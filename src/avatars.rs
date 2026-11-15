//! GitHub avatars, fetched once per login on a worker thread and kept in memory.
//! Until an image arrives (or when avatars are off) views draw initials instead.
use gpui::{Image, ImageFormat};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

enum Slot {
    Loading,
    Ready(Arc<Image>),
    Missing,
}

#[derive(Clone, Default)]
pub struct Avatars {
    slots: Arc<Mutex<HashMap<String, Slot>>>,
    /// Set when an image lands, so the workspace knows to repaint.
    pub dirty: Arc<AtomicBool>,
}

fn format(bytes: &[u8]) -> Option<ImageFormat> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, ..] => Some(ImageFormat::Jpeg),
        [b'G', b'I', b'F', ..] => Some(ImageFormat::Gif),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some(ImageFormat::Webp),
        _ => None,
    }
}

impl Avatars {
    /// The avatar for `login`, starting a download the first time it is asked for.
    pub fn get(&self, login: &str) -> Option<Arc<Image>> {
        if login.is_empty() {
            return None;
        }
        let mut slots = self.slots.lock().ok()?;
        match slots.get(login) {
            Some(Slot::Ready(img)) => return Some(img.clone()),
            Some(_) => return None,
            None => {}
        }
        slots.insert(login.to_string(), Slot::Loading);
        drop(slots);
        let (slots, dirty, login) = (self.slots.clone(), self.dirty.clone(), login.to_string());
        std::thread::spawn(move || {
            let url = format!("https://avatars.githubusercontent.com/{login}?s=80");
            let image = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("gitflow-desktop")
                .build()
                .and_then(|c| c.get(url).send())
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.bytes())
                .ok()
                .and_then(|b| format(&b).map(|f| Arc::new(Image::from_bytes(f, b.to_vec()))));
            if let Ok(mut s) = slots.lock() {
                s.insert(login, image.map(Slot::Ready).unwrap_or(Slot::Missing));
            }
            dirty.store(true, Ordering::SeqCst);
        });
        None
    }
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sniffs_image_formats() {
        use super::format;
        use gpui::ImageFormat;
        assert_eq!(format(&[0x89, b'P', b'N', b'G', 0]), Some(ImageFormat::Png));
        assert_eq!(format(&[0xFF, 0xD8, 0xFF]), Some(ImageFormat::Jpeg));
        assert_eq!(format(b"<html>"), None);
    }
}
